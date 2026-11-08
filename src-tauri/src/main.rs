// No console window next to the app in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // `Achaja.exe <command>`: the tools of the voice session and its hooks, without a window
    if let Some(code) = achaja_core::cli::try_run(&args) {
        std::process::exit(code);
    }
    achaja_lib::run(&args);
}
