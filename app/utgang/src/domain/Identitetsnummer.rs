pub(crate) struct Identitetsnummer {
    value: String,
}

impl Identitetsnummer {
    pub fn new(value: String) -> Option<Self> {
        if (value.len() != 11) {
            return None;
        }
        if (!value.chars().all(|c| c.is_ascii_digit())) {
            return None;
        }
        return Some(Self { value });
    }
}

impl From<Identitetsnummer> for String {
    fn from(identitetsnummer: Identitetsnummer) -> Self {
        identitetsnummer.value
    }
}
