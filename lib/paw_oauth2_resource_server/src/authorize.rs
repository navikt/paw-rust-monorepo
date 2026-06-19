use oauth2::principal::Principal;

pub trait Authorize: Send + Sync {
    fn is_authorized(&self, principal: &Principal) -> bool;
}

pub struct AllowAll;

impl Authorize for AllowAll {
    fn is_authorized(&self, _: &Principal) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::authorize::{AllowAll, Authorize};
    use errors::auth::AuthError;
    use oauth2::principal::{Borger, NavAnsatt, Principal};
    use types::identitetsnummer::Identitetsnummer;
    use types::nav_ident::NavIdent;

    #[test]
    fn allow_all_authorizes_nav_ansatt() {
        let principal = Principal::NavAnsatt(NavAnsatt {
            oid: "oid".to_string(),
            ident: NavIdent::new("A123456".to_string())
                .ok_or(AuthError::MissingClaim("NavIdent".to_string()))
                .unwrap(),
            name: None,
            roles: vec![],
        });
        assert!(AllowAll.is_authorized(&principal));
    }

    #[test]
    fn allow_all_authorizes_borger() {
        let principal = Principal::Borger(Borger {
            ident: Identitetsnummer::new("12345678901".to_string())
                .ok_or(AuthError::MissingClaim("pid".to_string()))
                .unwrap(),
        });
        assert!(AllowAll.is_authorized(&principal));
    }
}
