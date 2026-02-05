use std::sync::Once;

static INIT: Once = Once::new();

pub fn register_panic_logger() {
    INIT.call_once(|| {
        std::panic::set_hook(Box::new(|panic_info| {
            eprintln!("PANIC occurred: {}", panic_info);
            if let Some(location) = panic_info.location() {
                eprintln!(
                    "PANIC location: {}:{}:{}",
                    location.file(),
                    location.line(),
                    location.column()
                );
            }
            if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                eprintln!("PANIC message: {}", s);
            } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                eprintln!("PANIC message: {}", s);
            }
        }));
    });
}
