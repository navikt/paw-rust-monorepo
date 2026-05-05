use fmt::Display;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EksternOppgaveId(pub i64);

impl From<i64> for EksternOppgaveId {
    fn from(id: i64) -> Self {
        EksternOppgaveId(id)
    }
}

impl From<EksternOppgaveId> for i64 {
    fn from(id: EksternOppgaveId) -> Self {
        id.0
    }
}

impl Display for EksternOppgaveId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
