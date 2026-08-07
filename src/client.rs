//! Hand-written entry point that adds authentication on top of the generated per-plan
//! configurations. This is the only file (besides `lib.rs`) that is not generated.

use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

/// Error constructing a [`DoseSpotClient`].
///
/// The subscription key and access token become HTTP header values, so they must be visible
/// ASCII; building the underlying HTTP client can also fail (e.g. no TLS backend enabled).
#[derive(Debug)]
pub enum ClientBuildError {
    /// The subscription key or access token contains bytes not allowed in an HTTP header.
    InvalidHeaderValue(reqwest::header::InvalidHeaderValue),
    /// The underlying `reqwest::Client` could not be constructed.
    Reqwest(reqwest::Error),
}

impl std::fmt::Display for ClientBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHeaderValue(e) => write!(f, "credential is not a valid header value: {e}"),
            Self::Reqwest(e) => write!(f, "failed to build HTTP client: {e}"),
        }
    }
}

impl std::error::Error for ClientBuildError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidHeaderValue(e) => Some(e),
            Self::Reqwest(e) => Some(e),
        }
    }
}

/// Holds your DoseSpot credentials and hands out a ready-to-use [`Configuration`] for each plan.
///
/// The DoseSpot v2 API authenticates with a `Subscription-Key` header plus an OAuth2 bearer
/// token — neither is declared in the swagger specs, so the generated code does not attach
/// them itself. `DoseSpotClient` builds one `reqwest::Client` with both set as default headers
/// and produces a per-plan `Configuration` — with the correct base URL and that client already
/// wired in — via accessors like [`full`](DoseSpotClient::full) and
/// [`jumpstart`](DoseSpotClient::jumpstart).
///
/// Access tokens expire; construct a new `DoseSpotClient` when you refresh yours.
///
/// ```no_run
/// use autogen_dosespot::DoseSpotClient;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = DoseSpotClient::new("your-subscription-key", "your-access-token")?;
/// # #[cfg(feature = "full")]
/// let full = client.full(); // pass to autogen_dosespot::full::apis functions
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct DoseSpotClient {
    http: reqwest::Client,
}

impl DoseSpotClient {
    /// Create a client from a DoseSpot subscription key and an OAuth2 access token
    /// (obtained from `POST /webapi/v2/connect/token`; see DoseSpot's integration docs).
    pub fn new(
        subscription_key: impl AsRef<str>,
        access_token: impl AsRef<str>,
    ) -> Result<Self, ClientBuildError> {
        let mut headers = HeaderMap::new();
        let mut key = HeaderValue::from_str(subscription_key.as_ref())
            .map_err(ClientBuildError::InvalidHeaderValue)?;
        key.set_sensitive(true);
        headers.insert("Subscription-Key", key);
        let mut token = HeaderValue::from_str(&format!("Bearer {}", access_token.as_ref()))
            .map_err(ClientBuildError::InvalidHeaderValue)?;
        token.set_sensitive(true);
        headers.insert(AUTHORIZATION, token);

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(ClientBuildError::Reqwest)?;
        Ok(Self { http })
    }
}

/// Generates a `DoseSpotClient::<method>()` accessor returning the named plan's `Configuration`,
/// with the authenticated `reqwest::Client` (Subscription-Key + Bearer default headers) wired in
/// and a crate user-agent. The base URL is taken from the generated `Configuration::default()`
/// (i.e. the spec's server, `https://my.dosespot.com/webapi/v2`).
macro_rules! plan_config {
    ($method:ident, $feature:literal, $module:ident) => {
        #[cfg(feature = $feature)]
        impl DoseSpotClient {
            #[doc = concat!("Configuration for the DoseSpot `", stringify!($module), "` plan API.")]
            pub fn $method(&self) -> crate::$module::apis::configuration::Configuration {
                let mut config = crate::$module::apis::configuration::Configuration::default();
                config.client = self.http.clone();
                config.user_agent =
                    Some(format!("autogen-dosespot/{}", env!("CARGO_PKG_VERSION")));
                config
            }
        }
    };
}

plan_config!(full, "full", full);
plan_config!(full_epcs, "full-epcs", full_epcs);
plan_config!(hybrid, "hybrid", hybrid);
plan_config!(hybrid_epcs, "hybrid-epcs", hybrid_epcs);
plan_config!(jumpstart, "jumpstart", jumpstart);
plan_config!(jumpstart_epcs, "jumpstart-epcs", jumpstart_epcs);
plan_config!(readonly, "readonly", readonly);
