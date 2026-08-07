// Guards the datetime query-param rewrite in scripts/fix-datetime-query-params.py: the
// upstream rust generator serializes chrono DateTime query params via Display, which is
// not RFC 3339 and is rejected by the DoseSpot API. If a regeneration ever reintroduces
// the raw `to_string()` serialization, this test fails.
#![cfg(feature = "readonly")]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;

/// Serves exactly one request, returning its request line ("GET /path?query HTTP/1.1").
fn capture_one_request(listener: TcpListener) -> JoinHandle<String> {
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buf = [0u8; 8192];
        let n = stream.read(&mut buf).unwrap();
        let head = String::from_utf8_lossy(&buf[..n]).to_string();
        let body = r#"{}"#;
        let resp = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(resp.as_bytes()).unwrap();
        head.lines().next().unwrap_or_default().to_string()
    })
}

#[tokio::test]
async fn datetime_query_params_are_rfc3339() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let server = capture_one_request(listener);

    let config = autogen_dosespot::readonly::apis::configuration::Configuration {
        base_path: format!("http://{addr}"),
        ..Default::default()
    };
    let start = chrono::DateTime::parse_from_rfc3339("2026-07-17T00:00:00Z").unwrap();
    // `PatientId` (not i32) also guards the strict-ID rewrite in scripts/fix-id-types.py.
    let _ = autogen_dosespot::readonly::apis::self_reported_medications_api::self_reported_medications_get_patient_self_report_medications_v2(
        &config,
        autogen_dosespot::ids::PatientId(1),
        Some(start),
        None,
        None,
    )
    .await;

    let request_line = server.join().unwrap();
    assert!(
        request_line.contains("startDate=2026-07-17T00%3A00%3A00Z"),
        "datetime query param was not RFC 3339: {request_line}"
    );
    assert!(
        !request_line.contains("+00%3A00"),
        "datetime query param used chrono Display serialization: {request_line}"
    );
}
