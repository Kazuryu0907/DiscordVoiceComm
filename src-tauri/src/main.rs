// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{fs::File, sync::Arc};

use gag::Redirect;
use tracing::Level;
fn main() {
    let fs = File::create("./logfile.log")
        .unwrap_or_else(|e| {
            eprintln!("Warning: Could not create logfile.log: {}", e);
            std::io::stdout() // fallback to stdout
        });
    tracing_subscriber::fmt()
        .with_max_level(Level::ERROR)
        .with_ansi(false)
        .with_writer(Arc::new(fs))
        .init();
    let fs_stderr = File::create("./stderr.log")
        .unwrap_or_else(|e| {
            eprintln!("Warning: Could not create stderr.log: {}", e);
            std::io::stderr() // fallback to stderr
        });
    if let Err(e) = Redirect::stderr(fs_stderr) {
        eprintln!("Warning: Could not redirect stderr: {}", e);
    }
    discordvoicecommv1_lib::run()
}
