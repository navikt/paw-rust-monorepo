pub mod arrangor;
pub mod deltakelse;
pub mod gjennomforing;
pub mod opphav;
pub mod status;
pub mod tiltakskode;
pub mod tiltakstype;
pub mod virksomhet;

pub use arrangor::Arrangor;
pub use deltakelse::{ArenaDeltakelse, Deltakelse, TeamKometDeltakelse, TeamTiltakAvtale};
pub use gjennomforing::Gjennomforing;
pub use opphav::TiltakshistorikkOpphav;
pub use status::{
    ArenaStatus, TeamKometAarsak, TeamKometStatus, TeamKometStatusType, TeamTiltakStatus,
};
pub use tiltakskode::{TeamKometTiltakskode, TeamTiltakTiltakskode};
pub use tiltakstype::{ArenaTiltakstype, TeamKometTiltakstype, TeamTiltakTiltakstype};
pub use virksomhet::Virksomhet;

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Serialize, de::DeserializeOwned};
    use std::{fmt::Debug, str::FromStr};

    #[test]
    fn serde_and_strum_preserve_known_and_unknown_values() {
        assert_enum_round_trip(
            "GJENNOMFORES",
            ArenaStatus::Gjennomfores,
            ArenaStatus::Ukjent,
        );
        assert_enum_round_trip(
            "VENTER_PA_OPPSTART",
            TeamKometStatusType::VenterPaOppstart,
            TeamKometStatusType::Ukjent,
        );
        assert_enum_round_trip(
            "SAMARBEIDET_MED_ARRANGOREN_ER_AVBRUTT",
            TeamKometAarsak::SamarbeidetMedArrangorenErAvbrutt,
            TeamKometAarsak::Ukjent,
        );
        assert_enum_round_trip(
            "ENKELTPLASS_ARBEIDSMARKEDSOPPLAERING",
            TeamKometTiltakskode::EnkelplassArbeidsmarkedsopplaering,
            TeamKometTiltakskode::Ukjent,
        );
        assert_enum_round_trip(
            "ENKELTPLASS_FAG_OG_YRKESOPPLAERING",
            TeamKometTiltakskode::EnkelplassFagOgYrkesopplaering,
            TeamKometTiltakskode::Ukjent,
        );
        assert_enum_round_trip(
            "MIDLERTIDIG_LONNSTILSKUDD",
            TeamTiltakTiltakskode::MidlertidigLonnstilskudd,
            TeamTiltakTiltakskode::Ukjent,
        );
        assert_enum_round_trip(
            "KLAR_FOR_OPPSTART",
            TeamTiltakStatus::KlarForOppstart,
            TeamTiltakStatus::Ukjent,
        );
        assert_enum_round_trip(
            "TEAM_KOMET",
            TiltakshistorikkOpphav::TeamKomet,
            TiltakshistorikkOpphav::Ukjent,
        );
    }

    fn assert_enum_round_trip<T>(value: &str, expected: T, unknown: fn(String) -> T)
    where
        T: Debug + PartialEq + Serialize + DeserializeOwned + FromStr + AsRef<str>,
        T::Err: Debug,
    {
        for (value, expected) in [
            (value, expected),
            ("NEW_VALUE", unknown("NEW_VALUE".to_string())),
        ] {
            let json = serde_json::json!(value);
            assert_eq!(serde_json::from_value::<T>(json.clone()).unwrap(), expected);
            assert_eq!(serde_json::to_value(&expected).unwrap(), json);
            assert_eq!(value.parse::<T>().unwrap(), expected);
            assert_eq!(expected.as_ref(), value);
        }
    }
}
