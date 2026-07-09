use futures::future::BoxFuture;
use futures::FutureExt;
use testcontainers::core::logs::consumer::LogConsumer;
use testcontainers::core::logs::LogFrame;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};
use tokio::sync::OnceCell;

pub async fn setup_schema_registry_container(port: u16) -> anyhow::Result<TestContainerGuard> {
    let host_port = *CONTAINER_PORT
        .get_or_init(|| async {
            let container: &'static ContainerAsync<GenericImage> = Box::leak(Box::new(
                GenericImage::new("confluentinc/cp-schema-registry", "7.6.1")
                    .with_exposed_port(port.tcp())
                    .with_wait_for(WaitFor::message_on_stdout(
                        "Server started, listening for requests...",
                    ))
                    .with_env_var("SCHEMA_REGISTRY_HOST_NAME", "schema-registry")
                    .with_env_var(
                        "SCHEMA_REGISTRY_LISTENERS",
                        format!("http://0.0.0.0:{}", port),
                    )
                    .with_env_var("SCHEMA_REGISTRY_KAFKASTORE_BOOTSTRAP_SERVERS", "kafka:9092")
                    .with_log_consumer(PrintlnConsumer::new())
                    .start()
                    .await
                    .expect("Failed to start Schema Registry container"),
            ));
            container
                .get_host_port_ipv4(port)
                .await
                .expect("Failed to get Schema Registry port")
        })
        .await;

    Ok(TestContainerGuard { port: host_port })
}

static CONTAINER_PORT: OnceCell<u16> = OnceCell::const_new();

pub struct TestContainerGuard {
    pub port: u16,
}

struct PrintlnConsumer;

impl PrintlnConsumer {
    fn new() -> Self {
        Self {}
    }
}

impl LogConsumer for PrintlnConsumer {
    fn accept<'a>(&'a self, record: &'a LogFrame) -> BoxFuture<'a, ()> {
        async move {
            match record {
                LogFrame::StdOut(bytes) => println!("{}", String::from_utf8_lossy(bytes)),
                LogFrame::StdErr(bytes) => eprintln!("{}", String::from_utf8_lossy(bytes)),
            }
        }
        .boxed()
    }
}
