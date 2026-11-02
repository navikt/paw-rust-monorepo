use bekreftelse_interne_hendelser::BekreftelseHendelse;
use interne_hendelser::InterneHendelser;
use rdkafka::Message;
use rdkafka::message::{Headers, OwnedMessage};

pub(super) fn kilde_fra_hendelselogg(payload: Option<&[u8]>) -> Option<String> {
    let hendelse = serde_json::from_slice::<InterneHendelser>(payload?).ok()?;
    let kilde = match hendelse {
        InterneHendelser::Startet(h) => h.metadata.kilde,
        InterneHendelser::Avsluttet(h) => h.metadata.kilde,
        InterneHendelser::Avvist(h) => h.metadata.kilde,
        InterneHendelser::AvvistStoppAvPeriode(h) => h.metadata.kilde,
        InterneHendelser::ArbeidssoekerIdFlettetInn(h) => h.metadata.kilde,
        InterneHendelser::AutomatiskIdMergeIkkeMulig(h) => h.metadata.kilde,
        InterneHendelser::IdentitetsnummerSammenslaatt(h) => h.metadata.kilde,
        InterneHendelser::OpplysningerOmArbeidssoekerMottatt(h) => h.metadata().kilde.clone(),
    };
    (!kilde.is_empty()).then_some(kilde)
}

pub(super) fn kilde_fra_bekreftelse_hendelselogg(payload: Option<&[u8]>) -> Option<String> {
    let hendelse = BekreftelseHendelse::from_slice(payload?).ok()?;
    let kilde = match hendelse {
        BekreftelseHendelse::RegisterGracePeriodeUtloept(h) => Some(h.kilde),
        BekreftelseHendelse::BaOmAaAvsluttePeriode(h) => Some(h.kilde),
        BekreftelseHendelse::RegisterGracePeriodeUtloeptEtterEksternInnsamling(h) => Some(h.kilde),
        _ => None,
    }?;
    (!kilde.is_empty()).then_some(kilde)
}

pub(super) fn kilde_fra_header(msg: &OwnedMessage) -> Option<String> {
    let header = msg
        .headers()?
        .iter()
        .filter(|header| header.key == "source")
        .last()?;
    let value = header.value?;
    let kilde = String::from_utf8_lossy(value).into_owned();
    (!kilde.is_empty()).then_some(kilde)
}

#[cfg(test)]
mod tests {
    use super::*;

    const METADATA: &str = r#""metadata": {"tidspunkt": 1770897349.3, "utfoertAv": {"type": "SYSTEM", "id": "x"}, "kilde": "test-kilde", "aarsak": "x"}"#;

    fn hendelse(felter: &str) -> Vec<u8> {
        format!(
            r#"{{"hendelseId":"00000000-0000-0000-0000-000000000001", "identitetsnummer":"1", {METADATA}, {felter}}}"#
        )
        .into_bytes()
    }

    fn melding(
        topic: &str,
        payload: Option<Vec<u8>>,
        headers: Option<rdkafka::message::OwnedHeaders>,
    ) -> OwnedMessage {
        OwnedMessage::new(
            payload,
            None,
            topic.into(),
            rdkafka::Timestamp::NotAvailable,
            0,
            0,
            headers,
        )
    }

    #[test]
    fn hendelselogg_henter_kilde_fra_metadata() {
        assert_eq!(
            kilde_fra_hendelselogg(Some(&hendelse(
                r#""hendelseType":"intern.v1.startet", "id": 42"#
            ))),
            Some("test-kilde".to_string())
        );
    }

    #[tokio::test]
    async fn tom_payload_kilde_faller_tilbake_til_header() {
        use paw_kafka::topics::Topic;
        use rdkafka::message::{Header, OwnedHeaders};
        use schema_registry_converter::async_impl::schema_registry::SrSettings;

        let dekodere = crate::meldinger::Dekodere::new(
            [("hendelselogg", &Topic::Hendelselogg)],
            SrSettings::new("http://127.0.0.1:1".into()),
        );
        let payload = hendelse(r#""hendelseType":"intern.v1.startet", "id": 42"#);
        let payload = String::from_utf8(payload)
            .unwrap()
            .replace("test-kilde", "");
        let mut headers = OwnedHeaders::new();
        headers = headers.insert(Header {
            key: "source",
            value: Some(b"header-kilde"),
        });
        assert_eq!(
            dekodere
                .kilde(&melding(
                    "hendelselogg",
                    Some(payload.into_bytes()),
                    Some(headers),
                ))
                .await
                .unwrap(),
            Some("header-kilde".to_string())
        );
    }

    #[test]
    fn siste_source_header_vinner() {
        use rdkafka::message::{Header, OwnedHeaders};

        let mut headers = OwnedHeaders::new();
        headers = headers
            .insert(Header {
                key: "source",
                value: Some(b"forrige"),
            })
            .insert(Header {
                key: "source",
                value: Some(b"siste"),
            });
        assert_eq!(
            kilde_fra_header(&melding("topic", None, Some(headers))),
            Some("siste".into())
        );
    }

    #[test]
    fn uten_payloadkilde_og_header_returnerer_none() {
        assert_eq!(kilde_fra_header(&melding("topic", None, None)), None);
    }

    #[test]
    fn flettet_inn_bruker_metadata_kilde_ikke_kilde_struct() {
        let payload = hendelse(
            r#""hendelseType":"intern.v1.arbeidssoeker_id_flettet_inn", "id":42, "kilde":{"arbeidssoekerId":7,"identitetsnummer":["2"]}"#,
        );
        assert_eq!(
            kilde_fra_hendelselogg(Some(&payload)),
            Some("test-kilde".into())
        );
    }

    #[tokio::test]
    async fn periode_kilde_kommer_fra_avsluttet_metadata_nar_den_finnes() {
        use paw_kafka::topics::Topic;
        use schema_registry_mock::schema_registry_mock::create_schema_registry_mock;
        use test_data_generator::avro::AvroGenerator;
        use test_data_generator::eksterne_hendelser::{
            create_dummy_avslutt_periode, create_dummy_start_periode,
        };
        use uuid::Uuid;

        const PERIODE: &str = "paw.arbeidssokerperioder-v1";

        let mut server = mockito::Server::new_async().await;
        let mock = create_schema_registry_mock(&mut server).await.unwrap();
        let generator = AvroGenerator::new(mock.schema_registry_settings.clone());
        let dekodere = crate::meldinger::Dekodere::new(
            [(PERIODE, &Topic::Periode)],
            mock.schema_registry_settings,
        );

        let mut periode =
            create_dummy_avslutt_periode("12345678901", Uuid::from_u128(1), None, None);
        periode.startet.kilde = "startet".to_string();
        periode.avsluttet.as_mut().unwrap().kilde = "avsluttet".to_string();
        let payload = generator.create_avro_payload(PERIODE, periode).await;
        assert_eq!(
            dekodere
                .kilde(&melding(PERIODE, Some(payload), None))
                .await
                .unwrap(),
            Some("avsluttet".to_string())
        );

        let mut startet = create_dummy_start_periode("12345678901", Uuid::from_u128(2), None);
        startet.startet.kilde = "startet".to_string();
        let payload = generator.create_avro_payload(PERIODE, startet).await;
        assert_eq!(
            dekodere
                .kilde(&melding(PERIODE, Some(payload), None))
                .await
                .unwrap(),
            Some("startet".to_string())
        );
    }
}
