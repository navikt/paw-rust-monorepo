use chrono::NaiveDate;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serializer};
use serde_with::{DeserializeAs, SerializeAs};

/// `serde_with`-adapter for Avro `date`: antall dager siden 1970-01-01 som `i32`.
pub struct AvroDate;

const EPOCH: NaiveDate = NaiveDate::from_ymd_opt(1970, 1, 1).expect("gyldig dato");

impl SerializeAs<NaiveDate> for AvroDate {
    fn serialize_as<S: Serializer>(source: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error> {
        let days = i32::try_from(source.signed_duration_since(EPOCH).num_days())
            .map_err(serde::ser::Error::custom)?;
        serializer.serialize_i32(days)
    }
}

impl<'de> DeserializeAs<'de, NaiveDate> for AvroDate {
    fn deserialize_as<D: Deserializer<'de>>(deserializer: D) -> Result<NaiveDate, D::Error> {
        let days = i32::deserialize(deserializer)?;
        EPOCH
            .checked_add_signed(chrono::Duration::days(i64::from(days)))
            .ok_or_else(|| D::Error::custom("Avro date utenfor gyldig område"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use serde_with::serde_as;

    #[serde_as]
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Wrapper(#[serde_as(as = "AvroDate")] NaiveDate);

    #[test]
    fn dager_siden_epoch() {
        for (dato, dager) in [
            (NaiveDate::from_ymd_opt(1970, 1, 1).unwrap(), 0),
            (NaiveDate::from_ymd_opt(1970, 1, 2).unwrap(), 1),
            (NaiveDate::from_ymd_opt(1969, 12, 31).unwrap(), -1),
            (NaiveDate::from_ymd_opt(2026, 10, 28).unwrap(), 20_754),
        ] {
            let value = apache_avro::to_value(Wrapper(dato)).unwrap();
            assert_eq!(value, apache_avro::types::Value::Int(dager));
            let tilbake: Wrapper =
                apache_avro::from_value(&apache_avro::types::Value::Date(dager)).unwrap();
            assert_eq!(tilbake.0, dato);
        }
    }
}
