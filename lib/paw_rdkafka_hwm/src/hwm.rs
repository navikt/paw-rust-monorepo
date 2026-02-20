use rdkafka::Offset;

pub const DEFAULT_HWM: i64 = -1;

#[derive(Debug, Clone)]
pub struct Hwm {
    pub topic: String,
    pub partition: i32,
    pub offset: Option<i64>,
}

impl Hwm {
    pub fn seek_to_rd_kafka_offset(&self) -> Offset {
        match self.offset {
            None => Offset::Beginning,
            Some(offset) => Offset::Offset(offset + 1),
        }
    }
}

pub struct TopicPartition {
    pub topic: String,
    pub partition: i32,
}
