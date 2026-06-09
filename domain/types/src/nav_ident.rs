#[derive(Clone, Eq, PartialEq, Hash)]
pub struct NavIdent {
    value: String,
}

impl NavIdent {
    pub fn new(value: String) -> Option<Self> {
        Some(Self { value })
    }
}

impl AsRef<str> for NavIdent {
    fn as_ref(&self) -> &str {
        &self.value
    }
}

impl From<NavIdent> for String {
    fn from(nav_ident: NavIdent) -> Self {
        nav_ident.value
    }
}

impl std::fmt::Debug for NavIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NavIdent(*)")
    }
}

impl std::fmt::Display for NavIdent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NavIdent(*)")
    }
}
