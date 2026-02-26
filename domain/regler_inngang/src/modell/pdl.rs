use chrono::{NaiveDate, NaiveDateTime};

pub struct Person {
    pub foedselsdato: Vec<Foedselsdato>,
    pub statsborgerskap: Vec<Statsborgerskap>,
    pub opphold: Vec<Opphold>,
    pub folkeregisterpersonstatus: Vec<Folkeregisterpersonstatus>,
    pub bostedsadresse: Vec<Bostedsadresse>,
    pub innflytting_til_norge: Vec<InnflyttingTilNorge>,
    pub utflytting_fra_norge: Vec<UtflyttingFraNorge>,
}

pub struct Foedselsdato {
    pub foedselsdato: Option<NaiveDate>,
    pub foedselsaar: Option<i32>,
}

pub struct Statsborgerskap {}

pub struct Opphold {}

pub struct Folkeregisterpersonstatus {}

pub struct Bostedsadresse {
    pub angitt_flyttedato: Option<NaiveDate>,
    pub gyldig_fra_og_med: Option<NaiveDateTime>,
    pub gyldig_til_og_med: Option<NaiveDateTime>,
    pub vegadresse: Option<Vegadresse>,
    pub matrikkeladresse: Option<Matrikkeladresse>,
    pub ukjent_bosted: Option<UkjentBosted>,
    pub utenlandsk_adresse: Option<UtenlandskAdresse>,
}

pub struct Vegadresse {
    pub kommunenummer: Option<String>,
}

pub struct Matrikkeladresse {
    pub kommunenummer: Option<String>,
}

pub struct UkjentBosted {
    pub bostedskommune: Option<String>,
}

pub struct UtenlandskAdresse {
    pub landkode: String,
}

pub struct InnflyttingTilNorge {}

pub struct UtflyttingFraNorge {}
