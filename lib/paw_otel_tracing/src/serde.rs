use serde::{Deserialize, Deserializer};
use std::str::FromStr;

pub fn deserialize_tracing_level<'de, D>(deserializer: D) -> Result<tracing::Level, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    tracing::Level::from_str(&s).map_err(serde::de::Error::custom)
}
