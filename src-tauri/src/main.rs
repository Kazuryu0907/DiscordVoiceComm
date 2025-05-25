// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{fs::File, sync::Arc};

use gag::Redirect;
use tracing::Level;
fn main() {
    let fs = File::create("./logfile.log").unwrap();
    tracing_subscriber::fmt()
        .with_max_level(Level::ERROR)
        .with_ansi(false)
        .with_writer(Arc::new(fs))
        .init();
    // simple_logging::log_to_file("./logfile.log", LevelFilter::Debug).unwrap();
    let fs_stderr = File::create("./stderr.log").unwrap();
    let _redirect = Redirect::stderr(fs_stderr).expect("Failed to redirect stderr");
    let fs_stdout = File::create("./stdout.log").unwrap();
    let _redirect2 = Redirect::stdout(fs_stdout).expect("Failed to redirect stdout");
    discordvoicecommv1_lib::run()
}
