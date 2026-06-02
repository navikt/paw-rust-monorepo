use crate::identiteter::identitet_hendelse::IdentitetHendelse;
use std::error::Error;

pub type IdenitetSerdeError = Box<dyn Error + Send + Sync>;

pub fn deserialize_identitet_hendelse(
    payload: &[u8],
) -> Result<IdentitetHendelse, IdenitetSerdeError> {
    let payload_str = std::str::from_utf8(payload)
        .map_err(|e| IdenitetSerdeError::from(format!("Invalid UTF-8 in payload: {}", e)))?;
    serde_json::from_str(payload_str)
        .map_err(|e| IdenitetSerdeError::from(format!("Failed to deserialize event: {}", e)))
}

#[cfg(test)]
mod tests {
    use crate::identiteter::identitet::Identitet;
    use crate::identiteter::identitet_hendelse::IdentitetHendelse;
    use crate::identiteter::identitet_hendelse_serde::deserialize_identitet_hendelse;
    use crate::identiteter::identitet_type::IdentitetType;

    #[test]
    fn test_deserialize() {
        let endret_hendelse_json = br#"{
                        "hendelseId": "475a7393-55a8-4e31-8a80-010d22ec4549",
                        "hendelseType": "identitet.v1.identiteter_endret",
                        "hendelseTidspunkt": "2024-06-01T12:00:00Z",
                        "identiteter": [
                            {
                                "identitet": "12345678901",
                                "type": "FOLKEREGISTERIDENT",
                                "gjeldende": true
                            }
                        ],
                        "tidligereIdentiteter": [
                            {
                                "identitet": "12345678901",
                                "type": "FOLKEREGISTERIDENT",
                                "gjeldende": false
                            }
                        ]
                    }
                "#;

        let endret_hendelse_result = deserialize_identitet_hendelse(endret_hendelse_json);
        match endret_hendelse_result {
            Ok(hendelse) => {
                if let IdentitetHendelse::IdentiteterEndret(identitet_hendelse) = hendelse {
                    assert_eq!(
                        identitet_hendelse.hendelse_id.to_string(),
                        "475a7393-55a8-4e31-8a80-010d22ec4549"
                    );
                    assert_eq!(
                        identitet_hendelse.identiteter,
                        vec![Identitet {
                            identitet: "12345678901".to_string(),
                            identitet_type: IdentitetType::Folkeregisterident,
                            gjeldende: true
                        }]
                    );
                } else {
                    panic!("Expected IdentiteterEndret variant");
                }
            }
            Err(ouch) => {
                panic!("Failed to deserialize endret hendelse: {}", ouch);
            }
        }
    }
}
