use crate::metadata::Metadata;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Periode {
    pub id: Uuid,
    pub identitetsnummer: String,
    pub startet: Metadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avsluttet: Option<Metadata>,
}

impl Periode {
    pub fn is_active(&self) -> bool {
        self.avsluttet.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bruker::Bruker;
    use crate::brukertype::BrukerType;
    use crate::metadata::Metadata;
    use chrono::Utc;

    #[test]
    fn test_periode_is_active() {
        let active_periode = Periode {
            id: Uuid::new_v4(),
            identitetsnummer: "12345678901".to_string(),
            startet: create_test_metadata(),
            avsluttet: None,
        };
        assert!(active_periode.is_active());

        let ended_periode = Periode {
            id: Uuid::new_v4(),
            identitetsnummer: "12345678901".to_string(),
            startet: create_test_metadata(),
            avsluttet: Some(create_test_metadata()),
        };
        assert!(!ended_periode.is_active());
    }

    fn create_test_metadata() -> Metadata {
        Metadata {
            tidspunkt: Utc::now(),
            utfoert_av: Bruker {
                bruker_type: BrukerType::Sluttbruker,
                id: "12345678901".to_string(),
                sikkerhetsnivaa: Some("tokenx:Level4".to_string()),
            },
            kilde: "test-system".to_string(),
            aarsak: "Test".to_string(),
            tidspunkt_fra_kilde: None,
        }
    }
}
