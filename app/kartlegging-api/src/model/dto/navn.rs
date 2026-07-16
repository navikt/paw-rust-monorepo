pub struct Navn {
    pub fornavn: Option<String>,
    pub mellomnavn: Option<String>,
    pub etternavn: Option<String>,
}

impl Navn {
    pub fn new(fornavn: String, mellomnavn: Option<String>, etternavn: String) -> Self {
        Self {
            fornavn: Some(fornavn),
            mellomnavn,
            etternavn: Some(etternavn),
        }
    }
}

impl Default for Navn {
    fn default() -> Self {
        Self {
            fornavn: None,
            mellomnavn: None,
            etternavn: None,
        }
    }
}
