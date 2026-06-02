use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Stopp {
    #[serde(default)]
    pub frist_brutt: bool,
}

#[cfg(test)]
mod tests {
    use crate::bekreftelse::stopp::Stopp;

    #[test]
    fn test_serialize_stopp() {
        let stopp = Stopp { frist_brutt: true };
        let json = serde_json::to_string(&stopp).unwrap();
        assert_eq!(json, r#"{"fristBrutt":true}"#);
    }

    #[test]
    fn test_deserialize_stopp_med_default() {
        let stopp = Stopp { frist_brutt: false };
        let json = r#"{}"#;
        let deserialized: Stopp = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, stopp);
    }
}
