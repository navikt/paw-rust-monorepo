use crate::regler::{alvorlighet::Utfall, nokkel::Nokkel};

/// Result of one rule evaluation. Rule id and version are known by the
/// caller, so only the key and outcome are carried here.
#[derive(Debug, PartialEq, Eq)]
pub enum Funn {
    IkkeAktuell,
    AltOk(Nokkel),
    Feil(Nokkel, Utfall),
}
