//! Modell for `pdl.leesah-v1`, generert for hånd fra `avro/avdl/leesah`.
//!
//! Typene inneholder personopplysninger. Ikke logg dem med `Debug`; bare
//! [`Personhendelse`] har en `Debug` som skjuler opplysningene.

pub mod hendelse_type;
pub mod opplysninger;
pub mod personhendelse;
pub mod personhendelse_header;

pub use hendelse_type::{Endringstype, Opplysningstype};
pub use personhendelse::{
    PERSONHENDELSE_SCHEMA, Personhendelse, PersonhendelseDeserializer, PersonhendelseSerializer,
};
pub use personhendelse_header::{
    PERSONHENDELSE_HEADER_SCHEMA, PersonhendelseHeader, PersonhendelseHodeDeserializer,
};
