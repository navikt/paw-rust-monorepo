use std::fmt;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub enum OppgaveType {
    AvvistUnder18,
}

impl FromStr for OppgaveType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "AvvistUnder18" => Ok(OppgaveType::AvvistUnder18),
            _ => Err(()),
        }
    }
}

impl fmt::Display for OppgaveType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OppgaveType::AvvistUnder18 => write!(f, "AvvistUnder18"),
        }
    }
}
