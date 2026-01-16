use std::sync::Arc;

pub struct AppLogic {
    greeting: Arc<str>,
}

impl AppLogic {
    pub fn new(greeting: Arc<str>) -> Self {
        Self { greeting }
    }

    pub fn greet(&self, name: &str) -> String {
        format!("{}, {}!", self.greeting, name)
    }
}
