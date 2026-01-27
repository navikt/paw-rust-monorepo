use crate::{CheckType, HealthCheck};
use std::sync::atomic::AtomicBool;

#[derive(Debug)]
pub struct AppState {
    is_alive: AtomicBool,
    is_ready: AtomicBool,
    has_started: AtomicBool,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            is_alive: AtomicBool::new(true),
            is_ready: AtomicBool::new(true),
            has_started: AtomicBool::new(false),
        }
    }

    pub fn set_is_alive(&self, value: bool) {
        self.is_alive
            .store(value, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn set_has_started(&self, value: bool) {
        self.has_started
            .store(value, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn set_is_ready(&self, value: bool) {
        self.is_ready
            .store(value, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn is_alive(&self) -> bool {
        self.is_alive.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn is_ready(&self) -> bool {
        self.is_ready.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn has_started(&self) -> bool {
        self.has_started.load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl HealthCheck for AppState {
    fn name(&self) -> String {
        "Simple AppState".to_string()
    }

    fn check(&self, check_type: &CheckType) -> Option<bool> {
        match check_type {
            CheckType::IsAlive => Some(self.is_alive()),
            CheckType::IsReady => Some(self.is_ready()),
            CheckType::HasStarted => Some(self.has_started()),
        }
    }
}
