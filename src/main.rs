fn main() {
    if let Err(err) = polypty::runtime::run() {
        eprintln!("polypty: {err:#}");
        std::process::exit(1);
    }
}
