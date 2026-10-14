mod database;
mod message_processor;
mod signing_required;
mod topic_ids;
mod trace_operation_type;

use std::sync::Arc;

use message_processor::InternkontrollMessageProcessor;
use paw_kafka::hwm::kafka_connection::create_kafka_consumer_with_sender;
use paw_kafka::hwm::stream::stream_config::PawKafkaStreamConfig;
use paw_kafka::hwm::stream::topic_priority::TopicPriorityList;
use paw_kafka::hwm::{
    hwm_message_processor::{ProcessorError, hwm_process_message},
    rebalance::topic_partition_update::TopicPartitionUpdate,
    stream::{
        paw_kafka_stream::PawKafkaStream,
        stream_wrapper::{PawKafkaConsumerStream, init_stream_wrapper_metrics},
    },
};
use paw_kafka::kafka_config::KafkaConfig;
use paw_kafka::signing::RecordVerifier;
use paw_kafka::topics::{Topic, get_topic_names};
use paw_observability::server::spawn_health_server;
use paw_observability::{health::simple_app_state, otel::setup_nais_otel};
use paw_rust_base::{
    await_signal::await_signal, env::runtime_env, panic_logger::register_panic_logger,
};
use paw_rust_base::{config::read_toml_config, read_config_file};
use paw_sqlx::config::DatabaseConfig;
use paw_team_logs::{BufferedTeamLogs, TeamLogger};
use sqlx::{Connection, PgConnection, PgPool};
use std::error::Error;
use tokio::sync::mpsc;
use tracing::info;

const RESET_SQL: &str = include_str!("../reset.sql");

async fn reset_db_if_needed(pool: &PgPool, configured_hwm_version: i16) -> Result<bool, sqlx::Error> {
    let mut connection = pool.acquire().await?;
    reset_db_on_connection(&mut connection, configured_hwm_version).await
}

