use crate::leesah::{Endringstype, Opplysningstype};
use crate::serde::{AvroDatumDeserializer, AvroSerdeError};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{TimestampMilliSeconds, serde_as};
use std::fmt;

pub const PERSONHENDELSE_HEADER_SCHEMA: &str =
    include_str!("../../avro/avsc/leesah/PersonhendelseHeader.avsc");

#[serde_as]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonhendelseHeader {
    pub hendelse_id: String,
    pub personidenter: Vec<String>,
    pub master: String,
    #[serde_as(as = "TimestampMilliSeconds<i64>")]
    pub opprettet: DateTime<Utc>,
    pub opplysningstype: Opplysningstype,
    pub endringstype: Endringstype,
    pub tidligere_hendelse_id: Option<String>,
}

impl fmt::Debug for PersonhendelseHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PersonhendelseHode")
            .field("hendelse_id", &self.hendelse_id)
            .field("master", &self.master)
            .field("opprettet", &self.opprettet)
            .field("opplysningstype", &self.opplysningstype)
            .field("endringstype", &self.endringstype)
            .field("tidligere_hendelse_id", &self.tidligere_hendelse_id)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug)]
pub struct PersonhendelseHodeDeserializer(AvroDatumDeserializer);

impl PersonhendelseHodeDeserializer {
    pub fn new() -> Result<Self, AvroSerdeError> {
        AvroDatumDeserializer::new(PERSONHENDELSE_HEADER_SCHEMA).map(Self)
    }

    pub fn deserialize(&self, payload: &[u8]) -> Result<PersonhendelseHeader, AvroSerdeError> {
        self.0.deserialize_prefix(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::leesah::personhendelse::tests::{IDENT, fullt_utfylt, minimal};
    use crate::leesah::{PERSONHENDELSE_SCHEMA, Personhendelse, PersonhendelseSerializer};
    use apache_avro::Schema;
    use apache_avro::schema::RecordField;

    fn hode(h: &Personhendelse) -> PersonhendelseHeader {
        PersonhendelseHeader {
            hendelse_id: h.hendelse_id.clone(),
            personidenter: h.personidenter.clone(),
            master: h.master.clone(),
            opprettet: h.opprettet,
            opplysningstype: h.opplysningstype,
            endringstype: h.endringstype,
            tidligere_hendelse_id: h.tidligere_hendelse_id.clone(),
        }
    }

    fn felter(schema: &str) -> Vec<RecordField> {
        match Schema::parse_str(schema).unwrap() {
            Schema::Record(r) => r.fields,
            annet => panic!("forventet record, fikk {annet:?}"),
        }
    }

    #[test]
    fn hode_er_prefiks_av_fullt_skjema() {
        let hode = felter(PERSONHENDELSE_HEADER_SCHEMA);
        let full = felter(PERSONHENDELSE_SCHEMA);
        assert_eq!(hode.len(), 7);
        for (h, f) in hode.iter().zip(&full) {
            assert_eq!(h.name, f.name);
            assert_eq!(h.schema, f.schema, "felt {}", h.name);
        }
    }

    #[test]
    fn hode_lik_full_dekoding() {
        let ser = PersonhendelseSerializer::new().unwrap();
        let de = PersonhendelseHodeDeserializer::new().unwrap();
        for hendelse in [fullt_utfylt(), minimal()] {
            let bytes = ser.serialize(&hendelse).unwrap();
            assert_eq!(de.deserialize(&bytes).unwrap(), hode(&hendelse));
        }
    }

    #[test]
    fn hode_taaler_nye_felt_paa_slutten() {
        let mut bytes = PersonhendelseSerializer::new()
            .unwrap()
            .serialize(&minimal())
            .unwrap();
        bytes.extend_from_slice(&[2, 0x04, b'n', b'y']);
        let hode = PersonhendelseHodeDeserializer::new()
            .unwrap()
            .deserialize(&bytes)
            .unwrap();
        assert_eq!(hode.opplysningstype, Opplysningstype::DoedsfallV1);
    }

    #[test]
    fn debug_skjuler_personidenter() {
        let debug = format!("{:?}", hode(&fullt_utfylt()));
        assert!(debug.contains("BostedsadresseV1"), "{debug}");
        assert!(!debug.contains(IDENT), "{debug}");
        assert!(!debug.contains("1234567890123"), "{debug}");
    }
}
