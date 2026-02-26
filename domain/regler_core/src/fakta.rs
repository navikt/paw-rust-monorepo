use anyhow::Result;

pub enum Utfall {
    Neutral,
    Positive,
    Negative,
}

pub trait Fakta {
    fn id(&self) -> &'static str;
}

pub trait UtledeFakta<INN, UT> {
    fn utlede_fakta(&self, input: &INN) -> Result<Vec<UT>>;
}
