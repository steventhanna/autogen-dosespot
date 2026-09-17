//! Hand-written token acquisition for the DoseSpot v2 API.
//!
//! The token endpoint (`POST /webapi/v2/connect/token`) is not part of DoseSpot's swagger
//! specs, so it is not generated. It is an OAuth2 password grant with non-obvious field
//! values — most notably `password` is the **clinic key**, not a user password:
//!
//! | form field | value |
//! |---|---|
//! | `grant_type` | `password` |
//! | `client_id` | clinic ID |
//! | `client_secret` | clinic key |
//! | `username` | clinician user ID |
//! | `password` | clinic key (again) |
//! | `scope` | `api` |
//!
//! The `Subscription-Key` header is sent on the token request as well.
//!
//! This module is deliberately stateless: no caching, no refresh. Tokens expire
//! (see [`TokenResponse::expires_in`]), and when and for which clinician to mint one is
//! application policy. Feed the returned token to [`DoseSpotClient::new`](crate::DoseSpotClient::new).

use serde::Deserialize;

/// Successful response from the token endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    /// Bearer token to pass to [`DoseSpotClient::new`](crate::DoseSpotClient::new).
    pub access_token: String,
    /// Lifetime of the token in seconds, if the server reports one.
    #[serde(default)]
    pub expires_in: Option<i64>,
    /// Token type, if the server reports one (expected: `Bearer`).
    #[serde(default)]
    pub token_type: Option<String>,
}

/// Error from [`request_token`].
#[derive(Debug)]
pub enum TokenError {
    /// The HTTP request failed (transport, middleware, TLS, invalid header value, …).
    Http(reqwest_middleware::Error),
    /// The server answered with a non-success status.
    Api {
        status: reqwest::StatusCode,
        body: String,
    },
}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http(e) => write!(f, "token request failed: {e}"),
            Self::Api { status, body } => write!(f, "token endpoint returned {status}: {body}"),
        }
    }
}

impl std::error::Error for TokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Http(e) => Some(e),
            Self::Api { .. } => None,
        }
    }
}

/// Request an OAuth2 access token from the DoseSpot v2 token endpoint.
///
/// `http` is the same `reqwest_middleware::ClientWithMiddleware` (and middleware chain) you pass
/// to [`DoseSpotClient::builder`](crate::DoseSpotClient::builder), so the token call is traced
/// (or otherwise observed) exactly like every other request. A plain `reqwest::Client` converts
/// with `.into()` if you have no middleware to attach.
///
/// `base_url` is the host root — `https://my.dosespot.com` for production,
/// `https://my.staging.dosespot.com` for staging (a trailing slash is fine);
/// `/webapi/v2/connect/token` is appended. `clinician_id` becomes the grant's
/// `username` and determines which clinician the token acts as.
///
/// ```no_run
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let http = autogen_dosespot::reqwest_middleware::ClientBuilder::new(
///     autogen_dosespot::reqwest::Client::new(),
/// )
/// .build();
/// let token = autogen_dosespot::token::request_token(
///     &http,
///     "https://my.dosespot.com",
///     "your-subscription-key",
///     "your-clinic-id",
///     "your-clinic-key",
///     "your-clinician-id",
/// )
/// .await?;
/// let client = autogen_dosespot::DoseSpotClient::new("your-subscription-key", &token.access_token)?;
/// # Ok(())
/// # }
/// ```
pub async fn request_token(
    http: &reqwest_middleware::ClientWithMiddleware,
    base_url: &str,
    subscription_key: &str,
    clinic_id: &str,
    clinic_key: &str,
    clinician_id: &str,
) -> Result<TokenResponse, TokenError> {
    let url = format!(
        "{}/webapi/v2/connect/token",
        base_url.trim_end_matches('/')
    );
    let form = [
        ("grant_type", "password"),
        ("client_id", clinic_id),
        ("client_secret", clinic_key),
        ("username", clinician_id),
        // Not a user password: DoseSpot's v2 password grant expects the clinic key here.
        ("password", clinic_key),
        ("scope", "api"),
    ];

    let response = http
        .post(url)
        .header("Subscription-Key", subscription_key)
        .form(&form)
        .send()
        .await
        .map_err(TokenError::Http)?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(TokenError::Api { status, body });
    }
    response.json().await.map_err(|e| TokenError::Http(e.into()))
}
