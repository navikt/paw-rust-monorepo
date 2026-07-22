use crate::claim::{EntraIdClaims, IdPortenClaims, MaskinportenClaims, TokenXClaims};
use crate::token::validate_token;
use errors::auth::AuthError;
use jsonwebtoken::{Algorithm, DecodingKey};
use types::identitetsnummer::Identitetsnummer;
use types::nav_ident::NavIdent;

#[derive(Clone, Debug)]
pub struct Borger {
    pub ident: Identitetsnummer,
}

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
pub struct EksterntSystem {
    pub sub: Option<String>,
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

pub trait AsPrincipal {
    fn as_principal(&self) -> Result<Principal, AuthError>;
}

impl AsPrincipal for TokenXClaims {
    fn as_principal(&self) -> Result<Principal, AuthError> {
        let pid = self
            .pid
            .clone()
            .filter(|s| !s.is_empty())
            .ok_or(AuthError::MissingClaim("pid".to_string()))?;
        Ok(Principal::Borger(Borger {
            ident: Identitetsnummer::new(pid).ok_or(AuthError::MissingClaim("pid".to_string()))?,
        }))
    }
}

impl AsPrincipal for EntraIdClaims {
    fn as_principal(&self) -> Result<Principal, AuthError> {
        let nav_ident = self
            .nav_ident
            .clone()
            .filter(|s| !s.is_empty())
            .ok_or(AuthError::MissingClaim("NavIdent".to_string()))?;
        Ok(Principal::NavAnsatt(NavAnsatt {
            ident: NavIdent::new(nav_ident)
                .ok_or(AuthError::MissingClaim("NavIdent".to_string()))?,
            oid: self.oid.clone(),
            name: self.name.clone(),
            roles: self.roles.clone().unwrap_or_default(),
        }))
    }
}

impl AsPrincipal for IdPortenClaims {
    fn as_principal(&self) -> Result<Principal, AuthError> {
        let pid = self
            .pid
            .clone()
            .filter(|s| !s.is_empty())
            .ok_or(AuthError::MissingClaim("pid".to_string()))?;
        Ok(Principal::Borger(Borger {
            ident: Identitetsnummer::new(pid).ok_or(AuthError::MissingClaim("pid".to_string()))?,
        }))
    }
}

impl AsPrincipal for MaskinportenClaims {
    fn as_principal(&self) -> Result<Principal, AuthError> {
        Ok(Principal::EksterntSystem(EksterntSystem {
            sub: self.sub.clone(),
        }))
    }
}

pub fn build_tokenx_principal(
    token: &str,
    alg: Algorithm,
    key: &DecodingKey,
    issuer: &str,
    client_id: &str,
) -> Result<Principal, AuthError> {
    let claims = validate_token::<TokenXClaims>(token, alg, key, issuer, client_id)?;
    claims.as_principal()
}

pub fn build_azure_principal(
    token: &str,
    alg: Algorithm,
    key: &DecodingKey,
    issuer: &str,
    client_id: &str,
) -> Result<Principal, AuthError> {
    let claims = validate_token::<EntraIdClaims>(token, alg, key, issuer, client_id)?;
    claims.as_principal()
}

pub fn build_idporten_principal(
    token: &str,
    alg: Algorithm,
    key: &DecodingKey,
    issuer: &str,
    client_id: &str,
) -> Result<Principal, AuthError> {
    let claims = validate_token::<IdPortenClaims>(token, alg, key, issuer, client_id)?;
    claims.as_principal()
}

pub fn build_maskinporten_principal(
    token: &str,
    alg: Algorithm,
    key: &DecodingKey,
    issuer: &str,
    client_id: &str,
) -> Result<Principal, AuthError> {
    let claims = validate_token::<MaskinportenClaims>(token, alg, key, issuer, client_id)?;
    claims.as_principal()
}
