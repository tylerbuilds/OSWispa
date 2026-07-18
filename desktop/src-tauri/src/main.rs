#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if oswispa_desktop::run().is_err() {
        eprintln!("OSWispa desktop could not start");
        std::process::exit(1);
    }
}
