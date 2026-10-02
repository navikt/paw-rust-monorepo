//! Jackson (`JavaTimeModule`) skriver `Duration` som desimale sekunder, for eksempel
//! `86400.000000000`. Heltall leses som sekunder.

use chrono::TimeDelta;
use serde::{Deserialize, Deserializer, Serializer, de};

pub fn serialize<S: Serializer>(duration: &TimeDelta, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_f64(duration.as_seconds_f64())
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<TimeDelta, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Duration {
        Seconds(i64),
        Fractional(f64),
    }

    match Duration::deserialize(deserializer)? {
        Duration::Seconds(secs) => TimeDelta::try_seconds(secs),
        Duration::Fractional(secs) => {
            let whole = secs.floor();
            let nanos = (((secs - whole) * 1_000_000_000.0).round() as u32).min(999_999_999);
            TimeDelta::new(whole as i64, nanos)
        }
    }
    .ok_or_else(|| de::Error::custom("ugyldig Duration"))
}
