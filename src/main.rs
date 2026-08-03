fn main() {
    if let Err(err) = mux::runtime::run() {
        eprintln!("mux: {err:#}");
        std::process::exit(1);
    }
}
