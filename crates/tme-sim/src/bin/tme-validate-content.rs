fn main() {
    let mut raw_args = std::env::args_os();
    let _program = raw_args.next();
    let mut inputs = Vec::new();
    for argument in raw_args {
        match argument.into_string() {
            Ok(argument) => inputs.push(argument),
            Err(_) => {
                eprintln!("content path arguments must be valid UTF-8");
                std::process::exit(2);
            }
        }
    }
    if inputs.is_empty() {
        eprintln!("usage: tme-validate-content <simulation-scenario>...");
        std::process::exit(2);
    }

    let report = tme_sim::validate_content_paths(inputs);
    let valid = report.results.iter().all(|result| result.valid);
    match serde_json::to_string(&report) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("failed to serialize validation result: {error}");
            std::process::exit(2);
        }
    }
    if !valid {
        std::process::exit(1);
    }
}
