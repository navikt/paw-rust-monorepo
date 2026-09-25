use std::fmt::{Debug, Display};

use rdkafka::{Message, message::OwnedMessage};

pub struct MessageWrapper {
    pub message: OwnedMessage,
    pub timestamp_info: TimestampInfo,
}

impl MessageWrapper {
    pub fn new(message: OwnedMessage, delta: Option<i64>) -> Self {
        match delta {
            Some(delta) if delta >= 0 => Self {
                message,
                timestamp_info: TimestampInfo::InSequence { delta },
            },
            Some(delta) if delta < 0 => Self {
                message,
                timestamp_info: TimestampInfo::OutOfSequence { delta },
            },
            _ => Self {
                message,
                timestamp_info: TimestampInfo::None,
            },
        }
    }

    pub fn is_in_sequence(&self) -> bool {
        matches!(self.timestamp_info, TimestampInfo::InSequence { .. })
    }

    pub fn is_out_of_sequence(&self) -> bool {
        matches!(self.timestamp_info, TimestampInfo::OutOfSequence { .. })
    }

    pub fn timestamp(&self) -> Option<i64> {
        self.message.timestamp().to_millis()
    }
}

pub enum TimestampInfo {
    None,
    InSequence { delta: i64 },
    OutOfSequence { delta: i64 },
}

impl Display for TimestampInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimestampInfo::None => write!(f, "none"),
            TimestampInfo::InSequence { delta: _delta } => write!(f, "in_sequence"),
            TimestampInfo::OutOfSequence { delta: _delta } => write!(f, "out_of_sequence"),
        }
    }
}

impl Debug for TimestampInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimestampInfo::None => write!(f, "none"),
            TimestampInfo::InSequence { delta } => write!(f, "in_sequence ( delta: {}", delta),
            TimestampInfo::OutOfSequence { delta } => {
                write!(f, "out_of_sequence (delta: {})", delta)
            }
        }
    }
}
