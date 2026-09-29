use crate::{LogLevel, TeamLogger, TeamLogsError};

/// Discards Team Logs entries. Use explicitly when logging must not block the app.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoopTeamLogs;

impl TeamLogger for NoopTeamLogs {
    fn log(
        &self,
        _level: LogLevel,
        _logger_name: &str,
        _message: &str,
    ) -> Result<(), TeamLogsError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_all_levels_without_runtime_or_server() {
        let logger = NoopTeamLogs;
        assert!(logger.info("example", "started").is_ok());
        assert!(logger.warn("example", "warning").is_ok());
        assert!(logger.error("example", "failed").is_ok());
    }
}
