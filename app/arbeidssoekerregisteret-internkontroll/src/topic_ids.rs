use std::collections::HashMap;
use std::sync::Arc;

/// Topic name to `topic.id`. The IDs never change, so the mapping is read once
/// at startup. Cloning is cheap: clones share the same map.
#[derive(Debug, Clone)]
pub struct TopicIds {
    ids: Arc<HashMap<String, i16>>,
}

impl TopicIds {
    pub fn new(pairs: impl IntoIterator<Item = (impl Into<String>, i16)>) -> Self {
        let ids = pairs
            .into_iter()
            .map(|(name, id)| (name.into(), id))
            .collect();
        Self { ids: Arc::new(ids) }
    }

    pub fn get(&self, topic: &str) -> Option<i16> {
        self.ids.get(topic).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slaar_opp_kjente_topics() {
        let topic_ids = TopicIds::new([("topic-a", 1), ("topic-b", 2)]);
        assert_eq!(topic_ids.get("topic-a"), Some(1));
        assert_eq!(topic_ids.get("topic-b"), Some(2));
        assert_eq!(topic_ids.get("topic-c"), None);
    }
}
