use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "utfoert_av_type", rename_all = "snake_case")]
pub enum UtfoertAvType {
    Udefinert,
    UkjentVerdi,
    System,
    Sluttbruker,
    Veileder,
}

impl From<&interne_hendelser::vo::BrukerType> for UtfoertAvType {
    fn from(t: &interne_hendelser::vo::BrukerType) -> Self {
        use interne_hendelser::vo::BrukerType as B;
        match t {
            B::Udefinert => Self::Udefinert,
            B::UkjentVerdi => Self::UkjentVerdi,
            B::System => Self::System,
            B::Sluttbruker => Self::Sluttbruker,
            B::Veileder => Self::Veileder,
        }
    }
}

impl From<&eksterne_hendelser::vo::brukertype::BrukerType> for UtfoertAvType {
    fn from(t: &eksterne_hendelser::vo::brukertype::BrukerType) -> Self {
        use eksterne_hendelser::vo::brukertype::BrukerType as B;
        match t {
            B::Udefinert => Self::Udefinert,
            B::UkjentVerdi => Self::UkjentVerdi,
            B::System => Self::System,
            B::Sluttbruker => Self::Sluttbruker,
            B::Veileder => Self::Veileder,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Utfoert {
    pub tidspunkt_ms: i64,
    pub av_type: UtfoertAvType,
    pub av_id: String,
}

impl std::fmt::Debug for Utfoert {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Utfoert")
            .field("tidspunkt_ms", &self.tidspunkt_ms)
            .field("av_type", &self.av_type)
            .finish_non_exhaustive()
    }
}

impl Utfoert {
    pub fn fra_intern(metadata: &interne_hendelser::vo::Metadata) -> Self {
        Self {
            tidspunkt_ms: millis_fra_flyttall_sekunder(metadata.tidspunkt),
            av_type: (&metadata.utfoert_av.bruker_type).into(),
            av_id: metadata.utfoert_av.id.clone(),
        }
    }

    pub fn fra_ekstern(
        tidspunkt: DateTime<Utc>,
        utfoert_av: &eksterne_hendelser::vo::bruker::Bruker,
    ) -> Self {
        Self {
            tidspunkt_ms: tidspunkt.timestamp_millis(),
            av_type: (&utfoert_av.bruker_type).into(),
            av_id: utfoert_av.id.clone(),
        }
    }
}

// Hendelseloggen kan ha tidspunkt som sekunder i f64. Uten avrunding til
// mikrosekunder først blir .123 til 122 ms.
fn millis_fra_flyttall_sekunder(tidspunkt: DateTime<Utc>) -> i64 {
    let mikros = (i64::from(tidspunkt.timestamp_subsec_nanos()) + 500) / 1_000;
    tidspunkt.timestamp() * 1_000 + mikros / 1_000
}
