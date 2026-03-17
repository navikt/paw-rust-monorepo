use crate::pdl::hent_person_bolk::{
    HentPersonBolkHentPersonBolk, HentPersonBolkHentPersonBolkPerson,
    HentPersonBolkHentPersonBolkPersonBostedsadresse,
    HentPersonBolkHentPersonBolkPersonBostedsadresseMatrikkeladresse,
    HentPersonBolkHentPersonBolkPersonBostedsadresseUkjentBosted,
    HentPersonBolkHentPersonBolkPersonBostedsadresseUtenlandskAdresse,
    HentPersonBolkHentPersonBolkPersonBostedsadresseVegadresse,
    HentPersonBolkHentPersonBolkPersonFoedselsdato,
    HentPersonBolkHentPersonBolkPersonFolkeregisterpersonstatus,
    HentPersonBolkHentPersonBolkPersonFolkeregisterpersonstatusMetadata,
    HentPersonBolkHentPersonBolkPersonFolkeregisterpersonstatusMetadataEndringer,
    HentPersonBolkHentPersonBolkPersonInnflyttingTilNorge,
    HentPersonBolkHentPersonBolkPersonInnflyttingTilNorgeFolkeregistermetadata,
    HentPersonBolkHentPersonBolkPersonOpphold, HentPersonBolkHentPersonBolkPersonOppholdMetadata,
    HentPersonBolkHentPersonBolkPersonOppholdMetadataEndringer,
    HentPersonBolkHentPersonBolkPersonStatsborgerskap,
    HentPersonBolkHentPersonBolkPersonStatsborgerskapMetadata,
    HentPersonBolkHentPersonBolkPersonStatsborgerskapMetadataEndringer,
    HentPersonBolkHentPersonBolkPersonUtflyttingFraNorge,
    HentPersonBolkHentPersonBolkPersonUtflyttingFraNorgeFolkeregistermetadata, Oppholdstillatelse,
};
use graphql_client::GraphQLQuery;

pub type Date = String;
pub type DateTime = String;
pub type HentPerson = HentPersonBolkHentPersonBolk;
pub type Person = HentPersonBolkHentPersonBolkPerson;
pub type Foedselsdato = HentPersonBolkHentPersonBolkPersonFoedselsdato;
pub type Bostedsadresse = HentPersonBolkHentPersonBolkPersonBostedsadresse;
pub type Vegadresse = HentPersonBolkHentPersonBolkPersonBostedsadresseVegadresse;
pub type Matrikkeladresse = HentPersonBolkHentPersonBolkPersonBostedsadresseMatrikkeladresse;
pub type UkjentBosted = HentPersonBolkHentPersonBolkPersonBostedsadresseUkjentBosted;
pub type UtenlandskAdresse = HentPersonBolkHentPersonBolkPersonBostedsadresseUtenlandskAdresse;
pub type Statsborgerskap = HentPersonBolkHentPersonBolkPersonStatsborgerskap;
pub type StatsborgerskapMetadata = HentPersonBolkHentPersonBolkPersonStatsborgerskapMetadata;
pub type StatsborgerskapEndringer =
    HentPersonBolkHentPersonBolkPersonStatsborgerskapMetadataEndringer;
pub type Folkeregisterpersonstatus = HentPersonBolkHentPersonBolkPersonFolkeregisterpersonstatus;
pub type FolkeregisterpersonstatusMetadata =
    HentPersonBolkHentPersonBolkPersonFolkeregisterpersonstatusMetadata;
pub type FolkeregisterpersonstatusMetadataEndringer =
    HentPersonBolkHentPersonBolkPersonFolkeregisterpersonstatusMetadataEndringer;
pub type Opphold = HentPersonBolkHentPersonBolkPersonOpphold;
pub type OppholdMetadata = HentPersonBolkHentPersonBolkPersonOppholdMetadata;
pub type OppholdMetadataEndringer = HentPersonBolkHentPersonBolkPersonOppholdMetadataEndringer;
pub type UtflyttingFraNorge = HentPersonBolkHentPersonBolkPersonUtflyttingFraNorge;
pub type UtflyttingFraNorgeFolkeregistermetadata =
    HentPersonBolkHentPersonBolkPersonUtflyttingFraNorgeFolkeregistermetadata;
pub type InnflyttingTilNorge = HentPersonBolkHentPersonBolkPersonInnflyttingTilNorge;
pub type InnflyttingTilNorgeFolkeregistermetadata =
    HentPersonBolkHentPersonBolkPersonInnflyttingTilNorgeFolkeregistermetadata;

