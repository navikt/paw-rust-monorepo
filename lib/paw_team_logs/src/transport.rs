use std::env;
use std::sync::LazyLock;
use std::time::Duration;

use prometheus::{IntCounterVec, register_int_counter_vec};
use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::timeout;

use crate::{LogLevel, TeamLogsError};

const ENDPOINT: &str = "team-logs.nais-system:5170";
const SEND_TIMEOUT: Duration = Duration::from_secs(2);

static DELIVERY_FAILURES: LazyLock<IntCounterVec> = LazyLock::new(|| {
    register_int_counter_vec!(
        "paw_team_logs_delivery_failures_total",
        "Failed direct Team Logs sends",
        &["level"]
    )
    .expect("Failed to register Team Logs metric")
});

pub(crate) struct DirectTeamLogs {
    address: String,
    project: String,
    namespace: String,
    pod: String,
    app: String,
    stream: Option<TcpStream>,
}

#[derive(Serialize)]
struct Entry<'a> {
    #[serde(rename = "@timestamp")]
    timestamp: &'a str,
    #[serde(rename = "@version")]
    version: &'static str,
    google_cloud_project: &'a str,
    nais_namespace_name: &'a str,
    nais_pod_name: &'a str,
    nais_container_name: &'a str,
    logger_name: &'a str,
    level: &'a str,
    level_value: u32,
    message: &'a str,
    tags: [&'static str; 1],
    thread_name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    endpoint: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    queue_capacity: Option<usize>,
}

impl DirectTeamLogs {
    pub(crate) fn from_nais_env() -> Result<Self, TeamLogsError> {
        Ok(Self::new(
            ENDPOINT,
            required("GOOGLE_CLOUD_PROJECT")?,
            required("NAIS_NAMESPACE")?,
            required("NAIS_POD_NAME")?,
            required("NAIS_APP_NAME")?,
        ))
    }

    pub(crate) fn new(
        address: impl Into<String>,
        project: impl Into<String>,
        namespace: impl Into<String>,
        pod: impl Into<String>,
        app: impl Into<String>,
    ) -> Self {
        Self {
            address: address.into(),
            project: project.into(),
            namespace: namespace.into(),
            pod: pod.into(),
            app: app.into(),
            stream: None,
        }
    }

    pub(crate) async fn send(
        &mut self,
        level: LogLevel,
        logger_name: &str,
        message: &str,
        timestamp: &str,
        thread_name: &str,
        startup_capacity: Option<usize>,
    ) -> Result<(), TeamLogsError> {
        let result = self
            .send_inner(
                level,
                logger_name,
                message,
                timestamp,
                thread_name,
                startup_capacity,
            )
            .await;
        if result.is_err() {
            DELIVERY_FAILURES.with_label_values(&[level.as_str()]).inc();
        }
        result
    }

    async fn send_inner(
        &mut self,
        level: LogLevel,
        logger_name: &str,
        message: &str,
        timestamp: &str,
        thread_name: &str,
        startup_capacity: Option<usize>,
    ) -> Result<(), TeamLogsError> {
        let bytes = self.encode(
            level,
            logger_name,
            message,
            timestamp,
            thread_name,
            startup_capacity,
        )?;
        let result = timeout(SEND_TIMEOUT, async {
            if self.stream.is_none() {
                self.stream = Some(TcpStream::connect(&self.address).await?);
            }
            self.stream.as_mut().unwrap().write_all(&bytes).await
        })
        .await;
        match result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => {
                // A partial write may have reached the receiver; do not retry the line.
                self.stream = None;
                Err(error.into())
            }
            Err(_) => {
                self.stream = None;
                Err(TeamLogsError::Timeout)
            }
        }
    }

    fn encode(
        &self,
        level: LogLevel,
        logger_name: &str,
        message: &str,
        timestamp: &str,
        thread_name: &str,
        startup_capacity: Option<usize>,
    ) -> Result<Vec<u8>, TeamLogsError> {
        let entry = Entry {
            timestamp,
            version: "1",
            google_cloud_project: &self.project,
            nais_namespace_name: &self.namespace,
            nais_pod_name: &self.pod,
            nais_container_name: &self.app,
            logger_name,
            level: level.as_str(),
            level_value: level.value(),
            message,
            tags: ["TEAM_LOGS"],
            thread_name,
            endpoint: startup_capacity.map(|_| self.address.as_str()),
            queue_capacity: startup_capacity,
        };
        Ok(encode(&entry)?)
    }
}

