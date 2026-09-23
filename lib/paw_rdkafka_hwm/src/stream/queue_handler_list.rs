use rdkafka::{Message, message::OwnedMessage};

use crate::{rebalance::rebalance_message::TopicPartition, stream::queue_handler::QueueHandler};

pub fn ensure_queue_and_push<F>(
    queue_handlers: &mut Vec<QueueHandler>,
    message_or_key: MessageOrKey,
    builder: F,
) where
    F: FnOnce(TopicPartition) -> Option<QueueHandler>,
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
            queue_handlers[idx]
                .add_message(msg)
                .expect("Failed to add message to QueueHandler, direct key access should not fail");
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
pub fn push_if_assigned(queue_handlers: &mut [QueueHandler], message: OwnedMessage) -> bool {
    let key = TopicPartition {
        topic: message.topic().to_string(),
        partition: message.partition(),
    };
    let index = queue_handlers.iter().position(|q| q.key == key);
    if let Some(idx) = index {
        queue_handlers[idx]
            .add_message(message)
            .expect("Failed to add message to QueueHandler, direct key access should not fail");
        true
    } else {
        tracing::trace!(
            "No QueueHandler found for key: {:?}, its not assigned to this consumer, message with offset {} dropped",
            key,
            message.offset()
        );
        false
    }
}
