use serde::Deserialize;

const ENTRA_ID_M2M_ROLE: &str = "access_as_application";

#[derive(Debug, Deserialize)]
pub struct TokenXClaims {
    pub pid: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EntraIdClaims {
    pub oid: String,
    pub name: Option<String>,
    #[serde(rename = "NAVident")]
    pub nav_ident: Option<String>,
    pub roles: Option<Vec<String>>,
}

impl EntraIdClaims {
    pub fn is_obo_token(&self) -> bool {
        match &self.nav_ident {
            None => false,
            Some(nav_ident) => !nav_ident.is_empty(),
        }
    }
    pub fn is_m2m_token(&self) -> bool {
        match &self.roles {
            None => false,
            Some(roles) => roles.iter().any(|role| role == ENTRA_ID_M2M_ROLE),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct IdPortenClaims {
    pub pid: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MaskinportenClaims {
    pub sub: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IssClaim {
    pub iss: String,
}
