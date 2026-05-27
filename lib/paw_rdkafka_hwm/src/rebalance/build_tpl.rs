use crate::hwm::Hwm;
use crate::rebalance::rebalance_error::RebalanceError;
use rdkafka::Offset;
use rdkafka::topic_partition_list::TopicPartitionList;

pub(super) fn build_assignment_tpl_from(hwms: &[Hwm]) -> Result<TopicPartitionList, RebalanceError> {
    let mut tpl = TopicPartitionList::new();
    for hwm in hwms {
        let offset: Offset = match hwm.offset {
            None => Offset::Beginning,
            Some(offset) => Offset::Offset(offset + 1),
        };
        tpl.add_partition_offset(&hwm.topic, hwm.partition, offset)?;
    }
    Ok(tpl)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hwm::DEFAULT_HWM;

    #[test]
    fn tom_liste_gir_tom_tpl() {
        let tpl = build_assignment_tpl_from(&[]).unwrap();
        assert_eq!(tpl.count(), 0);
    }

    #[test]
    fn default_hwm_gir_offset_0() {
        let hwms = vec![hwm("topic-a", 0, Some(DEFAULT_HWM))];
        let tpl = build_assignment_tpl_from(&hwms).unwrap();

        assert_eq!(tpl.count(), 1);
        let elem = &tpl.elements()[0];
        assert_eq!(elem.topic(), "topic-a");
        assert_eq!(elem.partition(), 0);
        assert_eq!(elem.offset(), Offset::Offset(DEFAULT_HWM + 1));
    }

    #[test]
    fn none_offset_gir_beginning() {
        let hwms = vec![hwm("topic-a", 0, None)];
        let tpl = build_assignment_tpl_from(&hwms).unwrap();

        assert_eq!(tpl.elements()[0].offset(), Offset::Beginning);
    }

    #[test]
    fn flere_hwms_gir_korrekte_offsets() {
        let hwms = vec![
            hwm("topic-a", 0, Some(100)),
            hwm("topic-a", 1, None),
            hwm("topic-b", 0, Some(42)),
        ];
        let tpl = build_assignment_tpl_from(&hwms).unwrap();

        assert_eq!(tpl.count(), 3);
        assert_eq!(tpl.elements()[0].offset(), Offset::Offset(101));
        assert_eq!(tpl.elements()[1].offset(), Offset::Beginning);
        assert_eq!(tpl.elements()[2].offset(), Offset::Offset(43));
    }

    #[test]
    fn offset_0_gir_offset_1() {
        let hwms = vec![hwm("topic-a", 0, Some(0))];
        let tpl = build_assignment_tpl_from(&hwms).unwrap();

        assert_eq!(tpl.elements()[0].offset(), Offset::Offset(1));
    }

    #[test]
    fn stor_offset_gir_korrekt_verdi() {
        let hwms = vec![hwm("topic-a", 0, Some(i64::MAX - 1))];
        let tpl = build_assignment_tpl_from(&hwms).unwrap();

        assert_eq!(tpl.elements()[0].offset(), Offset::Offset(i64::MAX));
    }

    fn hwm(topic: &str, partition: i32, offset: Option<i64>) -> Hwm {
        Hwm {
            topic: topic.to_string(),
            partition,
            offset,
        }
    }
}
