use futures::future::BoxFuture;
use futures::FutureExt;
use testcontainers::core::logs::consumer::LogConsumer;
use testcontainers::core::logs::LogFrame;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::kafka::apache::Kafka;
use tokio::sync::OnceCell;

pub async fn setup_kafka_broker_container(port: u16) -> anyhow::Result<TestContainerGuard> {
    let host_port = *CONTAINER_PORT
        .get_or_init(|| async {
            let container: &'static ContainerAsync<Kafka> = Box::leak(Box::new(
                Kafka::default()
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
