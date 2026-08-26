use std::process::ExitCode;

fn main() -> ExitCode {
    tme_authoring::cli::run(std::env::args().skip(1).collect())
}
