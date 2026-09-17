#![cfg(feature = "full")]

//! Guards that `DoseSpotClient` and `request_token` both route every HTTP request through the
//! same attached `reqwest_middleware` chain, in call order: token request first, then a
//! generated API call.

use std::sync::{Arc, Mutex};

use autogen_dosespot::{DoseSpotClient, token::request_token};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[derive(Clone, Default)]
struct Recorder {
    seen: Arc<Mutex<Vec<(String, String)>>>,
}

#[async_trait::async_trait]
impl reqwest_middleware::Middleware for Recorder {
    async fn handle(
        &self,
        req: reqwest::Request,
        extensions: &mut http::Extensions,
        next: reqwest_middleware::Next<'_>,
    ) -> reqwest_middleware::Result<reqwest::Response> {
        self.seen.lock().unwrap().push((
            req.method().to_string(),
            req.url().host_str().unwrap_or_default().to_string(),
        ));
        next.run(req, extensions).await
    }
}

#[tokio::test]
async fn middleware_chain_sees_token_and_api_requests() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/webapi/v2/connect/token"))
        .and(header("Subscription-Key", "sub-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "tok-123",
            "expires_in": 3600,
            "token_type": "Bearer",
        })))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/webapi/v2/api/general/check"))
        .and(header("Subscription-Key", "sub-key"))
        .and(header("Authorization", "Bearer tok-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .expect(1)
        .mount(&server)
        .await;

    let recorder = Recorder::default();

    let chain = reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
        .with(recorder.clone())
        .build();
    let token = request_token(
        &chain,
        &server.uri(),
        "sub-key",
        "clinic-id",
        "clinic-key",
        "clinician-9",
    )
    .await
    .unwrap();
    assert_eq!(token.access_token, "tok-123");

    let client = DoseSpotClient::builder("sub-key", &token.access_token)
        .with(recorder.clone())
        .build()
        .unwrap();
    let mut config = client.full();
    config.base_path = format!("{}/webapi/v2", server.uri());

    let result =
        autogen_dosespot::full::apis::health_check_api::health_check_check_health_v2(&config)
            .await;
    assert!(result.is_ok(), "{result:?}");

    let seen = recorder.seen.lock().unwrap().clone();
    assert_eq!(
        seen,
        vec![
            ("POST".to_string(), "127.0.0.1".to_string()),
            ("GET".to_string(), "127.0.0.1".to_string()),
        ]
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}
