---
name: rust
description: Writing idiomatic Rust code and handling common Rust issues — error handling, async, ownership, and patterns
---

# Rust Skill

This skill provides patterns and guidance for writing idiomatic Rust code and resolving common issues.

## Error Handling

### Custom Errors with `thiserror`

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("not found: {id}")]
    NotFound { id: String },

    #[error("invalid input: {message}")]
    InvalidInput { message: String },
}
```

### Error Propagation

```rust
// Use ? to propagate errors
async fn fetch_user(id: &str) -> Result<User, AppError> {
    let user = db.find_user(id).await?;
    Ok(user)
}

// Use anyhow for application-level error aggregation
use anyhow::{Context, Result};

async fn run() -> Result<()> {
    let config = load_config().context("failed to load config")?;
    Ok(())
}
```

## Async / Tokio

### Spawning Tasks

```rust
use tokio::task::JoinHandle;

let handle: JoinHandle<Result<()>> = tokio::spawn(async move {
    // background work
    Ok(())
});

// Await and handle result
handle.await??; // first ? = JoinError, second ? = inner Result
```

### Concurrent Tasks with `tokio::select!`

```rust
tokio::select! {
    result = task_a => { /* handle task_a completing */ },
    result = task_b => { /* handle task_b completing */ },
    _ = shutdown_signal => { /* graceful shutdown */ },
}
```

### Shared State

Wrap the inner state in `Arc` so the outer handle is cheaply cloneable without requiring callers to deal with `Arc<State>` directly.

```rust
use std::sync::Arc;

#[derive(Clone)]
pub struct MyState {
    inner: Arc<MyStateInner>,
}

struct MyStateInner {
    started: AtomicBool,
}

impl MyState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(MyStateInner {
                started: AtomicBool::new(false),
            }),
        }
    }

    pub fn set_started(&self, value: bool) {
        self.inner.started.store(value, Ordering::SeqCst);
    }

    pub fn is_started(&self) -> bool {
        self.inner.started.load(Ordering::SeqCst)
    }
}
```

### Shared Mutable State

For simple flag/counter values, prefer atomics over locks (`Mutex`/`RwLock`):

| Type | Use for |
| :--- | :--- |
| `AtomicBool` | Flags (started, healthy, shutdown) |
| `AtomicI64` / `AtomicU64` | Counters, timestamps |
| `AtomicUsize` | Sizes, indices |

```rust
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// Read
let value = counter.load(Ordering::SeqCst);

// Write
flag.store(true, Ordering::SeqCst);

// Compare-and-swap
counter.fetch_add(1, Ordering::SeqCst);
```

For complex types that cannot use atomics, use `tokio::sync::RwLock`:

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

let state = Arc::new(RwLock::new(MyState::new()));

let state_clone = Arc::clone(&state);
tokio::spawn(async move {
    let mut guard = state_clone.write().await;
    guard.update();
});
```

## Ownership & Borrowing

### Common Patterns

```rust
// Clone when you need owned data in async context
let owned = some_ref.to_owned();
let cloned = some_string.clone();

// Use Arc for shared ownership across threads
let shared = Arc::new(expensive_object);

// Lifetime annotations when returning references
fn first<'a>(items: &'a [String]) -> Option<&'a String> {
    items.first()
}
```

### Moving into Closures

```rust
// Use `move` to transfer ownership into async block/closure
tokio::spawn(async move {
    // owns `config` and `client` here
    do_work(config, client).await;
});
```

## Structs & Traits

### Builder Pattern

```rust
#[derive(Default)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub timeout_ms: u64,
}

impl Config {
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
}

let config = Config::default().host("localhost").port(8080);
```

### Implementing Traits

```rust
use async_trait::async_trait;

#[async_trait]
pub trait MessageProcessor: Send + Sync {
    async fn process(&self, msg: &Message) -> Result<(), ProcessorError>;
}

pub struct MyProcessor;

#[async_trait]
impl MessageProcessor for MyProcessor {
    async fn process(&self, msg: &Message) -> Result<(), ProcessorError> {
        // implementation
        Ok(())
    }
}
```

### Using Traits

Default to static dispatch. Use dynamic dispatch only when truly needed (e.g. heterogeneous collections, object-safe trait stored in a struct).

```rust
trait SomeTrait {}

// Option 1 — impl Trait (preferred for simple cases)
fn do_something(item: &impl SomeTrait) {}

// Option 2 — trait bound (preferred when the type parameter is reused)
fn complex_do_something<T: SomeTrait>(item: &T) {}

// Option 3 — where clause (cleaner with multiple bounds)
fn multi_bound<T>(item: &T) where T: SomeTrait + std::fmt::Display {}

// Dynamic dispatch — only when required
fn do_something_dyn(item: &dyn SomeTrait) {}
```

## Serialization with Serde

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub event_type: EventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

// Deserialize from JSON string
let event: Event = serde_json::from_str(json_str)?;

// Serialize to JSON string
let json = serde_json::to_string(&event)?;
```

## Configuration

```rust
use serde::Deserialize;

#[derive(Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

// Load from TOML
let config: DatabaseConfig = toml::from_str(include_str!("../config/database.toml"))?;
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        let result = my_function(input);
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_async_something() {
        let result = my_async_function().await;
        assert!(result.is_ok());
    }
}
```

### Integration Tests

```rust
// tests/integration_test.rs
use my_crate::MyService;

#[tokio::test]
async fn test_full_flow() {
    let service = MyService::new_for_test().await;
    let result = service.run().await;
    assert!(result.is_ok());
}
```

## Common Pitfalls

### Avoid `.unwrap()` and `.expect()` in Production Code

```rust
// ❌ Both panic on None/Err — do not use outside tests
let value = option.unwrap();
let value = option.expect("Some message");

// ✅ Propagate with ?
let value = option.ok_or(AppError::NotFound)?;

// ✅ Provide a default
let value = option.unwrap_or_default();
let value = option.unwrap_or_else(|| compute_default());
```

### String Types

```rust
// &str  = borrowed string slice (prefer in function params)
// String = owned, heap-allocated string

fn greet(name: &str) -> String {
    format!("Hello, {name}!")
}

// Accept both &str and String using Into<String>
fn set_name(name: impl Into<String>) {
    self.name = name.into();
}
```

### Iterator Chains

```rust
let total: u32 = items
    .iter()
    .filter(|item| item.is_active)
    .map(|item| item.value)
    .sum();

// Collect into Vec
let names: Vec<String> = users
    .into_iter()
    .map(|u| u.name)
    .collect();
```

## Tracing / Observability

```rust
use tracing::{info, error, warn, instrument};

// Structured logging
info!(user_id = %id, event = "login", "user logged in");
error!(error = %err, "failed to process message");

// Auto-instrument async functions (adds span)
#[instrument(skip(db), fields(user_id = %id))]
async fn find_user(db: &Pool, id: &str) -> Result<User, AppError> {
    // tracing span automatically created and closed
    Ok(db.get(id).await?)
}
```

## Checklist

- [ ] No `.unwrap()` or `.expect()` in non-test code
- [ ] Custom error types defined with `thiserror`
- [ ] `Arc<T>` used for shared ownership across async tasks
- [ ] `move` closures used when capturing variables in `tokio::spawn`
- [ ] `#[instrument]` on key async functions for tracing
- [ ] Unit tests for business logic, integration tests for I/O
- [ ] No PII (personal identifiers) in log statements
- [ ] `serde` derives used for all data structures crossing API boundaries
- [ ] Prefer static dispatch over dynamic dispatch
