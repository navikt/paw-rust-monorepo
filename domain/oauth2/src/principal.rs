use crate::claim::{EntraIdClaims, IdPortenClaims, MaskinportenClaims, TokenXClaims};
use crate::token::validate_token;
use errors::auth::OAuthError;
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
    fn as_principal(&self) -> Result<Principal, OAuthError>;
}

impl AsPrincipal for TokenXClaims {
    fn as_principal(&self) -> Result<Principal, OAuthError> {
        let pid = self
            .pid
            .clone()
            .filter(|s| !s.is_empty())
            .ok_or(OAuthError::MissingClaim("pid".to_string()))?;
        Ok(Principal::Borger(Borger {
            ident: Identitetsnummer::new(pid).ok_or(OAuthError::MissingClaim("pid".to_string()))?,
        }))
    }
}

impl AsPrincipal for EntraIdClaims {
    fn as_principal(&self) -> Result<Principal, OAuthError> {
        if self.is_obo_token() && !self.is_m2m_token() {
            let nav_ident = self
                .nav_ident
                .clone()
                .filter(|s| !s.is_empty())
                .ok_or(OAuthError::MissingClaim("NavIdent".to_string()))?;
            Ok(Principal::NavAnsatt(NavAnsatt {
                oid: self.oid.clone(),
                ident: NavIdent::new(nav_ident.clone())
                    .ok_or(OAuthError::MissingClaim("NavIdent".to_string()))?,
                name: self.name.clone(),
                roles: self.roles.clone().unwrap_or_default(),
            }))
        } else if self.is_m2m_token() && !self.is_obo_token() {
            let roles = self
                .roles
                .clone()
                .filter(|s| !s.is_empty())
                .ok_or(OAuthError::MissingClaim("roles".to_string()))?;
            Ok(Principal::NavSystem(NavSystem {
                oid: self.oid.clone(),
                roles: roles.clone(),
            }))
        } else {
            Err(OAuthError::InvalidToken(
                "Mangler påkrevde claims".to_string(),
            ))
        }
    }
}

impl AsPrincipal for IdPortenClaims {
    fn as_principal(&self) -> Result<Principal, OAuthError> {
        let pid = self
            .pid
            .clone()
            .filter(|s| !s.is_empty())
            .ok_or(OAuthError::MissingClaim("pid".to_string()))?;
        Ok(Principal::Borger(Borger {
            ident: Identitetsnummer::new(pid).ok_or(OAuthError::MissingClaim("pid".to_string()))?,
        }))
    }
}

