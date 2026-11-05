use std::{error::Error, sync::Arc};

use paw_kafka::hwm::{
    hwm_message_processor::{ProcessorError, hwm_process_message},
    rebalance::hwm_rebalance_handler::HwmRebalanceHandler,
    stream::{paw_kafka_stream::PawKafkaStream, stream_wrapper::PawKafkaConsumerStream},
};
use paw_observability::health::simple_app_state::AppState;
use rdkafka::{Message, consumer::StreamConsumer, message::OwnedMessage};
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::features::innlesing::message_processor::InternkontrollMessageProcessor;

pub fn spawn_consumer_workers(
    stream: PawKafkaConsumerStream<StreamConsumer<HwmRebalanceHandler>>,
    message_processor: Arc<InternkontrollMessageProcessor>,
    pg_pool: PgPool,
    hwm_version: i16,
    app_state: Arc<AppState>,
) -> tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    let number_of_workers = 6;
    let mut senders: Vec<mpsc::Sender<OwnedMessage>> = Vec::with_capacity(number_of_workers);
    let mut workers: Vec<tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>>> =
        Vec::with_capacity(number_of_workers);
    for _ in 0..number_of_workers {
        let (sender, receiver) = mpsc::channel::<OwnedMessage>(10);
        senders.push(sender);
        workers.push(spawn_partition_worker(
            hwm_version,
            receiver,
            pg_pool.clone(),
            message_processor.clone(),
        ));
    }
    tokio::spawn({
        async move {
            let mut paw_stream = stream;
            while app_state.is_alive() {
                let (next_stream, msg) = paw_stream.receive().await?;
                paw_stream = next_stream;
                if let Some(msg) = msg {
                    let partition = msg.partition();
                    senders[partition as usize].send(msg).await?
                }
            }
            Ok::<(), ProcessorError>(())
        }
    })
}

fn spawn_partition_worker(
    hwm_version: i16,
    receiver: mpsc::Receiver<OwnedMessage>,
    pg_pool: PgPool,
    processor: Arc<InternkontrollMessageProcessor>,
) -> tokio::task::JoinHandle<Result<(), Box<dyn Error + Send + Sync>>> {
    tokio::spawn(async move {
        let mut receiver = receiver;
        let msg_processor = processor.as_ref();
        while let Some(msg) = receiver.recv().await {
            hwm_process_message(hwm_version, pg_pool.clone(), &msg, msg_processor).await?
        }
        Ok(())
    })
}