impl Default for Person {
    fn default() -> Self {
        Self {
            foedselsdato: vec![],
            statsborgerskap: vec![],
            opphold: vec![],
            folkeregisterpersonstatus: vec![],
            bostedsadresse: vec![],
            innflytting_til_norge: vec![],
            utflytting_fra_norge: vec![],
        }
    }
}

impl Default for Bostedsadresse {
    fn default() -> Self {
        Self {
            angitt_flyttedato: None,
            gyldig_fra_og_med: None,
            gyldig_til_og_med: None,
            vegadresse: None,
            matrikkeladresse: None,
            ukjent_bosted: None,
            utenlandsk_adresse: None,
        }
    }
}

impl Default for Vegadresse {
    fn default() -> Self {
        Self {
            kommunenummer: None,
        }
    }
}

impl Default for Matrikkeladresse {
    fn default() -> Self {
        Self {
            kommunenummer: None,
        }
    }
}

impl Default for UkjentBosted {
    fn default() -> Self {
        Self {
            bostedskommune: None,
        }
    }
}

impl Default for UtenlandskAdresse {
    fn default() -> Self {
        Self {
            landkode: "SWE".to_string(),
        }
    }
}

impl Default for Statsborgerskap {
    fn default() -> Self {
        Self {
            land: "".to_string(),
            metadata: StatsborgerskapMetadata::default(),
        }
    }
}

impl Default for StatsborgerskapMetadata {
    fn default() -> Self {
        Self { endringer: vec![] }
    }
}

impl Default for Folkeregisterpersonstatus {
    fn default() -> Self {
        Self {
            forenklet_status: "bosattEtterFolkeregisterloven".to_string(),
            metadata: FolkeregisterpersonstatusMetadata::default(),
        }
    }
}

impl Default for FolkeregisterpersonstatusMetadata {
    fn default() -> Self {
        Self { endringer: vec![] }
    }
}

impl Default for Opphold {
    fn default() -> Self {
        Self {
            type_: Oppholdstillatelse::PERMANENT,
            opphold_fra: None,
            opphold_til: None,
            metadata: OppholdMetadata::default(),
        }
    }
}

impl Default for OppholdMetadata {
    fn default() -> Self {
        Self { endringer: vec![] }
    }
}

impl Default for UtflyttingFraNorge {
    fn default() -> Self {
        Self {
            utflyttingsdato: None,
            folkeregistermetadata: None,
        }
    }
}

impl Default for InnflyttingTilNorge {
    fn default() -> Self {
        Self {
            folkeregistermetadata: None,
        }
    }
}

impl Clone for Oppholdstillatelse {
    fn clone(&self) -> Self {
        match self {
            Oppholdstillatelse::PERMANENT => Oppholdstillatelse::PERMANENT,
            Oppholdstillatelse::OPPLYSNING_MANGLER => Oppholdstillatelse::OPPLYSNING_MANGLER,
            Oppholdstillatelse::MIDLERTIDIG => Oppholdstillatelse::MIDLERTIDIG,
            Oppholdstillatelse::Other(status) => Oppholdstillatelse::Other(status.clone()),
        }
    }
}

impl Clone for InnflyttingTilNorge {
    fn clone(&self) -> Self {
        Self {
            folkeregistermetadata: self.folkeregistermetadata.clone(),
        }
    }
}

impl Clone for InnflyttingTilNorgeFolkeregistermetadata {
    fn clone(&self) -> Self {
        Self {
            gyldighetstidspunkt: self.gyldighetstidspunkt.clone(),
            ajourholdstidspunkt: self.ajourholdstidspunkt.clone(),
        }
    }
}

impl Clone for UtflyttingFraNorge {
    fn clone(&self) -> Self {
        Self {
            utflyttingsdato: self.utflyttingsdato.clone(),
            folkeregistermetadata: self.folkeregistermetadata.clone(),
        }
    }
}

impl Clone for UtflyttingFraNorgeFolkeregistermetadata {
    fn clone(&self) -> Self {
        Self {
            gyldighetstidspunkt: self.gyldighetstidspunkt.clone(),
            ajourholdstidspunkt: self.ajourholdstidspunkt.clone(),
        }
    }
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "graphql/pdl-schema.graphql",
    query_path = "graphql/hentPersonBolk.graphql"
)]
pub struct HentPersonBolk;
