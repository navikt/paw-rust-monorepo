use strum::{Display, EnumString};

#[derive(Display, EnumString)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum UtgangHendelseType {
    MetadataMottatt,
    Startet,
    PdlDataEndret,
    StatusEndretTilAvvist,
    StatusEndretTilOK,
    StatusIkkeEndret,
    Stoppet,
}

impl From<&crate::domain::utgang_hendelser::UtgangHendelser> for UtgangHendelseType {
    fn from(h: &crate::domain::utgang_hendelser::UtgangHendelser) -> Self {
        use crate::domain::utgang_hendelser::UtgangHendelser::*;
        match h {
            MetadataMottatt { .. } => Self::MetadataMottatt,
            Startet { .. } => Self::Startet,
            PdlDataEndret { .. } => Self::PdlDataEndret,
            StatusEndretTilAvvist { .. } => Self::StatusEndretTilAvvist,
            StatusEndretTilOK { .. } => Self::StatusEndretTilOK,
            StatusIkkeEndret { .. } => Self::StatusIkkeEndret,
            Stoppet { .. } => Self::Stoppet,
        }
    }
}
