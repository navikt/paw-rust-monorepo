use rdkafka::Offset;

pub const DEFAULT_HWM_OFFSET: i64 = -1;

#[derive(Debug, Clone)]
pub struct Hwm {
    pub topic: String,
    partition: u16,
    pub offset: Option<i64>,
}

impl Hwm {
    pub fn new(topic: impl Into<String>, partition: u16, offset: Option<i64>) -> Self {
        Self {
            topic: topic.into(),
            partition,
            offset,
        }
    }
    
    pub fn partition(&self) -> i32 {
        self.partition as i32
    }

    pub fn neste_offset(&self) -> Offset {
        match self.offset {
            Some(offset) => Offset::Offset(offset + 1),
            None => Offset::Beginning,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_offset_gir_beginning() {
        let hwm = Hwm::new("test", 0, None);
        assert_eq!(hwm.neste_offset(), Offset::Beginning);
    }

    #[test]
    fn default_hwm_gir_offset_0() {
        let hwm = Hwm::new("test", 0, Some(DEFAULT_HWM_OFFSET));
        assert_eq!(hwm.neste_offset(), Offset::Offset(0));
    }

    #[test]
    fn offset_0_gir_offset_1() {
        let hwm = Hwm::new("test", 0, Some(0));
        assert_eq!(hwm.neste_offset(), Offset::Offset(1));
    }
}
