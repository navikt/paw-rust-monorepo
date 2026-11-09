use crate::leesah::error::{EnumTypeParseError, enum_type_not_found};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use strum::{AsRefStr, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, EnumString, AsRefStr)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Endringstype {
    Opprettet,
    Korrigert,
    Annullert,
    Opphoert,
}

/// Utvid denne ved behov. Den er en streng i kontrakten, så den er ikke strengt enumerert.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, EnumString, AsRefStr)]
#[strum(
    serialize_all = "SCREAMING_SNAKE_CASE",
    parse_err_fn = enum_type_not_found,
    parse_err_ty = EnumTypeParseError
)]
pub enum Opplysningstype {
    AdressebeskyttelseV1,
    DeltBostedV1,
    DoedfoedtBarnV1,
    DoedsfallV1,
    FalskIdV1,
    FamilierelasjonV1,
    FoedselV1,
    FoedselsdatoV1,
    ForelderbarnrelasjonV1,
    SivilstandV1,
    VergemaalEllerFremtidsfullmaktV1,
    UtflyttingFraNorge,
    InnflyttingTilNorge,
    FolkeregisteridentifikatorV1,
    NavnV1,
    SikkerhetstiltakV1,
    StatsborgerskapV1,
    TelefonnummerV1,
    KontaktadresseV1,
    BostedsadresseV1,
    OppholdsadresseV1,
    #[default]
    UkjentVerdi,
}

impl Serialize for Opplysningstype {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_ref())
    }
}

impl<'de> Deserialize<'de> for Opplysningstype {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(value.parse().unwrap_or_else(|_| Self::UkjentVerdi))
    }
}

#[cfg(test)]
mod tests {
    use super::{Endringstype, Opplysningstype};
    use apache_avro::{Reader, Schema, Writer, from_value, to_value, types::Value};

    #[test]
    fn opplysningstype_serialiseres_som_avro_string() {
        let mappings = [
            (
                Opplysningstype::AdressebeskyttelseV1,
                "ADRESSEBESKYTTELSE_V1",
            ),
            (Opplysningstype::DeltBostedV1, "DELT_BOSTED_V1"),
            (Opplysningstype::DoedfoedtBarnV1, "DOEDFOEDT_BARN_V1"),
            (Opplysningstype::DoedsfallV1, "DOEDSFALL_V1"),
            (Opplysningstype::FalskIdV1, "FALSK_ID_V1"),
            (Opplysningstype::FamilierelasjonV1, "FAMILIERELASJON_V1"),
            (Opplysningstype::FoedselV1, "FOEDSEL_V1"),
            (Opplysningstype::FoedselsdatoV1, "FOEDSELSDATO_V1"),
            (
                Opplysningstype::ForelderbarnrelasjonV1,
                "FORELDERBARNRELASJON_V1",
            ),
            (Opplysningstype::SivilstandV1, "SIVILSTAND_V1"),
            (
                Opplysningstype::VergemaalEllerFremtidsfullmaktV1,
                "VERGEMAAL_ELLER_FREMTIDSFULLMAKT_V1",
            ),
            (Opplysningstype::UtflyttingFraNorge, "UTFLYTTING_FRA_NORGE"),
            (
                Opplysningstype::InnflyttingTilNorge,
                "INNFLYTTING_TIL_NORGE",
            ),
            (
                Opplysningstype::FolkeregisteridentifikatorV1,
                "FOLKEREGISTERIDENTIFIKATOR_V1",
            ),
            (Opplysningstype::NavnV1, "NAVN_V1"),
            (Opplysningstype::SikkerhetstiltakV1, "SIKKERHETSTILTAK_V1"),
            (Opplysningstype::StatsborgerskapV1, "STATSBORGERSKAP_V1"),
            (Opplysningstype::TelefonnummerV1, "TELEFONNUMMER_V1"),
            (Opplysningstype::KontaktadresseV1, "KONTAKTADRESSE_V1"),
            (Opplysningstype::BostedsadresseV1, "BOSTEDSADRESSE_V1"),
            (Opplysningstype::OppholdsadresseV1, "OPPHOLDSADRESSE_V1"),
            (Opplysningstype::UkjentVerdi, "UKJENT_VERDI"),
        ];

        for (opplysningstype, expected) in mappings {
            assert_eq!(
                to_value(opplysningstype).unwrap(),
                Value::String(expected.to_string()),
                "{opplysningstype:?}"
            );
        }
    }

