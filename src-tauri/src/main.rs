// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{fs::File, sync::Arc};

use log::LevelFilter;
use tracing::Level;
fn main() {
    let fs = File::create("./logfile.log").unwrap();
    tracing_subscriber::fmt()
        .with_max_level(Level::ERROR)
        .with_ansi(false)
        .with_writer(Arc::new(fs))
        .init();
    // simple_logging::log_to_file("./logfile.log", LevelFilter::Debug).unwrap();
    discordvoicecommv1_lib::run()
}
