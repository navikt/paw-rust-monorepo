//! Jackson (`JavaTimeModule`) skriver `Instant` som desimale sekunder, for eksempel
//! `1770897349.305000000`. Heltall leses som sekunder (Jacksons standard), og
//! ISO-8601-tekst godtas også.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serializer, de};

pub fn serialize<S: Serializer>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error> {
    let secs = dt.timestamp() as f64 + f64::from(dt.timestamp_subsec_nanos()) / 1_000_000_000.0;
    serializer.serialize_f64(secs)
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<DateTime<Utc>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Instant {
        Seconds(i64),
        Fractional(f64),
        Text(String),
    }

    match Instant::deserialize(deserializer)? {
        Instant::Seconds(secs) => DateTime::from_timestamp(secs, 0),
        Instant::Fractional(secs) => {
            let whole = secs.floor();
            let nanos = (((secs - whole) * 1_000_000_000.0).round() as u32).min(999_999_999);
            DateTime::from_timestamp(whole as i64, nanos)
        }
        Instant::Text(text) => DateTime::parse_from_rfc3339(&text)
            .map(|dt| dt.with_timezone(&Utc))
            .ok(),
    }
    .ok_or_else(|| de::Error::custom("ugyldig Instant"))
}
