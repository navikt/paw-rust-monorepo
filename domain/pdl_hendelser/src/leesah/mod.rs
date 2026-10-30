pub mod hendelse_type;
pub mod opplysninger;
pub mod personhendelse;

pub use hendelse_type::{Endringstype, Opplysningstype};
pub use personhendelse::{
    PERSONHENDELSE_SCHEMA, Personhendelse, PersonhendelseDeserializer, PersonhendelseSerializer,
};
