use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};

use prometheus::{IntCounterVec, register_int_counter_vec};
use tokio::sync::mpsc;

use crate::transport::DirectTeamLogs;
use crate::{LogLevel, TeamLogger, TeamLogsError};

static QUEUE_DROPS: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        "paw_team_logs_queue_drops_total",
        "Team Logs entries not enqueued due to full or closed queue",
        &["level"]
    )
    .expect("Failed to register Team Logs queue metric")
});

/// A bounded nonblocking Team Logs queue with a background TCP sender.
///
/// The first entry is an INFO startup message containing endpoint, Nais metadata
/// and queue capacity. Seeing it in Team Logs confirms that the route works;
/// constructing this client only confirms that the entry was queued.
/// Dropping the client or stopping the runtime can lose queued entries. This is
/// an observational log, not a durable audit journal.
#[derive(Clone)]
pub struct BufferedTeamLogs {
    sender: mpsc::Sender<QueuedEntry>,
    _worker: Arc<tokio::task::JoinHandle<()>>,
    accepting: Arc<AtomicBool>,
}

struct QueuedEntry {
    level: LogLevel,
    logger_name: String,
    message: String,
}

impl BufferedTeamLogs {
    /// Creates the client from Nais-provided environment variables.
    pub fn from_nais_env(capacity: usize) -> Result<Self, TeamLogsError> {
        Self::start(DirectTeamLogs::from_nais_env()?, capacity)
    }

    /// Uses explicit, trusted metadata and a destination (for local testing).
    pub fn new(
        address: impl Into<String>,
        project: impl Into<String>,
        namespace: impl Into<String>,
        pod: impl Into<String>,
        app: impl Into<String>,
        capacity: usize,
    ) -> Result<Self, TeamLogsError> {
        Self::start(
            DirectTeamLogs::new(address, project, namespace, pod, app),
            capacity,
        )
    }

    fn start(transport: DirectTeamLogs, capacity: usize) -> Result<Self, TeamLogsError> {
        if capacity == 0 {
            return Err(TeamLogsError::QueueUnavailable);
        }
        let (sender, mut receiver) = mpsc::channel::<QueuedEntry>(capacity);
        sender
            .try_send(QueuedEntry {
                level: LogLevel::Info,
                logger_name: "team-logs-logger".to_owned(),
                message: transport.startup_message(capacity),
            })
            .map_err(|_| TeamLogsError::QueueUnavailable)?;

        let accepting = Arc::new(AtomicBool::new(true));
        let worker_accepting = accepting.clone();
        let worker = tokio::spawn(async move {
            while let Some(entry) = receiver.recv().await {
                let _ = transport
                    .send(entry.level, &entry.logger_name, &entry.message)
                    .await;
            }
            worker_accepting.store(false, Ordering::Release);
        });
        Ok(Self {
            sender,
            _worker: Arc::new(worker),
            accepting,
        })
    }
}

impl TeamLogger for BufferedTeamLogs {
    fn log(&self, level: LogLevel, logger_name: &str, message: &str) -> Result<(), TeamLogsError> {
        if !self.accepting.load(Ordering::Acquire) {
            QUEUE_DROPS.with_label_values(&[level.as_str()]).inc();
            return Err(TeamLogsError::QueueUnavailable);
        }
        self.sender
            .try_send(QueuedEntry {
                level,
                logger_name: logger_name.to_owned(),
                message: message.to_owned(),
            })
            .map_err(|_| {
                QUEUE_DROPS.with_label_values(&[level.as_str()]).inc();
                TeamLogsError::QueueUnavailable
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rejects_zero_capacity() {
        assert!(matches!(
            BufferedTeamLogs::new("127.0.0.1:1", "dev-gcp", "paw", "pod-1", "app-1", 0),
            Err(TeamLogsError::QueueUnavailable)
        ));
    }

    #[tokio::test]
    async fn startup_entry_is_queued_before_application_logs() {
        let logger =
            BufferedTeamLogs::new("127.0.0.1:1", "dev-gcp", "paw", "pod-1", "app-1", 1).unwrap();
        // The worker cannot run until the current task yields, so the startup entry
        // occupies the sole slot. Nothing is sent to a server in this test.
        assert!(matches!(
            logger.warn("example", "later"),
            Err(TeamLogsError::QueueUnavailable)
        ));
    }
}
