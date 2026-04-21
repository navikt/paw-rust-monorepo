use chrono::{DateTime, Utc};
use interne_hendelser::{
    Avvist, InterneHendelser, Startet,
    vo::{Bruker, BrukerType, Metadata, Opplysning},
};
use std::collections::HashSet;
use uuid::Uuid;

pub fn rfc3339(input: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(input)
        .unwrap_or_else(|e| panic!("Ugyldig RFC 3339-tidspunkt '{input}': {e}"))
        .with_timezone(&Utc)
}

fn default_tidspunkt() -> DateTime<Utc> {
    rfc3339("2024-01-01T12:00:00Z")
}

fn default_ident() -> String {
    "12345678901".to_string()
}

pub struct StartetBuilder {
    pub hendelse_id: Uuid,
    pub arbeidssoeker_id: i64,
    pub identitetsnummer: String,
    pub tidspunkt: DateTime<Utc>,
    pub bruker_type: BrukerType,
    pub utfoert_av_id: String,
    pub opplysninger: HashSet<Opplysning>,
}

impl Default for StartetBuilder {
    fn default() -> Self {
        let ident = default_ident();
        Self {
            hendelse_id: Uuid::new_v4(),
            arbeidssoeker_id: 1,
            identitetsnummer: ident.clone(),
            tidspunkt: default_tidspunkt(),
            bruker_type: BrukerType::Sluttbruker,
            utfoert_av_id: ident,
            opplysninger: HashSet::new(),
        }
    }
}

impl StartetBuilder {
    pub fn build(&self) -> Startet {
        Startet {
            hendelse_id: self.hendelse_id,
            id: self.arbeidssoeker_id,
            identitetsnummer: self.identitetsnummer.clone(),
            metadata: Metadata {
                tidspunkt: self.tidspunkt,
                utfoert_av: Bruker {
                    bruker_type: self.bruker_type.clone(),
                    id: self.utfoert_av_id.clone(),
                    sikkerhetsnivaa: None,
                },
                kilde: "Testkilde".to_string(),
                aarsak: "Test".to_string(),
                tidspunkt_fra_kilde: None,
            },
            opplysninger: self.opplysninger.clone(),
        }
    }
}

pub trait AsJson {
    fn as_json(&self) -> String;
}

impl AsJson for Startet {
    fn as_json(&self) -> String {
        serde_json::to_string(&InterneHendelser::Startet(self.clone()))
            .expect("Startet skal kunne serialiseres")
    }
}

impl AsJson for Avvist {
    fn as_json(&self) -> String {
        serde_json::to_string(&InterneHendelser::Avvist(self.clone()))
            .expect("Avvist skal kunne serialiseres")
    }
}

pub struct AvvistBuilder {
    pub hendelse_id: Uuid,
    pub arbeidssoeker_id: i64,
    pub identitetsnummer: String,
    pub tidspunkt: DateTime<Utc>,
    pub bruker_type: BrukerType,
    pub utfoert_av_id: String,
    pub opplysninger: HashSet<Opplysning>,
    pub handling: Option<String>,
}

impl Default for AvvistBuilder {
    fn default() -> Self {
        let ident = default_ident();
        Self {
            hendelse_id: Uuid::new_v4(),
            arbeidssoeker_id: 1,
            identitetsnummer: ident.clone(),
            tidspunkt: default_tidspunkt(),
            bruker_type: BrukerType::Sluttbruker,
            utfoert_av_id: ident,
            opplysninger: HashSet::new(),
            handling: None,
        }
    }
}

impl AvvistBuilder {
    pub fn build(&self) -> Avvist {
        Avvist {
            hendelse_id: self.hendelse_id,
            id: self.arbeidssoeker_id,
            identitetsnummer: self.identitetsnummer.clone(),
            metadata: Metadata {
                tidspunkt: self.tidspunkt,
                utfoert_av: Bruker {
                    bruker_type: self.bruker_type.clone(),
                    id: self.utfoert_av_id.clone(),
                    sikkerhetsnivaa: None,
                },
                kilde: "Testkilde".to_string(),
                aarsak: "Test".to_string(),
                tidspunkt_fra_kilde: None,
            },
            opplysninger: self.opplysninger.clone(),
            handling: self.handling.clone(),
        }
    }
}
