use regler_core::regler::{Avvisning, Godkjenning, Grunnlag, Regel};

#[derive(Debug)]
pub struct IkkeFunnet {}

impl Regel for IkkeFunnet {
    fn id(&self) -> &'static str {
        "IKKE_FUNNET"
    }

    fn utled(&self, _grunnlag: Grunnlag) -> Result<&dyn Godkjenning, &dyn Avvisning> {
        todo!()
    }
}
