use crate::client::oppgave_client::OppgaveApiClient;
use crate::db::oppgave_functions::hent_og_oppdater_eldste_ubehandlede_oppgave;
use anyhow::Result;
use sqlx::PgPool;
use std::sync::Arc;

use tokio::time::{interval, Duration};
use crate::client::opprett_oppgave_request::to_oppgave_request;

pub async fn start_processing_loop(
    db_pool: PgPool,
    oppgave_api_client: Arc<OppgaveApiClient>,
) -> Result<(), anyhow::Error> {
    let mut interval = interval(Duration::from_secs(1));

    loop {
        interval.tick().await;
        /*
        if let Err(e) = prosesser_ubehandlede_oppgaver(db_pool.clone(), oppgave_api_client.clone()).await {
            log::error!("Feil i prosesseringsloop: {}", e);
        }
        */
    }
}

async fn prosesser_ubehandlede_oppgaver(
    db_pool: PgPool,
    oppgave_api_client: Arc<OppgaveApiClient>,
) -> Result<(), anyhow::Error> {
    let mut tx = db_pool.begin().await?;
    let oppgave = match hent_og_oppdater_eldste_ubehandlede_oppgave(&mut tx).await? {
        Some(oppgave) => oppgave,
        None => {
            log::info!("Ingen ubehandlede oppgaver funnet");
            return Ok(());
        }
    };

    let opprett_oppgave_request = to_oppgave_request(&oppgave);

    Ok(())
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use mockito::Server;
    use super::*;
    use paw_test::setup_test_db::setup_test_db;
    use texas_client::{M2MTokenClient, TokenResponse};

    #[tokio::test]
    async fn e2e_test_prosesser_ubehandlede_oppgaver() -> Result<()> {
        let mut server = Server::new_async().await;
        let token_client = Arc::new(MockTokenClient);
        let oppgave_api_client = Arc::new(OppgaveApiClient::new(server.url(), token_client));
        let (pg_pool, _db_container) = setup_test_db().await?;
        sqlx::migrate!("./migrations").run(&pg_pool).await?;

        // Ingen ubehandlede oppgaver
        {
            let result = prosesser_ubehandlede_oppgaver(pg_pool, oppgave_api_client).await?;
        }
        Ok(())
    }

    struct MockTokenClient;
    #[async_trait]
    impl M2MTokenClient for MockTokenClient {
        async fn get_token(&self, _target: String) -> Result<TokenResponse> {
            Ok(TokenResponse {
                access_token: "dummy-token".to_string(),
                expires_in: 3600,
                token_type: "Bearer".to_string(),
            })
        }
    }
}
