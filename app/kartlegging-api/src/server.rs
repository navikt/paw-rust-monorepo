use axum::Router;
use tokio::signal::unix::{signal, SignalKind};
use tokio::task::{JoinError, JoinHandle};

pub async fn web_server_task(routes: Router) -> JoinHandle<anyhow::Result<()>> {
    tracing::info!("Starter webserver på adresse 0.0.0.0:8080");
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
        axum::serve(listener, routes).await?;
        Ok(())
    })
}

pub async fn shutdown_signal_task() -> anyhow::Result<String> {
    let mut term_signal = signal(SignalKind::terminate())?;
    let mut interrupt_signal = signal(SignalKind::interrupt())?;
    tokio::select! {
        _ = term_signal.recv() => Ok("SIGTERM".to_string()),
        _ = interrupt_signal.recv() => Ok("SIGINT".to_string())
    }
}

pub fn async_task_handler(
    name: &str,
    result: Result<anyhow::Result<()>, JoinError>,
) -> anyhow::Result<()> {
    match result {
        Ok(Ok(())) => {
            tracing::info!("{} avsluttet normalt", name);
            Ok(())
        }
        Ok(Err(e)) => {
            tracing::error!("{} avsluttet med feil: {}", name, e);
            Err(e)
        }
        Err(e) => {
            tracing::error!("Feil i spawned task for {}: {}", name, e);
            Err(e.into())
        }
    }
}

pub fn shutdown_handler(signal: anyhow::Result<String>) -> anyhow::Result<()> {
    tracing::info!("Mottok shutdown-signal: {}", signal?);
    Ok(())
}
