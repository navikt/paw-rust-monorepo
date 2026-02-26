use opentelemetry::propagation::Extractor;
use opentelemetry::global;
use rdkafka::{
    message::Headers,
    Message,
};
use std::collections::HashMap;

use rdkafka::message::OwnedMessage;

#[derive(Debug, Clone)]
pub enum HeaderValue {
    String(String),
    Binary(Vec<u8>),
}

pub fn extract_headers_as_map(msg: &OwnedMessage) -> HashMap<&str, HeaderValue> {
    msg.headers()
        .into_iter()
        .flat_map(|headers| headers.iter())
        .filter_map(|header| {
            header.value.map(|val| {
                let header_value = if let Ok(s) = std::str::from_utf8(val) {
                    HeaderValue::String(s.to_string())
                } else {
                    HeaderValue::Binary(val.to_vec())
                };
                (header.key, header_value)
            })
        })
        .collect()
}

pub fn extract_remote_otel_context(headers: &HashMap<&str, HeaderValue>) -> Option<opentelemetry::Context> {
    struct HeaderExtractor<'a>(&'a HashMap<&'a str, HeaderValue>);

    impl<'a> Extractor for HeaderExtractor<'a> {
        fn get(&self, key: &str) -> Option<&str> {
            self.0.get(key).and_then(|v| match v {
                HeaderValue::String(s) => Some(s.as_str()),
                HeaderValue::Binary(_) => None,
            })
        }

        fn keys(&self) -> Vec<&str> {
            self.0.keys().copied().collect()
        }
    }

    let extractor = HeaderExtractor(headers);
    let context = global::get_text_map_propagator(|propagator| {
        propagator.extract(&extractor)
    });

    Some(context)
}
