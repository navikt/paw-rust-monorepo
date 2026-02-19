use strum::{Display, EnumString};

#[derive(Debug, Display, EnumString)]
pub enum Status {
    Ok,
    Avvist,
}
