use crate::model::dto::arbeidssoeker::Arbeidssoeker;
use crate::model::dto::navn::Navn;
use crate::model::error::{IdentityError, PayloadProcessorError};
use paw_key_gen_client::client::PawKeyGenClient;
use paw_key_gen_client::model::IdentitetType;
use pdl_client::client::PDLClient;
use rdkafka::message::OwnedMessage;
use std::sync::Arc;
use types::identitetsnummer::Identitetsnummer;

pub struct ArbeidssoekerService {
    key_gen_client: Arc<PawKeyGenClient>,
    pdl_client: Arc<PDLClient>,
}

impl ArbeidssoekerService {
    pub fn new(key_gen_client: Arc<PawKeyGenClient>, pdl_client: Arc<PDLClient>) -> Arc<Self> {
        Arc::new(Self {
            key_gen_client,
            pdl_client,
        })
    }

    #[tracing::instrument(skip_all)]
    pub async fn hent_identiteter<'a>(
        &'a self,
        message: &'a OwnedMessage,
        identitetsnummer: &'a String,
    ) -> anyhow::Result<Arbeidssoeker> {
        let identiteter_response = self
            .key_gen_client
            .finn_identiteter(identitetsnummer.clone())
            .await?;
        let arbeidssoeker_id = identiteter_response
            .arbeidssoeker_id
            .ok_or_else(|| IdentityError::not_found(message, IdentitetType::Arbeidssoekerid))?;
        let aktor_ider = identiteter_response.filter_by_type(IdentitetType::Aktorid);
        let aktor_id = aktor_ider
            .iter()
            .find(|&i| i.gjeldende)
            .ok_or_else(|| IdentityError::not_found(message, IdentitetType::Aktorid))?;
        let identiteter = identiteter_response.filter_by_type(IdentitetType::Folkeregisterident);
        let folkeregisterident = identiteter
            .iter()
            .find(|&i| i.gjeldende)
            .ok_or_else(|| IdentityError::not_found(message, IdentitetType::Folkeregisterident))?;
        Ok(Arbeidssoeker::from_identer(
            arbeidssoeker_id,
            aktor_id.identitet.clone(),
            folkeregisterident.identitet.clone(),
        ))
    }

    #[tracing::instrument(skip_all)]
    pub async fn hent_navn<'a>(
        &'a self,
        message: &'a OwnedMessage,
        identitetsnummer: &String,
    ) -> anyhow::Result<Option<Navn>> {
        let identitetsnummer_struct =
            Identitetsnummer::new(identitetsnummer.clone()).ok_or_else(|| {
                PayloadProcessorError::processing_error(
                    message,
                    "Ugyldig identitetsnummer fra kafka-key-gen",
                )
            })?;

        let pdl_navn_response = self
            .pdl_client
            .hent_person_navn(identitetsnummer_struct)
            .await?;
        let pdl_navn = pdl_navn_response.ok_or_else(|| {
            PayloadProcessorError::processing_error(message, "Fant ingen person i PDL")
        })?;

        if pdl_navn.navn.is_empty() {
            tracing::warn!("Fant ingen navn for person i PDL");
            Ok(None)
        } else {
            let pdl_navn_entry = pdl_navn.navn.first().ok_or_else(|| {
                PayloadProcessorError::processing_error(message, "Fant ingen navn for person i PDL")
            })?;
            Ok(Some(Navn::new(
                Some(pdl_navn_entry.fornavn.clone()),
                pdl_navn_entry.mellomnavn.clone(),
                Some(pdl_navn_entry.etternavn.clone()),
            )))
        }
    }
}
