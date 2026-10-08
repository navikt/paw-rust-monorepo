use rdkafka::Message;

use super::verifier::SignatureError;
use super::wire::{last_header, valid_key_id};
use super::{SIGNATURE_HEADER, SIGNING_KEY_ID_HEADER, TRACEPARENT_HEADER};

pub(crate) fn warning_message<M: Message>(error: &SignatureError, message: &M) -> String {
    let topic = message.topic();
    let partition = message.partition();
    let offset = message.offset();
    let traceparent = last_header(message.headers(), TRACEPARENT_HEADER)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .filter(|value| {
            value.len() == 55 && value.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-')
        })
        .unwrap_or("ukjent");
    let trace_id = traceparent.split('-').nth(1).unwrap_or("ukjent");
    match error {
        SignatureError::MissingSignature => {
            let signature = last_header(message.headers(), SIGNATURE_HEADER).is_some();
            let key_id = last_header(message.headers(), SIGNING_KEY_ID_HEADER).is_some();
            format!(
                "[kafka-signing] Mangler signaturheader(er) — topic={topic}, partition={partition}, offset={offset}, harSignatur={signature}, harNøkkelId={key_id}, trace_id={trace_id}, traceparent={traceparent}"
            )
        }
        SignatureError::UnknownKey { key_id } => {
            let key_id = if valid_key_id(key_id) {
                key_id.as_str()
            } else {
                "ukjent"
            };
            format!(
                "[kafka-signing] Ukjent signeringsnøkkel-id='{key_id}' — topic={topic}, partition={partition}, offset={offset}, trace_id={trace_id}, traceparent={traceparent}"
            )
        }
        SignatureError::InvalidSignature { key_id } => {
            let key_id = if valid_key_id(key_id) {
                key_id.as_str()
            } else {
                "ukjent"
            };
            format!(
                "[kafka-signing] Ugyldig signatur — topic={topic}, partition={partition}, offset={offset}, keyId='{key_id}', trace_id={trace_id}, traceparent={traceparent}"
            )
        }
        SignatureError::TechnicalError { reason } => format!(
            "[kafka-signing] Teknisk feil ved signaturvalidering ({reason}) — topic={topic}, partition={partition}, offset={offset}, trace_id={trace_id}, traceparent={traceparent}"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rdkafka::message::{Header, OwnedHeaders, OwnedMessage, Timestamp};

    #[test]
    fn warning_text_describes_signature_errors() {
        let record = OwnedMessage::new(
            None,
            None,
            "topic".into(),
            Timestamp::CreateTime(1),
            2,
            42,
            None,
        );
        assert_eq!(
            warning_message(&SignatureError::MissingSignature, &record),
            "[kafka-signing] Mangler signaturheader(er) — topic=topic, partition=2, offset=42, harSignatur=false, harNøkkelId=false, trace_id=ukjent, traceparent=ukjent"
        );
        assert_eq!(
            warning_message(
                &SignatureError::UnknownKey {
                    key_id: "key-id".into()
                },
                &record
            ),
            "[kafka-signing] Ukjent signeringsnøkkel-id='key-id' — topic=topic, partition=2, offset=42, trace_id=ukjent, traceparent=ukjent"
        );
        assert_eq!(
            warning_message(
                &SignatureError::InvalidSignature {
                    key_id: "key-id".into()
                },
                &record
            ),
            "[kafka-signing] Ugyldig signatur — topic=topic, partition=2, offset=42, keyId='key-id', trace_id=ukjent, traceparent=ukjent"
        );
    }

    #[test]
    fn does_not_copy_untrusted_trace_header_into_alarm() {
        let record = OwnedMessage::new(
            None,
            None,
            "topic".into(),
            Timestamp::CreateTime(1),
            2,
            42,
            Some(OwnedHeaders::new().insert(Header {
                key: TRACEPARENT_HEADER,
                value: Some("hello\nsecret"),
            })),
        );
        let line = warning_message(
            &SignatureError::TechnicalError {
                reason: "invalid signature DER",
            },
            &record,
        );
        assert!(line.contains("traceparent=ukjent"));
        assert!(!line.contains("secret"));
    }
}
