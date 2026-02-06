use chrono::{DateTime, Utc};
use crate::avvist_hendelse::AvvistHendelse;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct AvvistMeldingRow {
    pub melding_id: Uuid,
    pub aarsak: String,
    pub identitetsnummer: String,
    pub arbeidssoeker_id: i64,
    pub tidspunkt: DateTime<Utc>,
}

impl From<AvvistHendelse> for AvvistMeldingRow {
    fn from(hendelse: AvvistHendelse) -> Self {
        AvvistMeldingRow {
            melding_id: hendelse.hendelse_id,
            aarsak: hendelse.metadata.aarsak,
            identitetsnummer: hendelse.identitetsnummer,
            arbeidssoeker_id: hendelse.id,
            tidspunkt: hendelse.metadata.tidspunkt,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::avvist_hendelse::{Metadata, UtfoertAv};
    use uuid::Uuid;

    #[test]
    fn test_avvist_hendelse_to_avvist_melding_row() {
        let hendelse_id = Uuid::new_v4();
        let id = 12345;
        let aarsak = "Test årsak".to_string();
        let identitetsnummer = "12345678901".to_string();
        let now= Utc::now();

        let avvist_hendelse = AvvistHendelse {
            hendelse_id,
            id,
            identitetsnummer: identitetsnummer.clone(),
            metadata: Metadata {
                tidspunkt: now,
                utfoert_av: UtfoertAv {
                    bruker_type: "System".to_string(),
                    id: "123".to_string(),
                },
                kilde: "Testkilde".to_string(),
                aarsak: aarsak.clone(),
            },
            hendelse_type: "TestType".to_string(),
            opplysninger: vec![],
        };

        let avvist_melding_row = AvvistMeldingRow::from(avvist_hendelse.clone());

        assert_eq!(avvist_melding_row.melding_id, avvist_hendelse.hendelse_id);
        assert_eq!(avvist_melding_row.aarsak, avvist_hendelse.metadata.aarsak);
        assert_eq!(
            avvist_melding_row.identitetsnummer,
            avvist_hendelse.identitetsnummer
        );
        assert_eq!(avvist_melding_row.arbeidssoeker_id, avvist_hendelse.id);
        assert_eq!(
            avvist_melding_row.tidspunkt,
            avvist_hendelse.metadata.tidspunkt
        );
    }
}
