//! Felles oppsett: Postgres og innlesing satt opp som i app_runner, med ekte topic-katalog.
#![allow(dead_code, reason = "Ikke alle testfiler bruker alt")]

pub mod melding;

use std::collections::HashMap;
use std::sync::Mutex;

use melding::{Innhold, Melding};

use arbeidssoekerregisteret_internkontroll::features::innlesing::{
    message_processor::InternkontrollMessageProcessor,
    metadata::{meldinger::Dekodere, signing_keys::SigningKeyDbIds},
    topic::{load_topic_ids, register_topics},
    topic_catalog::subscribed_topics,
    topic_ids::TopicIds,
};
use arbeidssoekerregisteret_internkontroll::features::regler::{
    kjoring::kjor_runde,
    regel::{Regel, registrer_regler},
};
use mockito::ServerGuard;
use paw_kafka::hwm::rebalance::topic_partition_update::TopicPartition;
use paw_kafka::hwm::stream::assigned_partitions::AssignedPartitions;
use paw_kafka::signing::RecordVerifier;
use paw_kafka::topics::{Topic, get_topic};
use paw_rust_base::env::RuntimeEnv;
use postgres_testcontainer::postgres::{TestContainerGuard, setup_postgres_container};
use rdkafka::message::OwnedMessage;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use schema_registry_mock::schema_registry_mock::{
    SchemaRegistryMockGuard, create_schema_registry_mock,
};
use sqlx::PgPool;

pub struct TestApp {
    pub pool: PgPool,
    pub topic_ids: TopicIds,
    pub processor: InternkontrollMessageProcessor,
    pub sr_settings: SrSettings,
    neste_offset: Mutex<HashMap<(String, i32), i64>>,
    _pg: TestContainerGuard,
    _sr: (ServerGuard, SchemaRegistryMockGuard),
}

pub fn topic_navn(topic: &Topic) -> &'static str {
    get_topic(&RuntimeEnv::Local, topic)
}

impl TestApp {
    pub async fn start() -> Self {
        let pg = setup_postgres_container().await.unwrap();
        let pool = pg.pg_pool.clone();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        let mut server = mockito::Server::new_async().await;
        let sr = create_schema_registry_mock(&mut server).await.unwrap();
        let sr_settings = sr.schema_registry_settings.clone();

        let catalog = subscribed_topics(RuntimeEnv::Local);
        let mut conn = pool.acquire().await.unwrap();
        register_topics(&mut conn, &catalog.topic_names())
            .await
            .unwrap();
        let topic_ids = load_topic_ids(&mut conn).await.unwrap();
        let record_verifier = RecordVerifier::new();
        let signing_key_id_map = SigningKeyDbIds::new_from_db(record_verifier.key_ids(), &mut conn)
            .await
            .unwrap();
        let processor = InternkontrollMessageProcessor::new(
            catalog.signing_required_catalog(),
            record_verifier,
            topic_ids.clone(),
            Dekodere::new(catalog.topic_names_mapping(), sr_settings.clone()),
            catalog,
            signing_key_id_map,
        );
        Self {
            pool,
            topic_ids,
            processor,
            sr_settings,
            neste_offset: Mutex::default(),
            _pg: pg,
            _sr: (server, sr),
        }
    }

    pub fn topic_id(&self, topic: &Topic) -> i16 {
        self.topic_ids.get(topic_navn(topic)).unwrap()
    }

    /// Bygger meldingen og leser den inn. Returnerer meldingen slik den ble lest.
    pub async fn les_inn<I: Innhold>(&self, melding: impl Into<Melding<I>>) -> OwnedMessage {
        let melding = melding.into();
        let offset = {
            let mut neste = self.neste_offset.lock().unwrap();
            let neste = neste
                .entry((
                    topic_navn(&melding.innhold.topic()).to_string(),
                    melding.partisjon,
                ))
                .or_default();
            let offset = melding.offset.unwrap_or(*neste);
            *neste = offset + 1;
            offset
        };
        let msg = melding.bygg(offset, &self.sr_settings).await;
        self.les_inn_raa(&msg).await;
        msg
    }

    /// For meldinger som ikke har egen type i [`melding`], for eksempel ugyldig innhold.
    pub async fn les_inn_raa(&self, msg: &OwnedMessage) {
        let mut tx = self.pool.begin().await.unwrap();
        self.processor.process(&mut tx, msg).await.unwrap();
        tx.commit().await.unwrap();
    }
}

impl TestApp {
    /// Kjører `regler` på partisjon 0 til det ikke er mer å gjøre.
    pub async fn kjor_regler(&self, regler: Vec<Box<dyn Regel>>) {
        let regler = registrer_regler(regler, &self.topic_ids, &RuntimeEnv::Local);
        let tildelt = AssignedPartitions::default();
        tildelt.add([TopicPartition {
            topic: topic_navn(&Topic::Hendelselogg).to_string(),
            partition: 0,
        }]);
        while kjor_runde(&self.pool, &regler, &tildelt, 2).await.unwrap() > 0 {}
    }

    /// (topic, offset, kode) for alle funn, sortert.
    pub async fn funn(&self) -> Vec<(String, i64, i16)> {
        sqlx::query_as(
            "SELECT t.name::text, rm.record_offset, f.kode
             FROM funn f
             JOIN record_metadata rm ON rm.id = f.record_metadata_id
             JOIN topic t ON t.id = rm.topic_id
             ORDER BY 1, 2",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap()
    }
}
