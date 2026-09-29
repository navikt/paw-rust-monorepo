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
}

#[derive(Serialize)]
struct Entry<'a> {
    google_cloud_project: &'a str,
    nais_namespace_name: &'a str,
    nais_pod_name: &'a str,
    nais_container_name: &'a str,
    logger_name: &'a str,
    level: &'a str,
    message: &'a str,
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
        }
    }

    pub(crate) fn startup_message(&self, capacity: usize) -> String {
        format!(
            "Buffered TeamLogs initialized: endpoint={}, project={}, namespace={}, pod={}, app={}, queue_capacity={capacity}",
            self.address, self.project, self.namespace, self.pod, self.app,
        )
    }

    pub(crate) async fn send(
        &self,
        level: LogLevel,
        logger_name: &str,
        message: &str,
    ) -> Result<(), TeamLogsError> {
        let result = self.send_inner(level, logger_name, message).await;
        if result.is_err() {
            DELIVERY_FAILURES.with_label_values(&[level.as_str()]).inc();
        }
        result
    }

    async fn send_inner(
        &self,
        level: LogLevel,
        logger_name: &str,
        message: &str,
    ) -> Result<(), TeamLogsError> {
        let entry = Entry {
            google_cloud_project: &self.project,
            nais_namespace_name: &self.namespace,
            nais_pod_name: &self.pod,
            nais_container_name: &self.app,
            logger_name,
            level: level.as_str(),
            message,
        };
        let bytes = encode(&entry)?;
        timeout(SEND_TIMEOUT, async {
            let mut socket = TcpStream::connect(&self.address).await?;
            socket.write_all(&bytes).await?;
            socket.shutdown().await
        })
        .await
        .map_err(|_| TeamLogsError::Timeout)??;
        Ok(())
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
        let entry = Entry {
            google_cloud_project: "dev-gcp",
            nais_namespace_name: "paw",
            nais_pod_name: "pod-1",
            nais_container_name: "app-1",
            logger_name: "team-logs-logger",
            level: "WARN",
            message: "Ugyldig signatur \"abc\"",
        };
        let bytes = encode(&entry).unwrap();
        assert_eq!(bytes.last(), Some(&b'\n'));
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["message"], entry.message);
        assert_eq!(value["level"], "WARN");
        assert_eq!(value["nais_namespace_name"], "paw");
    }

    #[test]
    fn startup_entry_describes_destination_and_configuration() {
        let client = DirectTeamLogs::new(
            "team-logs.nais-system:5170",
            "dev-gcp",
            "paw",
            "pod-1",
            "app-1",
        );
        assert_eq!(
            client.startup_message(100),
            "Buffered TeamLogs initialized: endpoint=team-logs.nais-system:5170, project=dev-gcp, namespace=paw, pod=pod-1, app=app-1, queue_capacity=100"
        );
    }

    #[tokio::test]
    #[ignore = "requires permission to bind a local TCP socket"]
    async fn sends_one_private_json_line_with_nais_metadata() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let client = DirectTeamLogs::new(
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
            assert!(lines.next_line().await.unwrap().is_none());
            serde_json::from_str::<serde_json::Value>(&line).unwrap()
        });
        client
            .send(LogLevel::Warn, "team-logs-logger", "Ugyldig signatur")
            .await
            .unwrap();
        let entry = receiver.await.unwrap();
        assert_eq!(entry["level"], "WARN");
        assert_eq!(entry["message"], "Ugyldig signatur");
        assert_eq!(entry["google_cloud_project"], "dev-gcp");
        assert_eq!(entry["nais_namespace_name"], "paw");
        assert_eq!(entry["nais_pod_name"], "pod-1");
        assert_eq!(entry["nais_container_name"], "app-1");
    }
}
