use interne_hendelser::vo::Opplysning;
use uuid::Uuid;

pub struct OpplysningerHistorikk {
    pub id: i64,
    pub opplysninger_id: i64,
    pub periode_id: Uuid,
    pub identitetsnummer: String,
    pub gjeldende: Vec<Opplysning>,
    pub forrige: Option<Vec<Opplysning>>,
    pub startet: Option<Vec<Opplysning>>,
}
