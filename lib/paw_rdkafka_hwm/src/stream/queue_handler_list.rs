use rdkafka::{Message, message::OwnedMessage};

use crate::{
    rebalance::topic_partition_update::TopicPartition,
    stream::{
        paw_kafka_stream::StreamError,
        queue_handler::{PartitionMessageSource, QueueHandler},
    },
};

pub fn ensure_queue_and_push<S, F>(
    queue_handlers: &mut Vec<QueueHandler<S>>,
    message_or_key: MessageOrKey,
    builder: F,
) -> Result<(), StreamError>
where
    S: PartitionMessageSource,
    F: FnOnce(TopicPartition) -> Option<QueueHandler<S>>,
{
    let (key, message) = message_or_key.into_parts();
    let index = queue_handlers
        .iter()
        .position(|q| q.key == key)
        .or_else(|| {
            let new_queue = builder(key.clone());
            if let Some(new_queue) = new_queue {
                queue_handlers.push(new_queue);
                tracing::debug!("Created new QueueHandler for key: {:?}", key);
                Some(queue_handlers.len() - 1)
            } else {
                tracing::debug!("No QueueHandler created for key: {:?}", key);
                None
            }
        });
    match (index, message) {
        (Some(idx), Some(msg)) => {
            queue_handlers[idx].add_message(msg)?;
        }
        (None, Some(msg)) => {
            tracing::warn!(
                "No QueueHandler found or created for key: {:?}, message  with offset {} dropped",
                key,
                msg.offset()
            );
        }
        _ => {}
    }
    Ok(())
}

pub enum MessageOrKey {
    Message(OwnedMessage),
    Key(TopicPartition),
}

impl MessageOrKey {
    pub fn into_parts(self) -> (TopicPartition, Option<OwnedMessage>) {
        match self {
            MessageOrKey::Message(msg) => (
                TopicPartition {
                    topic: msg.topic().to_string(),
                    partition: msg.partition(),
                },
                Option::Some(msg),
            ),
            MessageOrKey::Key(key) => (key, None),
        }
    }
}

/// Pushes the message onto the queue for its topic partition.
/// Returns false if the partition is not assigned to this consumer,
/// in which case the message is dropped.
pub fn push_if_assigned<S: PartitionMessageSource>(
    queue_handlers: &mut [QueueHandler<S>],
    message: OwnedMessage,
) -> Result<bool, StreamError> {
    let key = TopicPartition {
        topic: message.topic().to_string(),
        partition: message.partition(),
    };
    let index = queue_handlers.iter().position(|q| q.key == key);
    if let Some(idx) = index {
        queue_handlers[idx].add_message(message)?;
        Ok(true)
    } else {
        tracing::trace!(
            "No QueueHandler found for key: {:?}, its not assigned to this consumer, message with offset {} dropped",
            key,
            message.offset()
        );
        Ok(false)
    }
}
