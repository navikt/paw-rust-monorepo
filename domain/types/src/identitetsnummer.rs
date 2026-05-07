#[derive(Clone, Eq, PartialEq)]
pub struct Identitetsnummer {
    value: String,
}

impl Identitetsnummer {
    pub fn new(value: String) -> Option<Self> {
        if value.len() != 11 {
            return None;
        }
        if !value.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some(Self { value })
    }
}

impl From<Identitetsnummer> for String {
    fn from(identitetsnummer: Identitetsnummer) -> Self {
        identitetsnummer.value
    }
}

impl std::fmt::Debug for Identitetsnummer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Identitetsnummer(*)")
    }
}

impl std::fmt::Display for Identitetsnummer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Identitetsnummer(*)")
    }
}
