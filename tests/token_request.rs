// Guards the hand-written token helper in src/token.rs: DoseSpot's v2 password grant has
// non-obvious field values (notably `password` = clinic key, not a user password) that are
// documented nowhere public. If a refactor ever changes the form fields, the endpoint path,
// or the Subscription-Key header, this test fails.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

/// Serves exactly one request, returning the full request head + body.
fn capture_one_request(listener: TcpListener) -> JoinHandle<String> {
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 8192];
        let n = stream.read(&mut buf).unwrap();
        let request = String::from_utf8_lossy(&buf[..n]).to_string();
        let body = r#"{"access_token":"tok-123","expires_in":3600,"token_type":"Bearer"}"#;
        let resp = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(resp.as_bytes()).unwrap();
        request
    })
}

#[tokio::test]
async fn token_request_sends_dosespot_password_grant() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = capture_one_request(listener);

    let token = autogen_dosespot::token::request_token(
        &format!("http://{addr}/"), // trailing slash must be tolerated
        "sub-key",
        "clinic-id",
        "clinic-key",
        "clinician-9",
    )
    .await
    .unwrap();

    assert_eq!(token.access_token, "tok-123");
    assert_eq!(token.expires_in, Some(3600));
    assert_eq!(token.token_type.as_deref(), Some("Bearer"));

    let request = server.join().unwrap();
    assert!(request.starts_with("POST /webapi/v2/connect/token HTTP/1.1"), "{request}");
    assert!(request.to_lowercase().contains("subscription-key: sub-key"), "{request}");
    let body = request.split("\r\n\r\n").nth(1).unwrap_or_default();
    assert_eq!(
        body,
        "grant_type=password&client_id=clinic-id&client_secret=clinic-key&username=clinician-9&password=clinic-key&scope=api"
    );
}
