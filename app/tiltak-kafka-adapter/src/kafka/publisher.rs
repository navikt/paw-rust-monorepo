use mulighetsrommet::tiltakshistorikk::TiltakshistorikkV1Response;
use std::future::Future;
use uuid::Uuid;

pub trait TiltakshistorikkPublisher {
    fn publiser(
        &self,
        periode_id: &Uuid,
        historikk: &TiltakshistorikkV1Response,
    ) -> impl Future<Output = anyhow::Result<()>> + Send;
}

pub struct LoggPublisher;

impl TiltakshistorikkPublisher for LoggPublisher {
    async fn publiser(
        &self,
        periode_id: &Uuid,
        historikk: &TiltakshistorikkV1Response,
    ) -> anyhow::Result<()> {
        tracing::info!(
            periode_id = %periode_id,
            antall_deltakelser = historikk.historikk.len(),
            "Hentet tiltakshistorikk"
        );
        Ok(())
    }
}
