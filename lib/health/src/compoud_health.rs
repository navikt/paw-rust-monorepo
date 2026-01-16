use crate::CheckType;
use crate::HealthCheck;

pub struct CompoudHealth {
    health_checks: Vec<Box<dyn HealthCheck + Send + Sync + 'static>>,
}
impl CompoudHealth {
    pub fn new() -> Self {
        CompoudHealth {
            health_checks: Vec::new(),
        }
    }

    pub fn add_health_check<H: HealthCheck + Send + Sync + 'static>(&mut self, health_check: H) {
        self.health_checks.push(Box::new(health_check));
    }
}

impl HealthCheck for CompoudHealth {
    fn name(&self) -> String {
        let names: String = self
            .health_checks
            .iter()
            .map(|hc| hc.name())
            .collect::<Vec<String>>()
            .join(", ");
        format!("CompoundHealth: [{}]", names)
    }

    fn check(&self, check_type: &CheckType) -> Option<bool> {
        self.health_checks
            .iter()
            .map(|check| check.check(check_type))
            .fold(None, |acc, opt| match (acc, opt) {
                (None, None) => None,
                (None, Some(b)) => Some(b),
                (Some(a), None) => Some(a),
                (Some(a), Some(b)) => Some(a && b),
            })
    }
}
