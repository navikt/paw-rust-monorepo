use crate::logic::mutation::{arbeidssoeker_mutation, kartlegging_mutation, periode_mutation};
use crate::model::dao::kartlegging::KartleggingRow;
use crate::model::dao::{arbeidssoeker, kartlegging};
use crate::model::dto::arbeidssoeker::Arbeidssoeker;
use eksterne_hendelser::periode::Periode;
use eksterne_hendelser::serde::AvroDeserializer;
use paw_key_gen_client::client::PawKeyGenClient;
use paw_key_gen_client::model::IdentitetType;
use paw_rdkafka_hwm::hwm_message_processor::ProcessorError;
use pdl_client::pdl_query::PDLClient;
use schema_registry_converter::async_impl::schema_registry::SrSettings;
use sqlx::{Postgres, Transaction};
use std::sync::Arc;
use types::identitetsnummer::Identitetsnummer;

pub struct PeriodeProcessor {
    pub key_gen_client: Arc<PawKeyGenClient>,
    pub pdl_client: Arc<PDLClient>,
    pub deserializer: AvroDeserializer,
}

impl PeriodeProcessor {
    pub fn new(
        key_gen_client: Arc<PawKeyGenClient>,
        pdl_client: Arc<PDLClient>,
        schema_registry_setting: SrSettings,
    ) -> Self {
        Self {
            key_gen_client,
            pdl_client,
            deserializer: AvroDeserializer::new(schema_registry_setting),
        }
    }

    pub async fn process_payload<'a>(
        &'a self,
        tx: &mut Transaction<'_, Postgres>,
        payload: &'a [u8],
    ) -> anyhow::Result<(), ProcessorError> {
        let hendelse: Periode = self.deserializer.deserialize(payload).await.map_err(|e| {
            ProcessorError::from(format!("Failed to deserialize payload: {}", e.to_string()))
        })?;

        tracing::info!("Mottok hendelse: {:?}", &hendelse);

        // Kafka Key Gen data
        let identiteter_response = self
            .key_gen_client
            .finn_identiteter(hendelse.identitetsnummer.clone())
            .await?;
        let aktor_ider = identiteter_response.filter_by_type(IdentitetType::Aktorid);
        let aktor_id = aktor_ider.iter().find(|&i| i.gjeldende).unwrap();
        let arbeidssoeker_id = identiteter_response.arbeidssoeker_id.unwrap();
        let identiteter = identiteter_response.filter_by_type(IdentitetType::Folkeregisterident);
        let identitet = identiteter.iter().find(|&i| i.gjeldende).unwrap();
        let identitetsnummer = Identitetsnummer::new(identitet.identitet.clone()).unwrap();

        let arbeidssoeker_rows =
            arbeidssoeker::select_by_arbeidssoeker_id(tx, &arbeidssoeker_id).await?;
        let parent_id = if arbeidssoeker_rows.is_empty() {
            // PDL data
            let pdl_navn_response = self.pdl_client.hent_navn(identitetsnummer.clone()).await?;
            let pdl_navn = pdl_navn_response.unwrap();
            let navn = pdl_navn.navn.first().unwrap();
            let arbeidssoeker = Arbeidssoeker {
                aktor_id: aktor_id.identitet.clone(),
                arbeidssoeker_id: identiteter_response.arbeidssoeker_id.unwrap(),
                identitetsnummer: identitet.identitet.clone(),
                fornavn: navn.fornavn.clone(),
                mellomnavn: navn.mellomnavn.clone(),
                etternavn: navn.etternavn.clone(),
                ledighetsperioder: vec![],
                kontortilknytninger: vec![],
            };
            arbeidssoeker_mutation::lagre_dto(tx, &arbeidssoeker).await?
        } else {
            let arbeidssoeker_row = arbeidssoeker_rows.first().unwrap();
            arbeidssoeker_row.id
        };

        kartlegging_mutation::lagre_hendelse(tx, parent_id, &hendelse).await?;
        periode_mutation::lagre_hendelse(tx, &hendelse).await?;
        Ok(())
    }
}
