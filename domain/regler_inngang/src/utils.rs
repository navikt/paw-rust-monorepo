use chrono::{Datelike, Local, NaiveDate};

pub fn finn_alder(foedselsdato: NaiveDate) -> i32 {
    let dagens_dato = Local::now().date_naive();
    let mut age = dagens_dato.year() - foedselsdato.year();
    if dagens_dato.ordinal() < foedselsdato.ordinal() {
        age -= 1;
    }
    age
}
