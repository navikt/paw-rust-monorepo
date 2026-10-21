#[derive(Clone, Eq, PartialEq, Hash)]
pub struct NavEnhetId {
    value: String,
}

impl NavEnhetId {
    pub fn new(value: String) -> Self {
        Self { value }
    }
}

impl AsRef<str> for NavEnhetId {
    fn as_ref(&self) -> &str {
        &self.value
    }
}

impl From<NavEnhetId> for String {
    fn from(nav_ident: NavEnhetId) -> Self {
        nav_ident.value
    }
}

impl std::fmt::Debug for NavEnhetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NavKontorId(*)")
    }
}

impl std::fmt::Display for NavEnhetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NavKontorId(*)")
    }
}
