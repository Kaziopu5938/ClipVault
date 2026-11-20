#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|arg| arg == "--selftest") {
        let ok = clipvault_lib::selftest::run();
        std::process::exit(if ok { 0 } else { 1 });
    }
    clipvault_lib::run()
}
