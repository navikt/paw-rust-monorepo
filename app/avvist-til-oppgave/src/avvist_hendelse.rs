use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct AvvistHendelse {
    #[serde(rename = "hendelseId")]
    pub hendelse_id: Uuid,
    pub id: i64,
    pub identitetsnummer: String,
    pub metadata: Metadata,
    #[serde(rename = "hendelseType")]
    pub hendelse_type: String,
    pub opplysninger: Vec<String>,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct Metadata {
    pub tidspunkt: f64,
    #[serde(rename = "utfoertAv")]
    pub utfoert_av: UtfoertAv,
    pub kilde: String,
    pub aarsak: String,
}

#[derive(Debug, Deserialize, PartialEq, Clone)]
pub struct UtfoertAv {
    #[serde(rename = "type")]
    pub bruker_type: String,
    pub id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn hendelse_deserialization() {
        let hendelse: AvvistHendelse = serde_json::from_str(AVVIST_HENDELSE_JSON).unwrap();

        let expected_uuid = Uuid::parse_str("723d5d09-83c7-4f83-97fd-35f7c9c5c798").unwrap();
        assert_eq!(hendelse.hendelse_id, expected_uuid);

        assert_eq!(hendelse.id, 1);
        assert_eq!(hendelse.identitetsnummer, "12345678901");
        assert_eq!(hendelse.hendelse_type, "intern.v1.avvist");
        assert_eq!(hendelse.opplysninger, vec!["ER_UNDER_18_AAR"]);

        assert_eq!(hendelse.metadata.tidspunkt, 1630404930.000000000);
        assert_eq!(hendelse.metadata.kilde, "Testkilde");
        assert_eq!(hendelse.metadata.aarsak, "Er under 18 år");

        assert_eq!(hendelse.metadata.utfoert_av.bruker_type, "SYSTEM");
        assert_eq!(hendelse.metadata.utfoert_av.id, "Testsystem");
    }

    // language=JSON
    const AVVIST_HENDELSE_JSON: &'static str =
        r#"
        {
          "hendelseId": "723d5d09-83c7-4f83-97fd-35f7c9c5c798",
          "id": 1,
          "identitetsnummer": "12345678901",
          "metadata": {
            "tidspunkt": 1630404930.000000000,
            "utfoertAv": {
              "type": "SYSTEM",
              "id": "Testsystem"
            },
            "kilde": "Testkilde",
            "aarsak": "Er under 18 år"
          },
          "hendelseType": "intern.v1.avvist",
          "opplysninger": [
            "ER_UNDER_18_AAR"
          ]
        }
    "#;

}
