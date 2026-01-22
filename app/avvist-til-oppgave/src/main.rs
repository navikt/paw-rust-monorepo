mod kafka_hwm;

use std::sync::Arc;
use health::simple_app_state::AppState;
use log::LevelFilter;
use log4rs::Config;
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Logger, Root};
use log4rs::encode::json::JsonEncoder;
use tokio::task::JoinHandle;
use axum_health::routes;

#[tokio::main]
async fn main() {
    init_log();
    log::info!("Application started");
    let appstate = Arc::new(AppState::new());
    appstate.set_has_started(true);;
    let health_routes = routes(appstate);
    let web_server_task : JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
        axum::serve(listener, health_routes).await?;
        Ok(())
    });

    match web_server_task.await {
        Ok(Ok(())) => log::info!("Web server exited successfully."),
        Ok(Err(e)) => log::error!("Web server error: {}", e),
        Err(e) => log::error!("Task join error: {}", e),
    }
}

fn init_log() {
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(JsonEncoder::new()))
        .build();
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .logger(Logger::builder().build("app::paw-arbeidssoekerregisteret-avvist-til-oppgave", LevelFilter::Info))
        .build(Root::builder().appender("stdout").build(LevelFilter::Info))
        .unwrap();
    log4rs::init_config(config).unwrap();
}