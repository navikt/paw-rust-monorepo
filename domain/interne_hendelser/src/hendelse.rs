use std::collections::HashSet;
use uuid::Uuid;

use crate::vo::{Metadata, Opplysning};

pub trait Hendelse {
    fn hendelse_id(&self) -> Uuid;
    fn id(&self) -> i64;
    fn identitetsnummer(&self) -> &str;
    fn metadata(&self) -> &Metadata;
    fn opplysninger(&self) -> &HashSet<Opplysning>;
}
