// Guards the lenient date-time deserializer applied by scripts/fix-datetime-fields.py: DoseSpot
// returns many date-times without an offset ("2026-03-12T23:38:38.207"), which chrono's default
// `DateTime<FixedOffset>` deserializer rejects, failing the whole response. If a regeneration
// drops the `deserialize_with` attribute, these tests fail.
#![cfg(feature = "jumpstart-epcs")]

use autogen_dosespot::jumpstart_epcs::{apis, models};
use chrono::{DateTime, FixedOffset};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn rfc3339(s: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(s).unwrap()
}

#[tokio::test]
async fn generated_get_accepts_offset_less_dates() {
    let server = MockServer::start().await;
    let body = r#"{
        "Items": [{
            "PatientAllergyId": 1,
            "DisplayName": "Penicillin",
            "OnsetDate": "2026-03-12T23:38:38.207"
        }]
    }"#;
    Mock::given(method("GET"))
        .and(path("/api/patients/1/allergies"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
        .mount(&server)
        .await;

    let config = apis::configuration::Configuration {
        base_path: server.uri(),
        ..Default::default()
    };
    let response = apis::allergies_api::allergies_get_patient_allergies_v2(
        &config,
        autogen_dosespot::ids::PatientId(1),
    )
    .await
    .expect("offset-less OnsetDate must parse");

    let items = response.items.expect("items");
    assert_eq!(
        items[0].onset_date,
        Some(rfc3339("2026-03-12T23:38:38.207Z"))
    );
}

#[test]
fn optional_field_keeps_offsets_and_handles_null_and_missing() {
    let parse = |json: &str| {
        serde_json::from_str::<models::PatientAllergy>(json)
            .unwrap()
            .onset_date
    };

    assert_eq!(
        parse(r#"{"OnsetDate": "2026-03-12T23:38:38-05:00"}"#),
        Some(rfc3339("2026-03-12T23:38:38-05:00"))
    );
    assert_eq!(
        parse(r#"{"OnsetDate": "2026-03-12T23:38:38Z"}"#),
        Some(rfc3339("2026-03-12T23:38:38Z"))
    );
    assert_eq!(
        parse(r#"{"OnsetDate": "2026-03-12T23:38:38"}"#),
        Some(rfc3339("2026-03-12T23:38:38Z"))
    );
    assert_eq!(parse(r#"{"OnsetDate": null}"#), None);
    assert_eq!(parse(r#"{}"#), None);
}

#[test]
fn optional_field_rejects_garbage() {
    assert!(
        serde_json::from_str::<models::PatientAllergy>(r#"{"OnsetDate": "not a date"}"#).is_err()
    );
}

#[test]
fn required_field_accepts_offset_less_dates() {
    let mut patient = serde_json::to_value(models::Patient::default()).unwrap();
    patient["DateOfBirth"] = "1980-01-02T00:00:00".into();
    let patient: models::Patient = serde_json::from_value(patient).unwrap();
    assert_eq!(patient.date_of_birth, rfc3339("1980-01-02T00:00:00Z"));
}
