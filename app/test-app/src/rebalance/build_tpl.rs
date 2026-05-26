use crate::rebalance::rebalance_error::RebalanceError;
use paw_rdkafka_hwm::hwm::Hwm;
use rdkafka::Offset;
use rdkafka::topic_partition_list::TopicPartitionList;

pub(super) fn build_tpl_from(hwms: &[Hwm]) -> Result<TopicPartitionList, RebalanceError> {
    let mut tpl = TopicPartitionList::new();
    for hwm in hwms {
        let offset: Offset = match hwm.offset {
            None => Offset::Beginning,
            Some(offset) => Offset::Offset(offset + 1)
        };
        tpl.add_partition_offset(&hwm.topic, hwm.partition, offset)?;
    }
    Ok(tpl)
}

#[cfg(test)]
mod tests {
    use paw_rdkafka_hwm::hwm::DEFAULT_HWM;
    use super::*;

    #[test]
    fn tom_liste_gir_tom_tpl() {
        let tpl = build_tpl_from(&[]).unwrap();
        assert_eq!(tpl.count(), 0);
    }

    #[test]
    fn default_hwm_gir_offset_0() {
        let tpl = build_tpl_from(&[hwm("topic-a", 0, Some(DEFAULT_HWM))]).unwrap();

        let elem = &tpl.elements()[0];
        assert_eq!(elem.offset(), Offset::Offset(0));
    }

    #[test]
    fn hwm_med_offset_gir_offset_pluss_en() {
        let tpl = build_tpl_from(&[hwm("topic-a", 0, Some(42))]).unwrap();

        let elem = &tpl.elements()[0];
        assert_eq!(elem.topic(), "topic-a");
        assert_eq!(elem.partition(), 0);
        assert_eq!(elem.offset(), Offset::Offset(43));
    }

    #[test]
    fn hwm_uten_offset_gir_beginning() {
        let tpl = build_tpl_from(&[hwm("topic-a", 0, None)]).unwrap();

        let elem = &tpl.elements()[0];
        assert_eq!(elem.offset(), Offset::Beginning);
    }

    #[test]
    fn flere_hwms_med_ulike_offsets() {
        let hwms = vec![
            hwm("topic-a", 0, Some(100)),
            hwm("topic-a", 1, None),
            hwm("topic-b", 0, Some(0)),
        ];

        let tpl = build_tpl_from(&hwms).unwrap();

        assert_eq!(tpl.count(), 3);

        let elements = tpl.elements();
        assert_eq!(elements[0].offset(), Offset::Offset(101));
        assert_eq!(elements[1].offset(), Offset::Beginning);
        assert_eq!(elements[2].offset(), Offset::Offset(1));
    }

    #[test]
    fn offset_null_gir_offset_en() {
        let tpl = build_tpl_from(&[hwm("topic-a", 0, Some(0))]).unwrap();

        let elem = &tpl.elements()[0];
        assert_eq!(elem.offset(), Offset::Offset(1));
    }

    fn hwm(topic: &str, partition: i32, offset: Option<i64>) -> Hwm {
        Hwm {
            topic: topic.to_string(),
            partition,
            offset,
        }
    }
}
