#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, sqlx::Type, serde::Deserialize)]
#[sqlx(type_name = "alvorlighet", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Alvorlighet {
    Info,
    Advarsel,
    Kritisk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Utfall {
    pub kode: i16,
    pub alvorlighet: Alvorlighet,
}

impl Alvorlighet {
    pub const ALLE: [Alvorlighet; 3] = [Self::Kritisk, Self::Advarsel, Self::Info];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Advarsel => "advarsel",
            Self::Kritisk => "kritisk",
        }
    }
}
