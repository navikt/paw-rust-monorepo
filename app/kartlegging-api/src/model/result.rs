#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessorResult {
    Continue,
    Pause { synced_topics: Vec<String> },
}