    #[test]
    fn opplysningstype_deserialiseres_fra_avro_string() {
        let mappings = [
            (
                "ADRESSEBESKYTTELSE_V1",
                Opplysningstype::AdressebeskyttelseV1,
            ),
            ("DELT_BOSTED_V1", Opplysningstype::DeltBostedV1),
            ("DOEDFOEDT_BARN_V1", Opplysningstype::DoedfoedtBarnV1),
            ("DOEDSFALL_V1", Opplysningstype::DoedsfallV1),
            ("FALSK_ID_V1", Opplysningstype::FalskIdV1),
            ("FAMILIERELASJON_V1", Opplysningstype::FamilierelasjonV1),
            ("FOEDSEL_V1", Opplysningstype::FoedselV1),
            ("FOEDSELSDATO_V1", Opplysningstype::FoedselsdatoV1),
            (
                "FORELDERBARNRELASJON_V1",
                Opplysningstype::ForelderbarnrelasjonV1,
            ),
            ("SIVILSTAND_V1", Opplysningstype::SivilstandV1),
            (
                "VERGEMAAL_ELLER_FREMTIDSFULLMAKT_V1",
                Opplysningstype::VergemaalEllerFremtidsfullmaktV1,
            ),
            ("UTFLYTTING_FRA_NORGE", Opplysningstype::UtflyttingFraNorge),
            (
                "INNFLYTTING_TIL_NORGE",
                Opplysningstype::InnflyttingTilNorge,
            ),
            (
                "FOLKEREGISTERIDENTIFIKATOR_V1",
                Opplysningstype::FolkeregisteridentifikatorV1,
            ),
            ("NAVN_V1", Opplysningstype::NavnV1),
            ("SIKKERHETSTILTAK_V1", Opplysningstype::SikkerhetstiltakV1),
            ("STATSBORGERSKAP_V1", Opplysningstype::StatsborgerskapV1),
            ("TELEFONNUMMER_V1", Opplysningstype::TelefonnummerV1),
            ("KONTAKTADRESSE_V1", Opplysningstype::KontaktadresseV1),
            ("BOSTEDSADRESSE_V1", Opplysningstype::BostedsadresseV1),
            ("OPPHOLDSADRESSE_V1", Opplysningstype::OppholdsadresseV1),
            ("UKJENT_VERDI", Opplysningstype::UkjentVerdi),
        ];

        for (value, expected) in mappings {
            assert_eq!(
                from_value::<Opplysningstype>(&Value::String(value.to_string())).unwrap(),
                expected,
                "{value}"
            );
        }
    }

    #[test]
    fn ukjent_opplysningstype_string_blir_ukjent_verdi() {
        assert_eq!(
            from_value::<Opplysningstype>(&Value::String("NY_OPPLYSNING".to_string())).unwrap(),
            Opplysningstype::UkjentVerdi
        );
    }

    #[test]
    fn opplysningstype_avviser_verdier_som_ikke_er_strings() {
        assert!(from_value::<Opplysningstype>(&Value::Int(1)).is_err());
    }

    #[test]
    fn endringstype_serialiseres_fortsatt_som_avro_enum() {
        let schema = Schema::parse_str(
            r#"{"type":"enum","name":"Endringstype","symbols":["OPPRETTET","KORRIGERT","ANNULLERT","OPPHOERT"]}"#,
        )
        .unwrap();
        let mut writer = Writer::new(&schema, Vec::new()).unwrap();
        writer.append_ser(Endringstype::Opprettet).unwrap();

        let encoded = writer.into_inner().unwrap();
        let decoded = Reader::new(encoded.as_slice())
            .unwrap()
            .next()
            .unwrap()
            .unwrap();
        assert_eq!(decoded, Value::Enum(0, "OPPRETTET".to_string()));
    }
}
