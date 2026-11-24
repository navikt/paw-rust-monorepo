use std::{error::Error, sync::Arc};

use paw_kafka::hwm::rebalance::topic_partition_update::TopicPartitionUpdate;
use paw_kafka::hwm::stream::assigned_partitions::AssignedPartitions;
use paw_kafka::hwm::{
    kafka_connection::create_kafka_consumer_with_sender,
    stream::{stream_config::PawKafkaStreamConfig, stream_wrapper::PawKafkaConsumerStream},
};
use paw_rust_base::await_signal::await_signal;
use tokio::sync::mpsc;
use tracing::info;

use crate::core::app_context::AppContext;
use crate::core::consumer_workers::spawn_consumer_workers;
use crate::features::innlesing::message_processor::InternkontrollMessageProcessor;
use crate::features::innlesing::metadata::meldinger::Dekodere;
use crate::features::innlesing::schema_registry::create_schema_registry_settings;
use crate::features::innlesing::topic::{load_topic_ids, register_topics};
use crate::features::innlesing::topic_catalog::subscribed_topics;
use crate::features::regler::kjoring::spawn_regelkjoring;
use crate::features::regler::regel::{alle_regler, registrer_regler};
use crate::features::visualisering::{self, VisState};

pub async fn run_app(app_context: AppContext) -> Result<(), Box<dyn Error>> {
    let vis_state = VisState {
        pg_pool: app_context.pg_pool.clone(),
        hwm_version: *app_context.kafka_config.hwm_version,
    };
    let http_routes = paw_observability::server::routes(app_context.app_state.clone()).merge(
        visualisering::routes(vis_state, app_context.auth_state.clone()),
    );
    let http_server_task = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
        axum::serve(listener, http_routes).await
    });
    let subscribed_topics = subscribed_topics(app_context.runtime_env.clone());
    let topics = subscribed_topics.topic_names();
    let mut conn = app_context.pg_pool.acquire().await?;
    register_topics(&mut conn, &topics).await?;
    let topic_ids = load_topic_ids(&mut conn).await?;
    drop(conn);
    let (topic_update_sender, topic_update_receiver) =
        mpsc::unbounded_channel::<TopicPartitionUpdate>();
    let signing_required_from = subscribed_topics.signing_required_catalog();
    let dekodere = Dekodere::new(
        subscribed_topics.topic_names_mapping(),
        create_schema_registry_settings(&app_context.runtime_env)?,
    );
    let topic_priorities = subscribed_topics.get_topic_priority_list();
    let stream_config =
        PawKafkaStreamConfig::from_kafka_config(&app_context.kafka_config, topic_priorities)?;
    let consumer = create_kafka_consumer_with_sender(
        app_context.app_state.clone(),
        app_context.pg_pool.clone(),
        &app_context.kafka_config,
        &topics.iter().map(|s| s.as_str()).collect::<Vec<&str>>(),
        topic_update_sender,
    )?;
    let assigned_partitions = AssignedPartitions::default();
    let stream = PawKafkaConsumerStream::new(
        topic_update_receiver,
        consumer,
        app_context.pg_pool.clone(),
        stream_config,
        assigned_partitions.clone(),
    );

    let regler = registrer_regler(alle_regler(), &topic_ids, &app_context.runtime_env);
    let message_processor = Arc::new(InternkontrollMessageProcessor::new(
        signing_required_from,
        app_context.record_verifier,
        topic_ids,
        dekodere,
        subscribed_topics,
        app_context.signing_key_db_ids,
    ));

    let kafka_task = spawn_consumer_workers(
        stream,
        message_processor,
        app_context.pg_pool.clone(),
        *app_context.kafka_config.hwm_version,
        app_context.app_state.clone(),
    );
    let regel_task = spawn_regelkjoring(
        app_context.pg_pool.clone(),
        regler,
        assigned_partitions,
        app_context.regel_config,
        app_context.app_state.clone(),
    );
    let signal = await_signal();
    app_context.app_state.set_has_started(true);
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
        result = regel_task => {
            match result {
                Ok(Ok(())) => info!("Regelkjøring stoppet."),
                Ok(Err(e)) => return Err(e.into()),
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
    app_context.app_state.set_is_alive(false);
    let _ = app_context.pg_pool.close().await;
    info!("Pg pool lukket");
    Ok(())
}
