fn main() {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    match tme_sim::run_cli_with_io(std::env::args(), stdin.lock(), stdout.lock()) {
        Ok(()) => {}
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