async fn reset_db_on_connection(
    connection: &mut PgConnection,
    configured_hwm_version: i16,
) -> Result<bool, sqlx::Error> {
    let hwm_exists: bool = sqlx::query_scalar("SELECT to_regclass('public.hwm') IS NOT NULL")
        .fetch_one(&mut *connection)
        .await?;

    if !hwm_exists {
        info!("HWM-tabellen mangler, hopper over nullstilling av DB");
        return Ok(false);
    }

    let max_hwm_version: Option<i16> = sqlx::query_scalar("SELECT MAX(version) FROM public.hwm")
        .fetch_one(&mut *connection)
        .await?;

    match max_hwm_version {
        Some(version) if configured_hwm_version <= version => {
            info!(
                configured_hwm_version,
                stored_hwm_version = version,
                "Konfigurert HWM-versjon er ikke nyere enn lagret versjon, hopper over nullstilling av DB"
            );
            return Ok(false);
        }
        Some(version) => {
            info!(
                configured_hwm_version,
                stored_hwm_version = version,
                "Konfigurert HWM-versjon er nyere enn lagret versjon, nullstiller DB"
            );
        }
        None => {
            info!(
                configured_hwm_version,
                "HWM-tabellen er tom, nullstiller DB"
            );
        }
    }
    let mut transaction = connection.begin().await?;
    sqlx::raw_sql(RESET_SQL).execute(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(true)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    register_panic_logger();
    setup_nais_otel().unwrap();
    init_stream_wrapper_metrics();
    let team_logger = BufferedTeamLogs::from_nais_env(100)?;
    let record_verifier = RecordVerifier::from_embedded_keys()?;
    run_app(team_logger, record_verifier).await
}

async fn run_app(
    team_logger: impl TeamLogger + Clone + 'static,
    record_verifier: RecordVerifier,
) -> Result<(), Box<dyn Error>> {
    let kafka_config: KafkaConfig = read_toml_config(read_config_file!("kafka_config.toml"))?;
    let database_config: DatabaseConfig =
        read_toml_config(read_config_file!("database_config.toml"))?;
    let app_state = Arc::new(simple_app_state::AppState::new());
    let http_server_task = spawn_health_server(app_state.clone());
    let hwm_version = *kafka_config.hwm_version;
    let runtime_env = runtime_env();
    let topics = get_topic_names(
        &runtime_env,
        &[
            Topic::Periode,
            Topic::Opplysninger,
            Topic::Profilering,
            Topic::PaaVegneAv,
            Topic::Bekreftelse,
            Topic::Hendelselogg,
            Topic::BekreftelseHendelseLogg,
            Topic::Egenvurdering,
        ],
    );
    let pg_pool = paw_sqlx::postgres::init_db(database_config).await?;
    reset_db_if_needed(&pg_pool, hwm_version).await?;
    sqlx::migrate!("./migrations").run(&pg_pool).await?;
    database::topic::register_topics(&pg_pool, &topics).await?;
    let (topic_update_sender, topic_update_receiver) =
        mpsc::unbounded_channel::<TopicPartitionUpdate>();
    let topic_priorities = TopicPriorityList::new(
        &runtime_env,
        vec![
            (&Topic::BekreftelseHendelseLogg, 0),
            (&Topic::Hendelselogg, 10),
            (&Topic::Periode, 50),
            (&Topic::Opplysninger, 60),
            (&Topic::Profilering, 70),
        ],
    );
    let stream_config = PawKafkaStreamConfig::from_kafka_config(&kafka_config, topic_priorities)?;
    let consumer = create_kafka_consumer_with_sender(
        app_state.clone(),
        pg_pool.clone(),
        kafka_config,
        &topics,
        topic_update_sender,
    )?;
    let stream = PawKafkaConsumerStream::new(
        topic_update_receiver,
        consumer,
        pg_pool.clone(),
        stream_config,
    );
    let kafka_task = tokio::spawn({
        let state = app_state.clone();
        let pg_pool = pg_pool.clone();
        async move {
            let message_processor =
                InternkontrollMessageProcessor::new(record_verifier, team_logger);
            let mut paw_stream = stream;
            while state.is_alive() {
                let (next_stream, msg) = paw_stream.receive().await?;
                paw_stream = next_stream;
                if let Some(msg) = msg {
                    hwm_process_message(hwm_version, pg_pool.clone(), &msg, &message_processor)
                        .await?
                }
            }
            Ok::<(), ProcessorError>(())
        }
    });
    let signal = await_signal();
    app_state.set_has_started(true);
    info!("Alle tjenester startet, applikasjon kjører");
    tokio::select! {
        result = http_server_task => {
            match result {
                Ok(Ok(())) => info!("HTTP server stoppet."),
                Ok(Err(e)) => return Err(e.into()),
                Err(join_error) => return Err(Box::new(join_error)),
            }
        }
        result = kafka_task => {
            match result {
                Ok(Ok(())) => info!("Kafka consumer stoppet."),
                Ok(Err(e)) => return Err(e),
                Err(join_error) => return Err(Box::new(join_error)),
            }
        }
        result = signal => {
            match result {
                Ok(signal) => info!("Signal '{}' mottatt, avslutter....", signal),
                Err(e) => return Err(e),
            }
        }
    }
    app_state.set_is_alive(false);
    let _ = pg_pool.close().await;
    info!("Pg pool lukket");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use postgres_testcontainer::postgres::setup_postgres_container;
    use tokio::sync::Mutex;

    static TEST_LOCK: Mutex<()> = Mutex::const_new(());

    async fn clean_public_schema(connection: &mut PgConnection) {
        let hwm_exists: bool = sqlx::query_scalar("SELECT to_regclass('public.hwm') IS NOT NULL")
            .fetch_one(&mut *connection)
            .await
            .unwrap();
        if hwm_exists {
            assert!(reset_db_on_connection(connection, i16::MAX).await.unwrap());
        }
    }

    async fn migrate(connection: &mut PgConnection) {
        sqlx::migrate!("./migrations")
            .run(&mut *connection)
            .await
            .unwrap();
    }

    async fn insert_test_rows(connection: &mut PgConnection, version: i16) {
        sqlx::query("INSERT INTO public.topic (name) VALUES ('reset-test-topic')")
            .execute(&mut *connection)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO public.hwm (version, topic, partition, hwm) VALUES ($1, 'reset-test-topic', 0, 42)",
        )
        .bind(version)
        .execute(&mut *connection)
        .await
        .unwrap();
    }

    async fn setup_public_connection() -> (
        postgres_testcontainer::postgres::TestContainerGuard,
        sqlx::pool::PoolConnection<sqlx::Postgres>,
    ) {
        let guard = setup_postgres_container().await.unwrap();
        let mut connection = guard.pg_pool.acquire().await.unwrap();
        sqlx::query("SET search_path TO public")
            .execute(&mut *connection)
            .await
            .unwrap();
        clean_public_schema(&mut connection).await;
        (guard, connection)
    }

    #[tokio::test]
    async fn manglende_hwm_tabell_nullstilles_ikke() {
        let _lock = TEST_LOCK.lock().await;
        let (_guard, mut connection) = setup_public_connection().await;

        assert!(!reset_db_on_connection(&mut connection, 20).await.unwrap());
        let hwm_exists: bool = sqlx::query_scalar("SELECT to_regclass('public.hwm') IS NOT NULL")
            .fetch_one(&mut *connection)
            .await
            .unwrap();
        assert!(!hwm_exists);
    }

    #[tokio::test]
    async fn tom_hwm_tabell_nullstilles_og_migrerer_paa_nytt() {
        let _lock = TEST_LOCK.lock().await;
        let (_guard, mut connection) = setup_public_connection().await;
        migrate(&mut connection).await;

        assert!(reset_db_on_connection(&mut connection, 20).await.unwrap());
        let hwm_exists: bool = sqlx::query_scalar("SELECT to_regclass('public.hwm') IS NOT NULL")
            .fetch_one(&mut *connection)
            .await
            .unwrap();
        assert!(!hwm_exists);

        migrate(&mut connection).await;
        let hwm_exists: bool = sqlx::query_scalar("SELECT to_regclass('public.hwm') IS NOT NULL")
            .fetch_one(&mut *connection)
            .await
            .unwrap();
        assert!(hwm_exists);
    }

    #[tokio::test]
    async fn nyere_konfigurert_hwm_versjon_nullstiller_appdata() {
        let _lock = TEST_LOCK.lock().await;
        let (_guard, mut connection) = setup_public_connection().await;
        migrate(&mut connection).await;
        insert_test_rows(&mut connection, 10).await;

        assert!(reset_db_on_connection(&mut connection, 11).await.unwrap());
        let topic_exists: bool =
            sqlx::query_scalar("SELECT to_regclass('public.topic') IS NOT NULL")
                .fetch_one(&mut *connection)
                .await
                .unwrap();
        assert!(!topic_exists);

        migrate(&mut connection).await;
        let topic_exists: bool =
            sqlx::query_scalar("SELECT to_regclass('public.topic') IS NOT NULL")
                .fetch_one(&mut *connection)
                .await
                .unwrap();
        let hwm_exists: bool = sqlx::query_scalar("SELECT to_regclass('public.hwm') IS NOT NULL")
            .fetch_one(&mut *connection)
            .await
            .unwrap();
        assert!(topic_exists);
        assert!(hwm_exists);

        let topic_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM public.topic")
            .fetch_one(&mut *connection)
            .await
            .unwrap();
        let hwm_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM public.hwm")
            .fetch_one(&mut *connection)
            .await
            .unwrap();
        assert_eq!(topic_count, 0);
        assert_eq!(hwm_count, 0);
    }

    #[tokio::test]
    async fn samme_eller_lavere_hwm_versjon_beholder_appdata() {
        let _lock = TEST_LOCK.lock().await;
        let (_guard, mut connection) = setup_public_connection().await;
        migrate(&mut connection).await;
        insert_test_rows(&mut connection, 10).await;

        assert!(!reset_db_on_connection(&mut connection, 9).await.unwrap());
        assert!(!reset_db_on_connection(&mut connection, 10).await.unwrap());
        let topic_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM public.topic")
            .fetch_one(&mut *connection)
            .await
            .unwrap();
        assert_eq!(topic_count, 1);
        let hwm_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM public.hwm")
            .fetch_one(&mut *connection)
            .await
            .unwrap();
        assert_eq!(hwm_count, 1);
    }
}
