use crate::model::dto::bekreftelse::{Bekreftelse, Bekreftelsesloesning};
use crate::model::dto::egenvurdering::Egenvurdering;
use crate::model::dto::opplysninger::Opplysninger;
use crate::model::dto::periode::Periode;
use crate::model::dto::profilering::Profilering;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ledighetsperiode {
    pub ledig_siden: Option<DateTime<Utc>>,
    pub periode: Option<Periode>,
    pub opplysninger: Option<Opplysninger>,
    pub profilering: Option<Profilering>,
    pub egenvurdering: Option<Egenvurdering>,
    pub bekreftelse: Option<Bekreftelse>,
    pub bekreftelse_paa_vegne_av: Vec<Bekreftelsesloesning>,
}
