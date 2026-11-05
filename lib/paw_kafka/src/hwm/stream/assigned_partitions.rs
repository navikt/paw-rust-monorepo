use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::hwm::rebalance::topic_partition_update::TopicPartition;

/// Topic-partisjonene som er tildelt denne consumeren. Kan klones og deles
/// med andre tasker; alle kloner ser samme tilstand.
///
/// `add` og `remove` tar låsen én gang per kall og gjør hele endringen før
/// låsen slippes. Kall dem derfor én gang per rebalanse-hendelse, ikke per
/// partisjon, så en leser aldri ser en halvveis oppdatert tilstand.
#[derive(Clone, Default)]
pub struct AssignedPartitions {
    inner: Arc<Mutex<HashSet<TopicPartition>>>,
}

impl AssignedPartitions {
    pub fn snapshot(&self) -> Vec<TopicPartition> {
        self.lock().iter().cloned().collect()
    }

    /// Unike partisjonsnumre på tvers av topics, sortert stigende.
    pub fn partitions(&self) -> Vec<i32> {
        let mut partitions: Vec<i32> = self.lock().iter().map(|tp| tp.partition).collect();
        partitions.sort_unstable();
        partitions.dedup();
        partitions
    }

    pub fn add(&self, topic_partitions: impl IntoIterator<Item = TopicPartition>) {
        self.lock().extend(topic_partitions);
    }

    pub fn remove(&self, topic_partitions: &[TopicPartition]) {
        let mut set = self.lock();
        for tp in topic_partitions {
            set.remove(tp);
        }
    }

    fn lock(&self) -> MutexGuard<'_, HashSet<TopicPartition>> {
        // En panikk i en annen holder kan ikke etterlate settet ugyldig, så vi ser bort fra poisoning.
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}
