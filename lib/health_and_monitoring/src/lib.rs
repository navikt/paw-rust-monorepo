pub mod compound_health;
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

