fn main() {
    let ok = clipvault_lib::selftest::run();
    std::process::exit(if ok { 0 } else { 1 });
}
