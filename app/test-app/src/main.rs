mod app_logic;
mod http_apis;

use crate::http_apis::register_http_apis;
use health::simple_app_state::AppState;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    match run_app().await {
        Ok(()) => println!("Application exited successfully."),
        Err(e) => eprintln!("Application error (code {}): {}", e.code(), e.description()),
    }
}

async fn run_app() -> Result<(), Box<dyn AppError>> {
    let app_state = Arc::new(AppState::new());
    let app_logic = Arc::new(app_logic::AppLogic::new(Arc::from("Hello")));
    let http_server_task = register_http_apis(app_state.clone(), app_logic.clone());
    app_state.set_has_started(true);
    match http_server_task.await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(Box::new(GenericError {
            description: format!("HTTP server error: {}", e),
            code: 500,
        })),
        Err(e) => Err(Box::new(GenericError {
            description: format!("Task join error: {}", e),
            code: 500,
        })),
    }
}

trait AppError {
    fn description(&self) -> &str;
    fn code(&self) -> u16;
}

struct GenericError {
    description: String,
    code: u16,
}

impl AppError for GenericError {
    fn description(&self) -> &str {
        &self.description
    }

    fn code(&self) -> u16 {
        self.code
    }
}
