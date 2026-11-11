use crate::leesah::opplysninger::adressebeskyttelse::Adressebeskyttelse;
use crate::leesah::opplysninger::bostedsadresse::Bostedsadresse;
use crate::leesah::opplysninger::delt_bosted::DeltBosted;
use crate::leesah::opplysninger::doedfoedt_barn::DoedfoedtBarn;
use crate::leesah::opplysninger::doedsfall::Doedsfall;
use crate::leesah::opplysninger::falsk_identitet::FalskIdentitet;
use crate::leesah::opplysninger::familierelasjon::Familierelasjon;
use crate::leesah::opplysninger::foedsel::Foedsel;
use crate::leesah::opplysninger::foedselsdato::Foedselsdato;
use crate::leesah::opplysninger::folkeregisteridentifikator::Folkeregisteridentifikator;
use crate::leesah::opplysninger::forelder_barn_relasjon::ForelderBarnRelasjon;
use crate::leesah::opplysninger::innflytting::InnflyttingTilNorge;
use crate::leesah::opplysninger::kontaktadresse::Kontaktadresse;
use crate::leesah::opplysninger::navn::Navn;
use crate::leesah::opplysninger::sikkerhetstiltak::Sikkerhetstiltak;
use crate::leesah::opplysninger::sivilstand::Sivilstand;
use crate::leesah::opplysninger::statsborgerskap::Statsborgerskap;
use crate::leesah::opplysninger::telefonnummer::Telefonnummer;
use crate::leesah::opplysninger::utflytting::UtflyttingFraNorge;
use crate::leesah::opplysninger::verge::VergemaalEllerFremtidsfullmakt;
use crate::leesah::{Endringstype, Opplysningstype};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{TimestampMilliSeconds, serde_as};

#[serde_as]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Personhendelse {
    pub hendelse_id: String,
    pub personidenter: Vec<String>,
    pub master: String,
    #[serde_as(as = "TimestampMilliSeconds<i64>")]
    pub opprettet: DateTime<Utc>,
    pub opplysningstype: Opplysningstype,
    pub endringstype: Endringstype,
    pub tidligere_hendelse_id: Option<String>,
    pub adressebeskyttelse: Option<Adressebeskyttelse>,
    pub delt_bosted: Option<DeltBosted>,
    pub doedfoedt_barn: Option<DoedfoedtBarn>,
    pub doedsfall: Option<Doedsfall>,
    pub falsk_identitet: Option<FalskIdentitet>,
    pub foedsel: Option<Foedsel>,
    pub foedselsdato: Option<Foedselsdato>,
    pub forelder_barn_relasjon: Option<ForelderBarnRelasjon>,
    pub familierelasjon: Option<Familierelasjon>,
    pub sivilstand: Option<Sivilstand>,
    pub vergemaal_eller_fremtidsfullmakt: Option<VergemaalEllerFremtidsfullmakt>,
    pub utflytting_fra_norge: Option<UtflyttingFraNorge>,
    #[serde(rename = "InnflyttingTilNorge")]
    pub innflytting_til_norge: Option<InnflyttingTilNorge>,
    #[serde(rename = "Folkeregisteridentifikator")]
    pub folkeregisteridentifikator: Option<Folkeregisteridentifikator>,
    pub navn: Option<Navn>,
    pub sikkerhetstiltak: Option<Sikkerhetstiltak>,
    pub statsborgerskap: Option<Statsborgerskap>,
    pub telefonnummer: Option<Telefonnummer>,
    pub kontaktadresse: Option<Kontaktadresse>,
    pub bostedsadresse: Option<Bostedsadresse>,
}