fn encode(entry: &Entry<'_>) -> Result<Vec<u8>, serde_json::Error> {
    let mut bytes = serde_json::to_vec(entry)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn required(name: &'static str) -> Result<String, TeamLogsError> {
    env::var(name)
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or(TeamLogsError::MissingEnvironment(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncBufReadExt;
    use tokio::net::TcpListener;

    #[test]
    fn encodes_one_json_line_without_exposing_message_as_metadata() {
        let client = DirectTeamLogs::new(
            "team-logs.nais-system:5170",
            "dev-gcp",
            "paw",
            "pod-1",
            "app-1",
        );
        let bytes = client
            .encode(
                LogLevel::Warn,
                "team-logs-logger",
                "Ugyldig signatur \"abc\"",
                "2026-09-29T09:52:18.314744215Z",
                "tokio-runtime-worker",
                None,
            )
            .unwrap();
        assert_eq!(bytes.last(), Some(&b'\n'));
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["message"], "Ugyldig signatur \"abc\"");
        assert_eq!(value["level"], "WARN");
        assert_eq!(value["level_value"], 30_000);
        assert_eq!(value["@timestamp"], "2026-09-29T09:52:18.314744215Z");
        assert_eq!(value["@version"], "1");
        assert_eq!(value["tags"], serde_json::json!(["TEAM_LOGS"]));
        assert_eq!(value["thread_name"], "tokio-runtime-worker");
        assert_eq!(value["nais_namespace_name"], "paw");
        assert!(value.get("endpoint").is_none());
        assert!(value.get("queue_capacity").is_none());
    }

    #[test]
    fn startup_entry_has_short_message_and_separate_settings() {
        let client = DirectTeamLogs::new(
            "team-logs.nais-system:5170",
            "dev-gcp",
            "paw",
            "pod-1",
            "app-1",
        );
        let value: serde_json::Value = serde_json::from_slice(
            &client
                .encode(
                    LogLevel::Info,
                    "team-logs-logger",
                    "Buffered TeamLogs initialized",
                    "2026-09-29T09:52:18.314744215Z",
                    "main",
                    Some(100),
                )
                .unwrap(),
        )
        .unwrap();
        assert_eq!(value["message"], "Buffered TeamLogs initialized");
        assert_eq!(value["level"], "INFO");
        assert_eq!(value["level_value"], 20_000);
        assert_eq!(value["endpoint"], "team-logs.nais-system:5170");
        assert_eq!(value["queue_capacity"], 100);
        assert_eq!(value["nais_pod_name"], "pod-1");
    }

    #[tokio::test]
    #[ignore = "requires permission to bind a local TCP socket"]
    async fn sends_one_private_json_line_with_nais_metadata() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut client = DirectTeamLogs::new(
            listener.local_addr().unwrap().to_string(),
            "dev-gcp",
            "paw",
            "pod-1",
            "app-1",
        );
        let receiver = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut lines = tokio::io::BufReader::new(socket).lines();
            let line = lines.next_line().await.unwrap().unwrap();
            serde_json::from_str::<serde_json::Value>(&line).unwrap()
        });
        client
            .send(
                LogLevel::Warn,
                "team-logs-logger",
                "Ugyldig signatur",
                "2026-09-29T09:52:18.314744215Z",
                "test-thread",
                None,
            )
            .await
            .unwrap();
        let entry = receiver.await.unwrap();
        assert_eq!(entry["level"], "WARN");
        assert_eq!(entry["message"], "Ugyldig signatur");
        assert_eq!(entry["level_value"], 30_000);
        assert_eq!(entry["tags"], serde_json::json!(["TEAM_LOGS"]));
        assert_eq!(entry["google_cloud_project"], "dev-gcp");
        assert_eq!(entry["nais_namespace_name"], "paw");
        assert_eq!(entry["nais_pod_name"], "pod-1");
        assert_eq!(entry["nais_container_name"], "app-1");
    }

    #[tokio::test]
    #[ignore = "requires permission to bind a local TCP socket"]
    async fn sends_multiple_json_lines_on_one_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut client = DirectTeamLogs::new(
            listener.local_addr().unwrap().to_string(),
            "dev-gcp",
            "paw",
            "pod-1",
            "app-1",
        );
        let receiver = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut lines = tokio::io::BufReader::new(socket).lines();
            let first = lines.next_line().await.unwrap().unwrap();
            let second = lines.next_line().await.unwrap().unwrap();
            [first, second]
        });
        for message in ["first", "second"] {
            client
                .send(
                    LogLevel::Info,
                    "example",
                    message,
                    "2026-09-29T09:52:18Z",
                    "test",
                    None,
                )
                .await
                .unwrap();
        }
        let lines = timeout(SEND_TIMEOUT, receiver).await.unwrap().unwrap();
        for (line, message) in lines.iter().zip(["first", "second"]) {
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(line).unwrap()["message"],
                message
            );
        }
        assert!(client.stream.is_some());
    }

    #[tokio::test]
    #[ignore = "requires permission to bind a local TCP socket"]
    async fn failed_write_drops_connection_and_next_entry_reconnects() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut client = DirectTeamLogs::new(
            listener.local_addr().unwrap().to_string(),
            "dev-gcp",
            "paw",
            "pod-1",
            "app-1",
        );
        client
            .send(
                LogLevel::Info,
                "example",
                "first",
                "2026-09-29T09:52:18Z",
                "test",
                None,
            )
            .await
            .unwrap();
        let (first_socket, _) = listener.accept().await.unwrap();
        let mut lines = tokio::io::BufReader::new(first_socket).lines();
        assert!(lines.next_line().await.unwrap().unwrap().contains("first"));

        // Closing our write half makes the next write fail without relying on
        // when a peer close becomes visible to TCP.
        client.stream.as_mut().unwrap().shutdown().await.unwrap();
        assert!(
            client
                .send(
                    LogLevel::Info,
                    "example",
                    "failed",
                    "2026-09-29T09:52:18Z",
                    "test",
                    None
                )
                .await
                .is_err()
        );
        assert!(client.stream.is_none());

        client
            .send(
                LogLevel::Info,
                "example",
                "third",
                "2026-09-29T09:52:18Z",
                "test",
                None,
            )
            .await
            .unwrap();
        let (second_socket, _) = timeout(SEND_TIMEOUT, listener.accept())
            .await
            .unwrap()
            .unwrap();
        let line = tokio::io::BufReader::new(second_socket)
            .lines()
            .next_line()
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&line).unwrap()["message"],
            "third"
        );
    }
}
