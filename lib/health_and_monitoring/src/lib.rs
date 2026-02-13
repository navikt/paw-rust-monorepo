pub mod compound_health;
pub mod error;
pub mod nais_otel_setup;
pub mod otel_json_format_layer;
pub mod simple_app_state;

pub trait HealthCheck {
    fn name(&self) -> String;
    fn check(&self, check_type: &CheckType) -> Option<bool>;
}

pub enum CheckType {
    IsReady,
    IsAlive,
    HasStarted,
}
