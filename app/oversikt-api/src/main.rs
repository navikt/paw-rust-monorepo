use errors::database::DatabaseError;
use health_and_monitoring::{nais_otel_setup::setup_nais_otel, simple_app_state};
use oversikt_api::api::build_router;
use oversikt_api::config::{read_auth_config, read_database_config};
use oversikt_api::model::context::AppContext;
use oversikt_api::server::{async_task_handler, shutdown_signal_task, web_server_task};
use paw_oauth2_resource_server::state::AuthState;
use paw_rust_base::panic_logger::register_panic_logger;
use paw_sqlx::postgres::{clear_db, init_db};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    register_panic_logger();
    setup_nais_otel()?;

    let app_state = Arc::new(simple_app_state::AppState::new());

    let db_config = read_database_config()?;
    let db = init_db(db_config).await?;

    // TODO: Fjern før prodsetting!!!
    clear_db(&db).await?;

    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .map_err(DatabaseError::MigrateSchema)?;

    // Om det legges flere felter inn i context må den wrappes i en Arc. Trengs ikke nå siden PgPool allerede er en Arc.
    let context = AppContext::new(db);

    let auth_config = read_auth_config()?;
    let auth_state = AuthState::new(auth_config)
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let router = build_router(app_state.clone(), context, auth_state);
    let server_task = web_server_task(router).await;
    let signal_task = shutdown_signal_task();

    app_state.set_has_started(true);

    tokio::select! {
        result = server_task => async_task_handler("Webserver", result),
        signal = signal_task => {
            tracing::info!("Mottok shutdown-signal: {}", signal?);
            Ok(())
        },
    }?;

    Ok(())
}
