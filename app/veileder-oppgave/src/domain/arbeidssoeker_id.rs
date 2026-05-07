use fmt::Display;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArbeidssoekerId(pub i64);

impl From<i64> for ArbeidssoekerId {
    fn from(id: i64) -> Self {
        ArbeidssoekerId(id)
    }
}

impl From<ArbeidssoekerId> for i64 {
    fn from(id: ArbeidssoekerId) -> Self {
        id.0
    }
}

impl Display for ArbeidssoekerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