impl AsPrincipal for MaskinportenClaims {
    fn as_principal(&self) -> Result<Principal, OAuthError> {
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
) -> Result<Principal, OAuthError> {
    let claims = validate_token::<TokenXClaims>(token, alg, key, issuer, client_id)?;
    claims.as_principal()
}

pub fn build_azure_principal(
    token: &str,
    alg: Algorithm,
    key: &DecodingKey,
    issuer: &str,
    client_id: &str,
) -> Result<Principal, OAuthError> {
    let claims = validate_token::<EntraIdClaims>(token, alg, key, issuer, client_id)?;
    claims.as_principal()
}

pub fn build_idporten_principal(
    token: &str,
    alg: Algorithm,
    key: &DecodingKey,
    issuer: &str,
    client_id: &str,
) -> Result<Principal, OAuthError> {
    let claims = validate_token::<IdPortenClaims>(token, alg, key, issuer, client_id)?;
    claims.as_principal()
}

pub fn build_maskinporten_principal(
    token: &str,
    alg: Algorithm,
    key: &DecodingKey,
    issuer: &str,
    client_id: &str,
) -> Result<Principal, OAuthError> {
    let claims = validate_token::<MaskinportenClaims>(token, alg, key, issuer, client_id)?;
    claims.as_principal()
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{EncodingKey, Header, encode};
    use serde::Serialize;

    const ISSUER: &str = "https://issuer.example.com";
    const CLIENT_ID: &str = "test-client";
    const SECRET: &[u8] = b"principal-test-signing-key";
    const PID: &str = "12345678901";

    type PrincipalBuilder =
        fn(&str, Algorithm, &DecodingKey, &str, &str) -> Result<Principal, OAuthError>;

    #[derive(Serialize)]
    struct TestClaims<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        iss: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        aud: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        exp: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        nbf: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pid: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        oid: Option<&'a str>,
        #[serde(rename = "NAVident", skip_serializing_if = "Option::is_none")]
        nav_ident: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        roles: Option<&'a [&'a str]>,
        #[serde(skip_serializing_if = "Option::is_none")]
        sub: Option<&'a str>,
    }

    impl Default for TestClaims<'_> {
        fn default() -> Self {
            Self {
                iss: Some(ISSUER),
                aud: Some(CLIENT_ID),
                exp: Some(9_999_999_999),
                nbf: None,
                pid: None,
                oid: None,
                nav_ident: None,
                name: None,
                roles: None,
                sub: None,
            }
        }
    }

    fn make_token(claims: &TestClaims<'_>) -> String {
        encode(
            &Header::new(Algorithm::HS256),
            claims,
            &EncodingKey::from_secret(SECRET),
        )
        .unwrap()
    }

    fn extract(builder: PrincipalBuilder, token: &str) -> Result<Principal, OAuthError> {
        builder(
            token,
            Algorithm::HS256,
            &DecodingKey::from_secret(SECRET),
            ISSUER,
            CLIENT_ID,
        )
    }

    #[test]
    fn tokenx_and_idporten_extract_borger() {
        let token = make_token(&TestClaims {
            pid: Some(PID),
            ..Default::default()
        });

        for builder in [
            build_tokenx_principal as PrincipalBuilder,
            build_idporten_principal,
        ] {
            let Principal::Borger(borger) = extract(builder, &token).unwrap() else {
                panic!("expected Borger");
            };
            assert_eq!(borger.ident.as_ref(), PID);
        }
    }

    #[test]
    fn tokenx_and_idporten_reject_missing_or_invalid_pid() {
        for pid in [
            None,
            Some(""),
            Some("1234567890"),
            Some("123456789012"),
            Some("1234567890a"),
            Some("1234567890 "),
        ] {
            let token = make_token(&TestClaims {
                pid,
                ..Default::default()
            });

            for builder in [
                build_tokenx_principal as PrincipalBuilder,
                build_idporten_principal,
            ] {
                assert!(
                    matches!(
                        extract(builder, &token),
                        Err(OAuthError::MissingClaim(claim)) if claim == "pid"
                    ),
                    "expected MissingClaim(pid) for {pid:?}"
                );
            }
        }
    }

    #[test]
    fn azure_extracts_obo_token_with_non_m2m_roles_as_nav_ansatt() {
        let token = make_token(&TestClaims {
            oid: Some("employee-oid"),
            nav_ident: Some("Z123456"),
            name: Some("Test Employee"),
            roles: Some(&["read", "write"]),
            ..Default::default()
        });

        let Principal::NavAnsatt(ansatt) = extract(build_azure_principal, &token).unwrap() else {
            panic!("expected NavAnsatt");
        };
        assert_eq!(ansatt.oid, "employee-oid");
        assert_eq!(ansatt.ident.as_ref(), "Z123456");
        assert_eq!(ansatt.name.as_deref(), Some("Test Employee"));
        assert_eq!(ansatt.roles, ["read", "write"]);
    }

    #[test]
    fn azure_extracts_nav_ansatt_without_optional_name_or_roles() {
        for roles in [None, Some(&[][..])] {
            let token = make_token(&TestClaims {
                oid: Some("employee-oid"),
                nav_ident: Some("Z123456"),
                roles,
                ..Default::default()
            });

            let Principal::NavAnsatt(ansatt) = extract(build_azure_principal, &token).unwrap()
            else {
                panic!("expected NavAnsatt");
            };
            assert_eq!(ansatt.oid, "employee-oid");
            assert_eq!(ansatt.ident.as_ref(), "Z123456");
            assert!(ansatt.name.is_none());
            assert!(ansatt.roles.is_empty());
        }
    }

    #[test]
    fn azure_extracts_m2m_token_as_nav_system() {
        for (nav_ident, roles) in [
            (None, &["access_as_application"][..]),
            (Some(""), &["read", "access_as_application", "write"][..]),
        ] {
            let token = make_token(&TestClaims {
                oid: Some("system-oid"),
                nav_ident,
                roles: Some(roles),
                ..Default::default()
            });

            let Principal::NavSystem(system) = extract(build_azure_principal, &token).unwrap()
            else {
                panic!("expected NavSystem");
            };
            assert_eq!(system.oid, "system-oid");
            assert_eq!(system.roles, roles);
        }
    }

    #[test]
    fn azure_rejects_token_that_is_neither_obo_nor_m2m() {
        for nav_ident in [None, Some("")] {
            for roles in [
                None,
                Some(&[][..]),
                Some(&["read", "write"][..]),
                Some(&["ACCESS_AS_APPLICATION"][..]),
                Some(&["access_as_application "][..]),
            ] {
                let token = make_token(&TestClaims {
                    oid: Some("system-oid"),
                    nav_ident,
                    roles,
                    ..Default::default()
                });

                assert!(
                    matches!(
                        extract(build_azure_principal, &token),
                        Err(OAuthError::InvalidToken(_))
                    ),
                    "expected InvalidToken for NAVident={nav_ident:?}, roles={roles:?}"
                );
            }
        }
    }

    #[test]
    fn azure_rejects_token_that_is_both_obo_and_m2m() {
        for roles in [
            &["access_as_application"][..],
            &["read", "access_as_application", "write"][..],
        ] {
            let token = make_token(&TestClaims {
                oid: Some("employee-oid"),
                nav_ident: Some("Z123456"),
                roles: Some(roles),
                ..Default::default()
            });

            assert!(matches!(
                extract(build_azure_principal, &token),
                Err(OAuthError::InvalidToken(_))
            ));
        }
    }

    #[test]
    fn azure_rejects_token_without_oid() {
        for claims in [
            TestClaims {
                nav_ident: Some("Z123456"),
                ..Default::default()
            },
            TestClaims {
                roles: Some(&["access_as_application"]),
                ..Default::default()
            },
        ] {
            assert!(matches!(
                extract(build_azure_principal, &make_token(&claims)),
                Err(OAuthError::InvalidToken(_))
            ));
        }
    }

    #[test]
    fn maskinporten_extracts_external_system_with_optional_subject() {
        for sub in [Some("external-system"), Some(""), None] {
            let token = make_token(&TestClaims {
                sub,
                ..Default::default()
            });

            let Principal::EksterntSystem(system) =
                extract(build_maskinporten_principal, &token).unwrap()
            else {
                panic!("expected EksterntSystem");
            };
            assert_eq!(system.sub.as_deref(), sub);
        }
    }

    #[test]
    fn all_builders_reject_invalid_tokens_before_extracting_principals() {
        let claims = TestClaims {
            pid: Some(PID),
            oid: Some("employee-oid"),
            nav_ident: Some("Z123456"),
            sub: Some("external-system"),
            ..Default::default()
        };
        let valid_token = make_token(&claims);
        let invalid_tokens = [
            ("malformed", "not-a-jwt".to_string()),
            (
                "wrong signature",
                encode(
                    &Header::new(Algorithm::HS256),
                    &claims,
                    &EncodingKey::from_secret(b"different-test-signing-key"),
                )
                .unwrap(),
            ),
            (
                "wrong algorithm",
                encode(
                    &Header::new(Algorithm::HS384),
                    &claims,
                    &EncodingKey::from_secret(SECRET),
                )
                .unwrap(),
            ),
            (
                "wrong issuer",
                make_token(&TestClaims {
                    iss: Some("https://other-issuer.example.com"),
                    ..claims
                }),
            ),
            (
                "wrong audience",
                make_token(&TestClaims {
                    aud: Some("other-client"),
                    ..claims
                }),
            ),
            (
                "expired",
                make_token(&TestClaims {
                    exp: Some(1),
                    ..claims
                }),
            ),
            (
                "not yet valid",
                make_token(&TestClaims {
                    nbf: Some(9_999_999_998),
                    ..claims
                }),
            ),
            (
                "missing issuer",
                make_token(&TestClaims {
                    iss: None,
                    ..claims
                }),
            ),
            (
                "missing audience",
                make_token(&TestClaims {
                    aud: None,
                    ..claims
                }),
            ),
            (
                "missing expiration",
                make_token(&TestClaims {
                    exp: None,
                    ..claims
                }),
            ),
        ];

        for (name, builder) in [
            ("TokenX", build_tokenx_principal as PrincipalBuilder),
            ("Entra ID", build_azure_principal),
            ("ID-porten", build_idporten_principal),
            ("Maskinporten", build_maskinporten_principal),
        ] {
            assert!(
                extract(builder, &valid_token).is_ok(),
                "{name} should accept the valid control token"
            );
            for (case, token) in &invalid_tokens {
                assert!(
                    matches!(extract(builder, token), Err(OAuthError::InvalidToken(_))),
                    "{name} should reject {case}"
                );
            }
        }
    }
}
