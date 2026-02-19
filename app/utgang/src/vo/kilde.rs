use strum::{Display, EnumString};

#[derive(Debug, Display, EnumString)]
pub enum InfoKilde {
    StartetHendelse,
    PdlSjekk,
}
