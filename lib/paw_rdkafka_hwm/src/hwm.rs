use rdkafka::Offset;

pub const DEFAULT_HWM: i64 = -1;

#[derive(Debug, Clone)]
pub struct Hwm {
    pub topic: String,
    pub partition: i32,
    pub offset: i64,
}

impl Hwm {
    pub fn seek_to_rd_kafka_offset(&self) -> Offset {
        match self.offset {
            DEFAULT_HWM => Offset::Beginning,
            _ => Offset::Offset(self.offset + 1),
        }
    }
}

pub struct TopicPartition {
    pub topic: String,
    pub partition: i32,
}
