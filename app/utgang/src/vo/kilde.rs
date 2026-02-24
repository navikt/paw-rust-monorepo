use strum::{Display, EnumString};

#[derive(Debug, Display, EnumString, PartialEq, Eq, Hash)]
pub enum InfoKilde {
    StartetHendelse,
    PdlSjekk,
}
