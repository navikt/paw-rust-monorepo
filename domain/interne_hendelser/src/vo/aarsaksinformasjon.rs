use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Aarsaksinformasjon {
    pub aarsak: AvsluttetAarsakType,
    pub regel_eval_resultat: RegelEvalResultat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AvsluttetAarsakType {
    SvarteNeiIBekreftelse,
    BekreftelseIkkeLevertInnenFrist,
    Feilregistrering,
    Udefinert,
    #[serde(other)]
    UkjentVerdi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RegelEvalResultat {
    IkkeRelevant,
    Udefinert,
    FeilUnderEval,
    Doed,
    Savnet,
    Opphoert,
    EuEoesIkkeBosatt,
    NorskIkkeBosatt,
    IkkeBosatt,
    Ok,
    #[serde(other)]
    UkjentVerdi,
}
