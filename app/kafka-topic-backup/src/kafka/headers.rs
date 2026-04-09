use base64::{Engine as _, engine::general_purpose};
use rdkafka::{
    Message,
    message::{Headers, OwnedMessage},
};
use serde_json::{Map, Value};

/// Converts Kafka message headers to JSON format
///
/// Returns None if the message has no headers, otherwise returns a JSON object
/// where keys are header names and values are either strings (for UTF-8 data)
/// or base64-encoded strings (for binary data)
pub fn extract_headers_as_json(message: &OwnedMessage) -> Option<Value> {
    let headers = message.headers()?;
    let mut header_map = Map::new();

    for header in headers.iter() {
        let value = match header.value {
            Some(val) => match std::str::from_utf8(val) {
                Ok(s) => Value::String(s.to_string()),
                Err(_) => Value::String(general_purpose::STANDARD.encode(val)),
            },
            None => Value::Null,
        };
        header_map.insert(header.key.to_string(), value);
    }

    Some(Value::Object(header_map))
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose;
    use rdkafka::message::{Header, OwnedHeaders, Timestamp};

    #[test]
    fn ingen_headers_gir_none() {
        assert_eq!(extract_headers_as_json(&melding_uten_headers()), None);
    }

    #[test]
    fn utf8_header_dekodes_som_streng() {
        let headers = OwnedHeaders::new().insert(Header {
            key: "content-type",
            value: Some("application/json"),
        });
        let json_headers = extract_headers_as_json(&melding_med_headers(headers)).unwrap();
        assert_eq!(
            json_headers["content-type"],
            Value::String("application/json".to_string())
        );
    }

    #[test]
    fn header_med_none_verdi_gir_null() {
        let headers = OwnedHeaders::new().insert(Header::<&str> {
            key: "tom-header",
            value: None,
        });
        let json_headers = extract_headers_as_json(&melding_med_headers(headers)).unwrap();
        assert_eq!(json_headers["tom-header"], Value::Null);
    }

    #[test]
    fn ikke_utf8_header_base64_enkodes() {
        let ugyldig_utf8: &[u8] = &[0xFF, 0xFE];
        let headers = OwnedHeaders::new().insert(Header {
            key: "binær",
            value: Some(ugyldig_utf8),
        });
        let json_headers = extract_headers_as_json(&melding_med_headers(headers)).unwrap();
        let expected = general_purpose::STANDARD.encode(ugyldig_utf8);
        assert_eq!(json_headers["binær"], Value::String(expected));
    }

    #[test]
    fn alle_headers_konverteres() {
        let headers = OwnedHeaders::new()
            .insert(Header {
                key: "a",
                value: Some("1"),
            })
            .insert(Header {
                key: "b",
                value: Some("2"),
            });
        let json_headers = extract_headers_as_json(&melding_med_headers(headers)).unwrap();
        assert_eq!(json_headers["a"], Value::String("1".to_string()));
        assert_eq!(json_headers["b"], Value::String("2".to_string()));
    }

    fn melding_med_headers(headers: OwnedHeaders) -> OwnedMessage {
        OwnedMessage::new(
            None,
            None,
            "topic".to_string(),
            Timestamp::NotAvailable,
            0,
            0,
            Some(headers),
        )
    }

    fn melding_uten_headers() -> OwnedMessage {
        OwnedMessage::new(
            None,
            None,
            "topic".to_string(),
            Timestamp::NotAvailable,
            0,
            0,
            None,
        )
    }
}
