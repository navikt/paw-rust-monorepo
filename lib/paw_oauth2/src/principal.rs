use crate::claim::{AzureClaims, IdPortenClaims, MaskinportenClaims, TokenXClaims};
use crate::token::validate_token;
use errors::auth::AuthError;
use jsonwebtoken::{Algorithm, DecodingKey};
use types::identitetsnummer::Identitetsnummer;
use types::nav_ident::NavIdent;

#[derive(Clone, Debug)]
pub struct NavAnsatt {
    pub oid: String,
    pub ident: NavIdent,
    pub name: Option<String>,
    pub roles: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct NavSystem {
    pub oid: String,
    pub roles: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct Borger {
    pub ident: Identitetsnummer,
}

#[derive(Clone, Debug)]
pub struct EksterntSystem {
    sub: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Anonym;

#[derive(Clone, Debug)]
pub enum Principal {
    NavAnsatt(NavAnsatt),
    NavSystem(NavSystem),
    Borger(Borger),
    EksterntSystem(EksterntSystem),
    Anonym(Anonym),
}

pub fn build_azure_principal(
    token: &str,
    alg: Algorithm,
    key: &DecodingKey,
    issuer: &str,
    client_id: &str,
) -> Result<Principal, AuthError> {
    let claims = validate_token::<AzureClaims>(token, alg, key, issuer, client_id)?;
    let nav_ident = claims
        .nav_ident
        .filter(|s| !s.is_empty())
        .ok_or(AuthError::MissingNavIdent)?;
    Ok(Principal::NavAnsatt(NavAnsatt {
        ident: NavIdent::new(nav_ident).ok_or(AuthError::MissingNavIdent)?,
        oid: claims.oid,
        name: claims.name,
        roles: claims.roles.unwrap_or_default(),
    }))
}

pub fn build_tokenx_principal(
    token: &str,
    alg: Algorithm,
    key: &DecodingKey,
    issuer: &str,
    client_id: &str,
) -> Result<Principal, AuthError> {
    let claims = validate_token::<TokenXClaims>(token, alg, key, issuer, client_id)?;
    let pid = claims
        .pid
        .filter(|s| !s.is_empty())
        .ok_or(AuthError::MissingPid)?;
    Ok(Principal::Borger(Borger {
        ident: Identitetsnummer::new(pid).ok_or(AuthError::MissingPid)?,
    }))
}

pub fn build_idporten_principal(
    token: &str,
    alg: Algorithm,
    key: &DecodingKey,
    issuer: &str,
    client_id: &str,
) -> Result<Principal, AuthError> {
    let claims = validate_token::<IdPortenClaims>(token, alg, key, issuer, client_id)?;
    let pid = claims
        .pid
        .filter(|s| !s.is_empty())
        .ok_or(AuthError::MissingPid)?;
    Ok(Principal::Borger(Borger {
        ident: Identitetsnummer::new(pid).ok_or(AuthError::MissingPid)?,
    }))
}

pub fn build_maskinporten_principal(
    token: &str,
    alg: Algorithm,
    key: &DecodingKey,
    issuer: &str,
    client_id: &str,
) -> Result<Principal, AuthError> {
    let claims = validate_token::<MaskinportenClaims>(token, alg, key, issuer, client_id)?;
    Ok(Principal::EksterntSystem(EksterntSystem {
        sub: claims.sub,
    }))
}
