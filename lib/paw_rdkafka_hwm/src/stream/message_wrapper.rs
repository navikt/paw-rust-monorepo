use std::fmt::{Debug, Display};

use rdkafka::message::OwnedMessage;

pub struct MessageWrapper {
    pub message: OwnedMessage,
    pub timestamp_info: TimestampInfo,
}

pub enum TimestampInfo {
    /// The message has no timestamp.
    None,
    /// The message has a timestamp, but it is not a valid Unix timestamp.
    InSequence,
    /// The message has a valid Unix timestamp.
    OutOfSequence { delta: i64 },
}

impl Display for TimestampInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimestampInfo::None => write!(f, "none"),
            TimestampInfo::InSequence => write!(f, "in_sequence"),
            TimestampInfo::OutOfSequence { delta: _delta } => write!(f, "out_of_sequence"),
        }
    }
}

impl Debug for TimestampInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimestampInfo::None => write!(f, "none"),
            TimestampInfo::InSequence => write!(f, "in_sequence"),
            TimestampInfo::OutOfSequence { delta } => {
                write!(f, "out_of_sequence (delta: {})", delta)
            }
        }
    }
}
