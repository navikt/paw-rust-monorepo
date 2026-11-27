//! 🔴 Rød sone: gå gjennom nøye. Dette er det eneste stedet der fødselsnummer
//! fra databasen er på vei til HTML. Alt som ikke er tillatt i klartekst, blir
//! byttet ut med en bokstav som bare gjelder for én visning.

use std::collections::HashMap;

use crate::features::innlesing::metadata::utfoert::UtfoertAvType;

/// Gir hver ulike verdi en bokstav (A, B, C …) i den rekkefølgen verdiene
/// blir sett. Bokstaven er ikke avledet fra verdien, så den kan ikke regnes
/// tilbake, og samme verdi får samme bokstav bare innenfor samme tabell.
#[derive(Default)]
pub struct Pseudonymer {
    tildelt: HashMap<String, usize>,
}

impl Pseudonymer {
    pub fn bokstav(&mut self, verdi: &str) -> String {
        let neste = self.tildelt.len();
        let nr = *self.tildelt.entry(verdi.to_string()).or_insert(neste);
        navn(nr)
    }

    /// `utfoert_av_id` i klartekst bare når typen og formen sier at det er en
    /// NAV-ident eller en system-id. Alt annet, også ukjente typer og verdier
    /// som ser ut som fødselsnummer, får en bokstav fra samme tabell. Da ser man
    /// om en sluttbruker er den samme personen som meldingen gjelder.
    pub fn utfoert_av(&mut self, av_type: UtfoertAvType, av_id: &str) -> String {
        if tillatt_i_klartekst(av_type, av_id) {
            av_id.to_string()
        } else {
            self.bokstav(av_id)
        }
    }
}

fn tillatt_i_klartekst(av_type: UtfoertAvType, av_id: &str) -> bool {
    if ser_ut_som_fnr(av_id) {
        return false;
    }
    match av_type {
        UtfoertAvType::Veileder => er_nav_ident(av_id),
        UtfoertAvType::System => true,
        UtfoertAvType::Sluttbruker | UtfoertAvType::Udefinert | UtfoertAvType::UkjentVerdi => false,
    }
}

/// Én stor bokstav fulgt av seks sifre, for eksempel `Z123456`.
fn er_nav_ident(verdi: &str) -> bool {
    let mut tegn = verdi.chars();
    tegn.next().is_some_and(|c| c.is_ascii_uppercase())
        && tegn.clone().count() == 6
        && tegn.all(|c| c.is_ascii_digit())
}

/// Elleve sifre, også med mellomrom eller bindestrek imellom.
fn ser_ut_som_fnr(verdi: &str) -> bool {
    let sifre = verdi.chars().filter(char::is_ascii_digit).count();
    sifre == 11
        && verdi
            .chars()
            .all(|c| c.is_ascii_digit() || c == ' ' || c == '-')
}

/// A, B … Z, deretter AA, AB …
fn navn(nr: usize) -> String {
    let mut nr = nr;
    let mut ut = Vec::new();
    loop {
        ut.push(char::from(b'A' + (nr % 26) as u8));
        if nr < 26 {
            break;
        }
        nr = nr / 26 - 1;
    }
    format!("Person {}", ut.into_iter().rev().collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FNR: &str = "12345678901";
    const ANNET_FNR: &str = "10987654321";

    #[test]
    fn samme_verdi_gir_samme_bokstav_i_rekkefolge() {
        let mut p = Pseudonymer::default();
        assert_eq!(p.bokstav(FNR), "Person A");
        assert_eq!(p.bokstav(ANNET_FNR), "Person B");
        assert_eq!(p.bokstav(FNR), "Person A");
    }

    #[test]
    fn bokstavene_fortsetter_etter_z() {
        assert_eq!(navn(25), "Person Z");
        assert_eq!(navn(26), "Person AA");
        assert_eq!(navn(27), "Person AB");
    }

    #[test]
    fn nav_ident_og_system_vises_i_klartekst() {
        let mut p = Pseudonymer::default();
        assert_eq!(p.utfoert_av(UtfoertAvType::Veileder, "Z123456"), "Z123456");
        assert_eq!(
            p.utfoert_av(UtfoertAvType::System, "paw-import"),
            "paw-import"
        );
    }

    #[test]
    fn alt_annet_blir_bokstav() {
        let mut p = Pseudonymer::default();
        assert_eq!(p.bokstav(FNR), "Person A");
        // Sluttbruker er samme person som meldingen gjelder.
        assert_eq!(p.utfoert_av(UtfoertAvType::Sluttbruker, FNR), "Person A");
        for (av_type, av_id) in [
            (UtfoertAvType::Veileder, FNR),
            (UtfoertAvType::Veileder, "z123456"),
            (UtfoertAvType::Veileder, "Z1234567"),
            (UtfoertAvType::System, ANNET_FNR),
            (UtfoertAvType::System, "109876 54321"),
            (UtfoertAvType::Udefinert, "noe"),
            (UtfoertAvType::UkjentVerdi, "Z123456"),
        ] {
            let vist = p.utfoert_av(av_type, av_id);
            assert!(vist.starts_with("Person "), "{av_type:?} {av_id}: {vist}");
        }
    }
}
