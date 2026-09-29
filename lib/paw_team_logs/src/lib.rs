//! Private Team Logs delivery for Nais. Entries are never copied to stdout or Loki.

mod client;
mod noop;
mod transport;

pub use client::BufferedTeamLogs;
pub use noop::NoopTeamLogs;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TeamLogsError {
    #[error("missing Team Logs environment variable: {0}")]
    MissingEnvironment(&'static str),
    #[error("could not deliver Team Logs entry: {0}")]
    Transport(#[from] std::io::Error),
    #[error("Team Logs delivery timed out")]
    Timeout,
    #[error("could not encode Team Logs entry: {0}")]
    Encode(#[from] serde_json::Error),
    #[error("Team Logs queue is full or closed")]
    QueueUnavailable,
}

/// Levels supported by the private Team Logs channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }

    pub(crate) fn value(self) -> u32 {
        match self {
            Self::Info => 20_000,
            Self::Warn => 30_000,
            Self::Error => 40_000,
        }
    }
}

/// Accepts a private log entry; a successful call does not guarantee delivery.
///
/// Applications can depend on this interface and implement it in memory for tests,
/// without Nais environment variables, a runtime, or a log server.
pub trait TeamLogger: Send + Sync {
    fn log(&self, level: LogLevel, logger_name: &str, message: &str) -> Result<(), TeamLogsError>;

    fn info(&self, logger_name: &str, message: &str) -> Result<(), TeamLogsError> {
        self.log(LogLevel::Info, logger_name, message)
    }

    fn warn(&self, logger_name: &str, message: &str) -> Result<(), TeamLogsError> {
        self.log(LogLevel::Warn, logger_name, message)
    }

    fn error(&self, logger_name: &str, message: &str) -> Result<(), TeamLogsError> {
        self.log(LogLevel::Error, logger_name, message)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct MemoryLogger(Mutex<Vec<(LogLevel, String, String)>>);

    impl TeamLogger for MemoryLogger {
        fn log(
            &self,
            level: LogLevel,
            logger_name: &str,
            message: &str,
        ) -> Result<(), TeamLogsError> {
            self.0
                .lock()
                .unwrap()
                .push((level, logger_name.to_owned(), message.to_owned()));
            Ok(())
        }
    }

    fn function_under_test(logs: &impl TeamLogger) -> Result<(), TeamLogsError> {
        logs.info("example", "started")?;
        logs.warn("example", "invalid input")?;
        logs.error("example", "failed")
    }

    #[test]
    fn application_can_test_logging_without_environment_or_server() {
        let logs = MemoryLogger::default();
        function_under_test(&logs).unwrap();
        assert_eq!(
            *logs.0.lock().unwrap(),
            vec![
                (LogLevel::Info, "example".into(), "started".into()),
                (LogLevel::Warn, "example".into(), "invalid input".into()),
                (LogLevel::Error, "example".into(), "failed".into()),
            ]
        );
    }
}
