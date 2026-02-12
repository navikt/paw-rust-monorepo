use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serializer, de};

pub fn serialize<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let timestamp = dt.timestamp() as f64 + (dt.timestamp_subsec_nanos() as f64 / 1_000_000_000.0);
    serializer.serialize_f64(timestamp)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Timestamp {
        Millis(i64),
        Seconds(f64),
    }

    match Timestamp::deserialize(deserializer)? {
        Timestamp::Millis(millis) => {
            let secs = millis / 1000;
            let nanos = ((millis % 1000) * 1_000_000) as u32;
            DateTime::from_timestamp(secs, nanos)
                .ok_or_else(|| de::Error::custom("Invalid timestamp"))
        }
        Timestamp::Seconds(secs) => {
            let sec = secs.trunc() as i64;
            let nanos = ((secs.fract() * 1_000_000_000.0).round() as u32).min(999_999_999);
            DateTime::from_timestamp(sec, nanos)
                .ok_or_else(|| de::Error::custom("Invalid timestamp"))
        }
    }
}
