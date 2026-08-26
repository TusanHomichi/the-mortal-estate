use std::io::Read;
use std::path::Path;

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let mut args = std::env::args();
    let _program = args.next();
    let Some(path) = args.next() else {
        eprintln!("usage: tme-validate-trace <trace.json|->");
        return 2;
    };
    if args.next().is_some() {
        eprintln!("usage: tme-validate-trace <trace.json|->");
        return 2;
    }

    let input = if path == "-" {
        let mut input = String::new();
        if let Err(error) = std::io::stdin().read_to_string(&mut input) {
            eprintln!("ERROR $: failed to read stdin: {error}");
            return 1;
        }
        input
    } else {
        match std::fs::read_to_string(Path::new(&path)) {
            Ok(input) => input,
            Err(error) => {
                eprintln!("ERROR $: failed to read {path}: {error}");
                return 1;
            }
        }
    };

    match tme_sim::validate_trace_json(&input) {
        Ok(report) => {
            println!(
                "OK: {} steps, trace V{} consistent",
                report.step_count, report.contract_version
            );
            0
        }
        Err(errors) => {
            for error in &errors {
                eprintln!("ERROR {error}");
            }
            eprintln!("FAIL: {} consistency error(s)", errors.len());
            1
        }
    }
}
