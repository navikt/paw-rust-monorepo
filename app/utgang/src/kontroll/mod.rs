mod evaluer;
mod hent_data;
mod task;

pub use evaluer::{KontrollStatus, SjekkFeil, sjekk_status};
pub use task::{KontrollTask, start_kontroll_task};
