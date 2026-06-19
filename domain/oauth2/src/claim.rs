use serde::Deserialize;

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
