use interne_hendelser::vo::BrukerType;

use crate::domain::{
    arbeidssoeker_id::ArbeidssoekerId, opplysninger::Opplysninger,
};
use types::identitetsnummer::Identitetsnummer;

pub(crate) enum UtgangHendelser {
    // Metadata mottatt, det kommer trolig en Startet koblet til denne,
    // men det er ikke gitt. Som regel vil denne komme før Startet, men rekkefølgen er ikke 100%
    // garantert.
    MetadataMottatt {
        hendelse_id: uuid::Uuid,
        timestamp: chrono::DateTime<chrono::Utc>,
        arbeidssoker_id: ArbeidssoekerId,
        opplysninger: Opplysninger,
    },
    //En arbeidssøkerperiode har blitt startet.
    Startet {
        periode_id: uuid::Uuid,
        timestamp: chrono::DateTime<chrono::Utc>,
        identitetsnummer: Identitetsnummer,
        brukertype: BrukerType,
    },
    //Pdl data er endret siden siste kontroll,
    //eventuelt siden metadata mottatt. Dette skal trigger en kontroll som vi gir en av følgende
    //utfall: StatusEndretTilAvvist, StatusEndretTilOK eller StatusIkkeEndret
    PdlDataEndret {
        periode_id: uuid::Uuid,
        timestamp: chrono::DateTime<chrono::Utc>,
        opplysninger: Opplysninger,
    },
    //Personen oppflyller ikke lenger ingangsvilkårene til til registeret og ville ikke blir godtatt
    //i dag.
    StatusEndretTilAvvist {
        period_id: uuid::Uuid,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    //Personen oppfyller fortsatt inngangsvilkårene og ville blitt godtatt i dag.
    StatusEndretTilOK {
        period_id: uuid::Uuid,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    //Status er ikke endret siden siste
    StatusIkkeEndret {
        period_id: uuid::Uuid,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    //Perioden har blitt avsluttet.
    Stoppet {
        period_id: uuid::Uuid,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}
