// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tracing_subscriber::EnvFilter;

fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("anyflipdl=info".parse().unwrap()))
        .with_target(true)
        .with_thread_ids(true)
        .init();

    tracing::info!("AnyflipDL starting");

    anyflipdl_lib::run()
}
