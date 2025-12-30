use std::panic;

pub fn install_panic_hook() {
    panic::set_hook(Box::new(|info| {
        eprintln!("Sentinel panic occurred:");
        eprintln!("{}", info);

        // Log panic to file
        let panic_log = format!(
            "Panic at {:?}\n{:?}\n",
            std::time::SystemTime::now(),
            info
        );

        let log_path = std::env::temp_dir().join("sentinel_panic.log");

        if let Err(e) = std::fs::write(&log_path, panic_log) {
            eprintln!("Warning: Failed to write panic log: {}", e);
        }
    }));
}
