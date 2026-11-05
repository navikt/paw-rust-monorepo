use std::time::Duration;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RegelConfig {
    /// Maks antall meldinger per regel og partisjon i én runde.
    pub batch_size: i64,
    /// Pause etter en runde uten noe å gjøre.
    pub idle_sleep_secs: u64,
}

impl RegelConfig {
    pub fn idle_sleep(&self) -> Duration {
        Duration::from_secs(self.idle_sleep_secs)
    }
}
