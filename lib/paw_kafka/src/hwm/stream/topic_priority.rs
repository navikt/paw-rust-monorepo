use paw_rust_base::env::RuntimeEnv;

use crate::topics::Topic;

pub struct TopicPriority {
    pub topic: String,
    pub priority: i64,
}

pub struct TopicPriorityList {
    list: Vec<TopicPriority>,
    default_priority: i64,
}

impl TopicPriorityList {
    pub fn new(runtime: &RuntimeEnv, topics: Vec<(&Topic, i64)>) -> Self {
        TopicPriorityList {
            list: topics
                .into_iter()
                .map(|(topic, priority)| {
                    (
                        crate::topics::get_topic(runtime, topic).to_string(),
                        priority,
                    )
                })
                .map(|(topic, priority)| TopicPriority { topic, priority })
                .collect(),
            default_priority: 100,
        }
    }

    pub fn new_from_strings(topics: Vec<(String, i64)>) -> Self {
        TopicPriorityList {
            list: topics
                .into_iter()
                .map(|(topic, priority)| TopicPriority { topic, priority })
                .collect(),
            default_priority: 100,
        }
    }

    pub fn empty() -> Self {
        TopicPriorityList {
            list: Vec::new(),
            default_priority: 100,
        }
    }

    pub fn get_priority(&self, topic: &str) -> i64 {
        for tp in &self.list {
            if tp.topic == topic {
                return tp.priority;
            }
        }
        self.default_priority
    }
}
