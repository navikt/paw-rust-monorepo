use crate::kafka::error::OversiktProcessorError;
use crate::logic::process::bekreftelse_paavegneav_process::BekreftelsePaaVegneAvProcessor;
use crate::logic::process::bekreftelse_process::BekreftelseProcessor;
use crate::logic::process::egenvurdering_process::EgenvurderingProcessor;
use crate::logic::process::oppfolgingsperiode_process::OppfolgingsperiodeProcessor;
use crate::logic::process::opplysninger_process::OpplysningerProcessor;
use crate::logic::process::periode_process::PeriodeProcessor;
use crate::logic::process::profilering_process::ProfileringProcessor;
use dab_oppfolgingperioder::oppfolgingsperiode::POAO_SISTE_OPPFOLGINGSPERIODE_V3_TOPIC;
use eksterne_hendelser::bekreftelse::bekreftelse::PAW_BEKREFTELSE_TOPIC;
use eksterne_hendelser::bekreftelse::paa_vegne_av::PAW_BEKREFTELSE_PAAVEGNEAV_TOPIC;
use eksterne_hendelser::egenvurdering::PAW_EGENVURDERING_TOPIC;
use eksterne_hendelser::opplysninger::PAW_OPPLYSNINGER_TOPIC;
use eksterne_hendelser::periode::PAW_PERIODE_TOPIC;
use eksterne_hendelser::profilering::PAW_PROFILERING_TOPIC;
use nais_schema_registry::config::create_schema_registry_settings;
use paw_key_gen_client::client::PawKeyGenClient;
use paw_rdkafka_hwm::hwm_message_processor::{MessageProcessor, ProcessorError};
use pdl_client::pdl_query::PDLClient;
use rdkafka::message::OwnedMessage;
use rdkafka::Message;
use sqlx::{Postgres, Transaction};
use std::pin::Pin;
use std::sync::Arc;
use tracing::{warn, Instrument};

pub trait MessageProcessorTrait {
    fn process_message<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<(), ProcessorError>> + Send + 'a>>;
}

pub struct KartleggingMessageProcessor {
    periode_processor: Arc<PeriodeProcessor>,
    opplysninger_processor: Arc<OpplysningerProcessor>,
    profilering_processor: Arc<ProfileringProcessor>,
    egenvurdering_processor: Arc<EgenvurderingProcessor>,
    bekreftelse_processor: Arc<BekreftelseProcessor>,
    bekreftelse_paavegneav_processor: Arc<BekreftelsePaaVegneAvProcessor>,
    oppfolgingsperiode_processor: Arc<OppfolgingsperiodeProcessor>,
}

impl KartleggingMessageProcessor {
    pub fn new(
        key_gen_client: Arc<PawKeyGenClient>,
        pdl_client: Arc<PDLClient>,
    ) -> anyhow::Result<Self> {
        let schema_registry_settings = create_schema_registry_settings()?;
        Ok(Self {
            periode_processor: Arc::new(PeriodeProcessor::new(
                key_gen_client,
                pdl_client,
                schema_registry_settings.clone(),
            )),
            opplysninger_processor: Arc::new(OpplysningerProcessor::new(
                schema_registry_settings.clone(),
            )),
            profilering_processor: Arc::new(ProfileringProcessor::new(
                schema_registry_settings.clone(),
            )),
            egenvurdering_processor: Arc::new(EgenvurderingProcessor::new(
                schema_registry_settings.clone(),
            )),
            bekreftelse_processor: Arc::new(BekreftelseProcessor::new(
                schema_registry_settings.clone(),
            )),
            bekreftelse_paavegneav_processor: Arc::new(BekreftelsePaaVegneAvProcessor::new(
                schema_registry_settings.clone(),
            )),
            oppfolgingsperiode_processor: Arc::new(OppfolgingsperiodeProcessor::new()),
        })
    }
}

impl MessageProcessor for KartleggingMessageProcessor {
    fn process_message<'a>(
        &'a self,
        tx: &'a mut Transaction<'_, Postgres>,
        message: &'a OwnedMessage,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<(), ProcessorError>> + Send + 'a>> {
        Box::pin(
            async move {
                tracing::info!(
                    "Mottok melding på topic: {}, partition: {}, offset: {}",
                    message.topic(),
                    message.partition(),
                    message.offset()
                );
                match (message.topic(), message.payload()) {
                    (topic, None) => Err(OversiktProcessorError::NoPayload {
                        topic: topic.to_string(),
                        partition: message.partition(),
                        offset: message.offset(),
                    }
                    .into()),
                    (topic, Some(payload)) if topic == PAW_PERIODE_TOPIC => {
                        self.periode_processor.process_payload(tx, payload).await
                    }
                    (topic, Some(payload)) if topic == PAW_OPPLYSNINGER_TOPIC => {
                        self.opplysninger_processor
                            .process_payload(tx, payload)
                            .await
                    }
                    (topic, Some(payload)) if topic == PAW_PROFILERING_TOPIC => {
                        self.profilering_processor
                            .process_payload(tx, payload)
                            .await
                    }
                    (topic, Some(payload)) if topic == PAW_EGENVURDERING_TOPIC => {
                        self.egenvurdering_processor
                            .process_payload(tx, payload)
                            .await
                    }
                    (topic, Some(payload)) if topic == PAW_BEKREFTELSE_TOPIC => {
                        self.bekreftelse_processor
                            .process_payload(tx, payload)
                            .await
                    }
                    (topic, Some(payload)) if topic == PAW_BEKREFTELSE_PAAVEGNEAV_TOPIC => {
                        self.bekreftelse_paavegneav_processor
                            .process_payload(tx, payload)
                            .await
                    }
                    (topic, Some(payload)) if topic == POAO_SISTE_OPPFOLGINGSPERIODE_V3_TOPIC => {
                        self.oppfolgingsperiode_processor
                            .process_payload(tx, payload)
                            .await
                    }
                    (topic, _) => {
                        warn!("Mottok melding på ukjent topic: {}", topic);
                        Ok(())
                    }
                }
            }
            .instrument(tracing::Span::current()),
        )
    }
}
