#[derive(Clone, Eq, PartialEq, Hash)]
pub struct NavKontorId {
    value: String,
}

impl NavKontorId {
    pub fn new(value: String) -> Self {
        Self { value }
    }
}

impl AsRef<str> for NavKontorId {
    fn as_ref(&self) -> &str {
        &self.value
    }
}

impl From<NavKontorId> for String {
    fn from(nav_ident: NavKontorId) -> Self {
        nav_ident.value
    }
}

impl std::fmt::Debug for NavKontorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NavKontorId(*)")
    }
}

impl std::fmt::Display for NavKontorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NavKontorId(*)")
    }
}
