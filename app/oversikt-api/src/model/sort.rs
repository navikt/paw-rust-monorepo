use serde::{Deserialize, Serialize};
use std::fmt;
use strum::{AsRefStr, EnumString};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, EnumString, AsRefStr)]
pub enum SortOrder {
    #[strum(serialize = "ASC")]
    #[serde(rename = "ASC")]
    Ascending,
    #[strum(serialize = "DESC")]
    #[serde(rename = "DESC")]
    Descending,
}

impl fmt::Display for SortOrder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.as_ref().to_string())
    }
}
