use crate::fakta::Fakta;

pub struct Grunnlag {
    fakta: Vec<Box<dyn Fakta>>,
}
pub trait Godkjenning {
    fn fakta(&self) -> Vec<&dyn Fakta>;
    fn regel(&self) -> &dyn Regel;
}
pub trait Avvisning {}

pub trait Regel {
    fn id(&self) -> &'static str;
    fn utled(&self, grunnlag: Grunnlag) -> Result<&dyn Godkjenning, &dyn Avvisning>;
}

pub struct Regelsett<'a> {
    regler: Vec<&'a dyn Regel>,
}

pub struct Regelmotor<'a> {
    regelsett: Regelsett<'a>,
}
