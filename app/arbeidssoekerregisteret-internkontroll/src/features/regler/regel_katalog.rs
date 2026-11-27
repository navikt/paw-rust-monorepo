#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[repr(i16)]
pub enum RegelId {
    HeleTraceSignert = 1,
    IngenPeriodeUtenStartet = 2,
    StartetErSignert = 3,
    StartetGirPeriode = 4,
    PeriodeStartetLikStartet = 5,
}

impl RegelId {
    pub const ALLE: [RegelId; 5] = [
        Self::HeleTraceSignert,
        Self::IngenPeriodeUtenStartet,
        Self::StartetErSignert,
        Self::StartetGirPeriode,
        Self::PeriodeStartetLikStartet,
    ];

    /// Navnet som vises i visualiseringen.
    pub fn navn(self) -> &'static str {
        match self {
            Self::HeleTraceSignert => "Gyldig signatur i hele forløpet",
            Self::IngenPeriodeUtenStartet => "Perioden har en startet-hendelse",
            Self::StartetErSignert => "Startet-hendelsen er signert",
            Self::StartetGirPeriode => "Startet-hendelsen gir en periode",
            Self::PeriodeStartetLikStartet => "Perioden stemmer med startet-hendelsen",
        }
    }

    /// Kort forklaring av hva regelen sjekker.
    pub fn beskrivelse(self) -> &'static str {
        match self {
            Self::HeleTraceSignert => {
                "Alle meldinger i samme trace skal ha gyldig signatur. \
                 Meldinger fra før signering ble påkrevd, telles ikke med."
            }
            Self::IngenPeriodeUtenStartet => {
                "En periode skal ikke starte uten at det finnes en startet-hendelse \
                 for den i hendelsesloggen."
            }
            Self::StartetErSignert => {
                "Hver startet-hendelse i hendelsesloggen skal ha gyldig signatur. \
                 Ukjent eller tilbakekalt nøkkel og signatur som ikke stemmer, er kritisk. \
                 Tekniske feil gir advarsel."
            }
            Self::StartetGirPeriode => {
                "En startet-hendelse skal gi en periode på periode-topicen. \
                 Var forrige hendelse for personen også en startet-hendelse, \
                 er personen allerede registrert, og det skal ikke komme en ny periode."
            }
            Self::PeriodeStartetLikStartet => {
                "Perioden og startet-hendelsen skal ha samme fødselsnummer og samme utført av. \
                 Tidspunktene kan skille med inntil 1 sekund."
            }
        }
    }

    /// Kort tekst for en utfallskode. `None` for ukjente koder.
    pub fn kode_tekst(self, kode: i16) -> Option<&'static str> {
        Some(match (self, kode) {
            (Self::HeleTraceSignert, 1) => "Ugyldig signatur i trace",
            (Self::IngenPeriodeUtenStartet, 1) => "Mangler startet-hendelse",
            (Self::StartetErSignert, 1) => "Mangler signatur",
            (Self::StartetErSignert, 2) => "Signaturen stemmer ikke",
            (Self::StartetErSignert, 3) => "Ugyldig signaturformat",
            (Self::StartetErSignert, 4) => "Tilbakekalt nøkkel",
            (Self::StartetErSignert, 5) => "Ukjent nøkkel",
            (Self::StartetErSignert, 6) => "For stor melding",
            (Self::StartetErSignert, 7) => "Mangler tidsstempel",
            (Self::StartetErSignert, 8) => "Teknisk feil",
            (Self::StartetGirPeriode, 1) => "Mangler periode",
            (Self::PeriodeStartetLikStartet, 1) => "Ulikt fødselsnummer",
            (Self::PeriodeStartetLikStartet, 2) => "Ulik utført av",
            (Self::PeriodeStartetLikStartet, 3) => "Ulikt tidspunkt",
            _ => return None,
        })
    }

    pub fn fra_i16(verdi: i16) -> Option<Self> {
        Self::ALLE.into_iter().find(|r| *r as i16 == verdi)
    }
}
