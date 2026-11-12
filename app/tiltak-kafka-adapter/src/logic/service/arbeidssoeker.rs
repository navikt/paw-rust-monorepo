use std::sync::Arc;

pub struct ArbeidssoekerService {}

impl ArbeidssoekerService {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }
}
