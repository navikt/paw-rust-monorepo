use fmt::Display;
use std::fmt;

#[derive(Debug, Eq, Clone, Copy, PartialEq)]
pub struct OppgaveId(pub i64);

impl From<i64> for OppgaveId {
    fn from(id: i64) -> Self {
        OppgaveId(id)
    }
}

impl From<OppgaveId> for i64 {
    fn from(id: OppgaveId) -> Self {
        id.0
    }
}

impl Display for OppgaveId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
