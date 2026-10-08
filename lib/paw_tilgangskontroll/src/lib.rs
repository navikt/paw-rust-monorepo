//! Trait-basert tilgangskontroll for Axum med appstyrte regler og problemsvar.
//!
//! Dispatch er en fail-closed stub som må fullføres før biblioteket tas i bruk.

pub mod middleware;
pub mod policy;

pub use middleware::{TilgangskontrollLayer, tilgangskontroll};
pub use policy::Policy;