impl std::fmt::Debug for Personhendelse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Personhendelse")
            .field("hendelse_id", &self.hendelse_id)
            .field("master", &self.master)
            .field("opprettet", &self.opprettet)
            .field("opplysningstype", &self.opplysningstype)
            .field("endringstype", &self.endringstype)
            .field("tidligere_hendelse_id", &self.tidligere_hendelse_id)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for Personhendelse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Personhendelse")
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use mockito::Server;
    use paw_avro_serde::serde::{
        AvroDeserializer, AvroSerializer, AvroStringDeserializer, SubjectNameStrategy,
    };
    use schema_registry_mock::schema_definitions::avro_schemas;
    use schema_registry_mock::schema_registry_mock::create_schema_registry_mock_2;
    use uuid::Uuid;

    #[test]
    fn debug_skjuler_personopplysninger() {
        let personhendelse = create_dummy_personhendelse_navn();
        let debug = format!("{:?}", personhendelse);
        assert!(debug.contains("FREG"));
        for pii in ["01017012345", "Luke", "Skywalker"] {
            assert!(!debug.contains(pii), "Debug inneholder {pii}: {debug}");
        }
    }

    #[tokio::test]
    async fn serde_test_key() {
        let mut mockito_server = Server::new_async().await;
        let schemas = avro_schemas();
        let guard = create_schema_registry_mock_2(&mut mockito_server, schemas)
            .await
            .unwrap();
        let schema_registry_settings = guard.schema_registry_settings;

        let serializer = AvroSerializer::new(schema_registry_settings.clone());
        let deserializer = AvroStringDeserializer::new(schema_registry_settings.clone());
        let key_naming_strategy =
            SubjectNameStrategy::TopicNameStrategy("pdl.leesah-v1".to_string(), true);

        let source_avro = "123456".to_string();
        let payload = serializer
            .serialize(&source_avro, &key_naming_strategy)
            .await
            .unwrap();
        let target_avro = deserializer.deserialize(&payload).await.unwrap();

        assert_eq!(target_avro, source_avro);
    }

    #[tokio::test]
    async fn serde_test_navn_endring() {
        let mut mockito_server = Server::new_async().await;
        let schemas = avro_schemas();
        let guard = create_schema_registry_mock_2(&mut mockito_server, schemas)
            .await
            .unwrap();
        let schema_registry_settings = guard.schema_registry_settings;

        let serializer = AvroSerializer::new(schema_registry_settings.clone());
        let deserializer = AvroDeserializer::new(schema_registry_settings.clone());
        let value_naming_strategy =
            SubjectNameStrategy::TopicNameStrategy("pdl.leesah-v1".to_string(), false);

        let source_avro = create_dummy_personhendelse_navn();
        let payload = serializer
            .serialize(&source_avro, &value_naming_strategy)
            .await
            .unwrap();
        let target_avro: Personhendelse = deserializer.deserialize(&payload).await.unwrap();

        assert_eq!(target_avro, source_avro);
    }

    pub fn create_dummy_personhendelse_navn() -> Personhendelse {
        Personhendelse {
            hendelse_id: Uuid::new_v4().to_string(),
            personidenter: vec!["01017012345".to_string()],
            master: "FREG".to_string(),
            opprettet: DateTime::<Utc>::from_timestamp_millis(1_700_000_000_000).unwrap(),
            opplysningstype: Opplysningstype::NavnV1,
            endringstype: Endringstype::Opprettet,
            tidligere_hendelse_id: None,
            adressebeskyttelse: None,
            delt_bosted: None,
            doedfoedt_barn: None,
            doedsfall: None,
            falsk_identitet: None,
            foedsel: None,
            foedselsdato: None,
            forelder_barn_relasjon: None,
            familierelasjon: None,
            sivilstand: None,
            vergemaal_eller_fremtidsfullmakt: None,
            utflytting_fra_norge: None,
            innflytting_til_norge: None,
            folkeregisteridentifikator: None,
            navn: Some(Navn {
                fornavn: "Luke".to_string(),
                mellomnavn: None,
                etternavn: "Skywalker".to_string(),
                forkortet_navn: None,
                originalt_navn: None,
                gyldig_fra_og_med: None,
            }),
            sikkerhetstiltak: None,
            statsborgerskap: None,
            telefonnummer: None,
            kontaktadresse: None,
            bostedsadresse: None,
        }
    }
}
