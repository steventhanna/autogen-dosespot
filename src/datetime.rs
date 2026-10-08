//! Lenient date-time deserialization for generated models.
//!
//! The specs declare every date as `format: date-time`, so the generator types them as
//! `chrono::DateTime<FixedOffset>`, whose default deserializer requires an offset. DoseSpot
//! returns many values without one (`"2026-03-12T23:38:38.207"`), which would fail the whole
//! response. `scripts/fix-datetime-fields.py` points every generated date-time field at these
//! functions. They accept everything chrono's default accepts, plus offset-less ISO 8601
//! date-times, which are read as UTC. Serialization is unchanged (RFC 3339 with an offset).

use chrono::{DateTime, FixedOffset, NaiveDateTime};
use serde::{Deserialize, Deserializer, de::Error};

/// Parses `s` as a date-time with an offset, or as an offset-less date-time read as UTC.
fn parse(s: &str) -> Option<DateTime<FixedOffset>> {
    s.parse::<DateTime<FixedOffset>>()
        .or_else(|_| {
            s.parse::<NaiveDateTime>()
                .map(|naive| naive.and_utc().fixed_offset())
        })
        .ok()
}

/// `deserialize_with` target for required `DateTime<FixedOffset>` fields.
pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<DateTime<FixedOffset>, D::Error> {
    let s = String::deserialize(deserializer)?;
    // The value is deliberately left out of the message: these fields can carry PHI (e.g. DOB).
    parse(&s).ok_or_else(|| {
        D::Error::custom("expected an ISO 8601 date-time, with or without an offset")
    })
}

/// `deserialize_with` target for `Option<DateTime<FixedOffset>>` fields; pair with
/// `#[serde(default)]` so a missing field stays `None`.
pub fn deserialize_option<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<DateTime<FixedOffset>>, D::Error> {
    #[derive(Deserialize)]
    struct Wrapper(#[serde(deserialize_with = "deserialize")] DateTime<FixedOffset>);

    Ok(Option::<Wrapper>::deserialize(deserializer)?.map(|Wrapper(dt)| dt))
}
