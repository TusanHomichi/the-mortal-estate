//! The compiler's command line, including the Workbench's one bridge into it.
//!
//! Four of these subcommands exist so that a tool outside this crate can reach
//! the compiler's semantics without reimplementing them. They are machine entry
//! points: each writes ONE JSON document to standard output and says yes or no
//! with its exit code. There is no human-readable variant, because a second
//! output format is a second thing to keep true.
//!
//! | Exit | Meaning |
//! | --- | --- |
//! | 0 | the answer is yes: the candidate is accepted, the replay produced one |
//! | 1 | the answer is no: rejected or refused, with the reason on stdout |
//! | 2 | the request could not be understood or its inputs could not be read |
//!
//! **None of them grants authority.** `validate-candidate` reads no receipt and
//! consults no reviewed digest. `replay` reads the accepted master as bytes,
//! writes a candidate wherever its caller says, and produces nothing
//! [`crate::promotion::load`] would accept. Only `--check` and `--report` touch
//! tracked output, and they are the promoted path exactly as before.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde_json::{Value, json};

use crate::Result;
use crate::contract::{self, LandContract, MemberContract};
use crate::{BuildMode, build, candidate, compile, emit, export, operations, replay};

const CANDIDATE_NAME: &str = "candidate-member.tmj";
const CANDIDATE_PROJECTION_NAME: &str = "candidate_projection.json";

const USAGE: &str = "\
usage: tme-authoring [--check] [--report]
       tme-authoring describe-operations --land <id> [--member <id>]
       tme-authoring validate-candidate --land <id> [--member <id>] <document.json>
       tme-authoring project-candidate --land <id> [--member <id>] <document.json>
       tme-authoring replay --land <id> [--member <id>]
                            --operations <operations.json> --output-dir <directory>
                            --expect-base-sha256 <hex> [--validate] [--project]

The first form compiles every authored land. Every other subcommand addresses
ONE land, named explicitly: there is no default land, because a default is how a
tool edits a document nobody asked about. `--member` defaults to the land's one
candidate entry point and is required only where a land declares more than one.

Every subcommand but the first writes one JSON document to stdout.
Exit 0 yes, 1 no (the reason is on stdout), 2 the request could not be read.";

pub const EXIT_NO: u8 = 1;
pub const EXIT_UNREADABLE: u8 = 2;

pub fn run(arguments: Vec<String>) -> ExitCode {
    match arguments.first().map(String::as_str) {
        Some("describe-operations") => describe_operations(&arguments[1..]),
        Some("validate-candidate") => document_command(&arguments[1..], validate_candidate),
        Some("project-candidate") => document_command(&arguments[1..], project_candidate),
        Some("replay") => answer(replay_command(&arguments[1..])),
        _ => build_command(arguments),
    }
}

/// The land and member a subcommand addresses, resolved from explicit flags.
struct Target {
    land: &'static LandContract,
    member: &'static MemberContract,
}

/// Pull `--land` and `--member` out of an argument list, leaving the rest.
///
/// The land is required and never defaulted. The member defaults to the land's
/// single candidate entry point, which is a derivation rather than a guess: a
/// land declaring two would refuse to answer instead of picking one.
fn take_target(arguments: &[String]) -> Result<(Target, Vec<String>)> {
    let mut land_id: Option<String> = None;
    let mut member_id: Option<String> = None;
    let mut rest = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        let argument = arguments[index].as_str();
        let value = |name: &str| {
            arguments
                .get(index + 1)
                .cloned()
                .ok_or_else(|| format!("{name} needs a value"))
        };
        match argument {
            "--land" => {
                land_id = Some(value("--land")?);
                index += 2;
            }
            "--member" => {
                member_id = Some(value("--member")?);
                index += 2;
            }
            other => {
                rest.push(other.to_owned());
                index += 1;
            }
        }
    }
    let land = contract::land(&land_id.ok_or("this subcommand needs --land <id>")?)?;
    let member = match member_id {
        Some(id) => land.member(&id)?,
        None => land.candidate_member()?,
    };
    Ok((Target { land, member }, rest))
}

// ---------------------------------------------------------------------------
// The build entry point — unchanged, and still the only one that writes tracked
// bytes
// ---------------------------------------------------------------------------

fn build_command(arguments: Vec<String>) -> ExitCode {
    let mut mode = BuildMode {
        check: false,
        report: false,
    };
    for argument in arguments {
        match argument.as_str() {
            "--check" => mode.check = true,
            "--report" => mode.report = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            _ => {
                eprintln!("unknown argument: {argument}\n{USAGE}");
                return ExitCode::from(EXIT_UNREADABLE);
            }
        }
    }
    match crate::repository_root().and_then(|root| build(&root, mode)) {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("authored lands: FAIL\n{error}");
            ExitCode::FAILURE
        }
    }
}

// ---------------------------------------------------------------------------
// The Workbench bridge
// ---------------------------------------------------------------------------

/// A verdict: the document to print, and whether the answer was yes.
struct Verdict {
    document: Value,
    yes: bool,
}

fn answer(outcome: Result<Verdict>) -> ExitCode {
    match outcome {
        Ok(verdict) => {
            print(&verdict.document);
            if verdict.yes {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(EXIT_NO)
            }
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(EXIT_UNREADABLE)
        }
    }
}

fn print(document: &Value) {
    match emit::json(document) {
        Ok(bytes) => print!("{}", String::from_utf8_lossy(&bytes)),
        Err(error) => eprintln!("the answer could not be serialized: {error}"),
    }
}

fn describe_operations(arguments: &[String]) -> ExitCode {
    let (target, rest) = match take_target(arguments) {
        Ok(resolved) => resolved,
        Err(error) => {
            eprintln!("{error}\n{USAGE}");
            return ExitCode::from(EXIT_UNREADABLE);
        }
    };
    if !rest.is_empty() {
        eprintln!("unexpected argument: {}\n{USAGE}", rest[0]);
        return ExitCode::from(EXIT_UNREADABLE);
    }
    print(&operations::vocabulary_document(target.land, target.member));
    ExitCode::SUCCESS
}

fn document_command(
    arguments: &[String],
    handler: impl Fn(&Target, &Value, &Path) -> Result<Verdict>,
) -> ExitCode {
    let (target, rest) = match take_target(arguments) {
        Ok(resolved) => resolved,
        Err(error) => {
            eprintln!("{error}\n{USAGE}");
            return ExitCode::from(EXIT_UNREADABLE);
        }
    };
    let [path] = rest.as_slice() else {
        eprintln!("expected exactly one document path\n{USAGE}");
        return ExitCode::from(EXIT_UNREADABLE);
    };
    let path = PathBuf::from(path);
    let outcome = read_document(&path).and_then(|document| handler(&target, &document, &path));
    answer(outcome)
}

fn read_document(path: &Path) -> Result<Value> {
    let bytes = emit::read(path)?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))
}

fn validate_candidate(target: &Target, document: &Value, _path: &Path) -> Result<Verdict> {
    let report = candidate::validate_candidate(target.land.id, target.member, document)?;
    Ok(Verdict {
        yes: report.accepted,
        document: serde_json::to_value(&report).map_err(|error| error.to_string())?,
    })
}

/// The candidate's logical view, for a preview that shows what Apply would
/// produce rather than an approximation of it.
fn project_candidate(target: &Target, document: &Value, path: &Path) -> Result<Verdict> {
    let digest = emit::digest(&emit::json(document)?);
    match compile::compile_member(target.member, document) {
        Ok(member) => {
            let projection = export::candidate_document(
                target.land,
                &member,
                &path.display().to_string(),
                &digest,
            );
            Ok(Verdict {
                yes: true,
                document: serde_json::to_value(&projection).map_err(|error| error.to_string())?,
            })
        }
        // A candidate that does not compile has no logical view to show. Saying
        // so in the same shape as the validator does keeps one refusal format.
        Err(diagnostic) => Ok(Verdict {
            yes: false,
            document: json!({
                "schema_version": 1,
                "kind": "workbench_candidate_projection_refused",
                "candidate_sha256": digest,
                "diagnostics": [diagnostic],
            }),
        }),
    }
}

struct ReplayRequest {
    target: Target,
    operations: PathBuf,
    output_directory: PathBuf,
    expect_base_sha256: String,
    validate: bool,
    project: bool,
}

fn parse_replay(arguments: &[String]) -> Result<ReplayRequest> {
    let (target, arguments) = take_target(arguments)?;
    let arguments = arguments.as_slice();
    let mut operations = None;
    let mut output_directory = None;
    let mut expect_base_sha256 = None;
    let mut validate = false;
    let mut project = false;
    let mut index = 0;
    while index < arguments.len() {
        let argument = arguments[index].as_str();
        let value = || {
            arguments
                .get(index + 1)
                .cloned()
                .ok_or_else(|| format!("{argument} needs a value"))
        };
        match argument {
            "--operations" => {
                operations = Some(PathBuf::from(value()?));
                index += 2;
            }
            "--output-dir" => {
                output_directory = Some(PathBuf::from(value()?));
                index += 2;
            }
            "--expect-base-sha256" => {
                expect_base_sha256 = Some(value()?);
                index += 2;
            }
            "--validate" => {
                validate = true;
                index += 1;
            }
            "--project" => {
                project = true;
                index += 1;
            }
            other => return Err(format!("unknown argument: {other}\n{USAGE}")),
        }
    }
    Ok(ReplayRequest {
        target,
        operations: operations.ok_or("replay needs --operations")?,
        output_directory: output_directory.ok_or("replay needs --output-dir")?,
        // Required, never defaulted. Replaying against bytes the caller did not
        // expect is how a stale session quietly edits a document it never saw.
        expect_base_sha256: expect_base_sha256.ok_or("replay needs --expect-base-sha256")?,
        validate,
        project,
    })
}

/// A path as this repository names it, when it is inside this repository.
///
/// The candidate lands in a session directory the caller named, which is
/// ordinarily an absolute path. Everything else in the Workbench addresses
/// files repository-relatively, and one document carrying both forms is one
/// document a consumer has to branch on.
fn addressed(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn refusal(stage: &str, error: String) -> Verdict {
    Verdict {
        yes: false,
        document: json!({
            "schema_version": 1,
            "kind": "workbench_replay_refused",
            "stage": stage,
            "error": error,
        }),
    }
}

fn replay_command(arguments: &[String]) -> Result<Verdict> {
    let request = parse_replay(arguments)?;
    let root = crate::repository_root()?;
    let base_path = request.target.member.document;
    let master = root.join(base_path);
    let base_bytes = emit::read(&master)?;
    let base_sha256 = emit::digest(&base_bytes);
    if base_sha256 != request.expect_base_sha256 {
        return Ok(refusal(
            "base",
            format!(
                "{base_path} holds {base_sha256}; the caller expected {}",
                request.expect_base_sha256
            ),
        ));
    }

    let set: operations::OperationSet = {
        let bytes = emit::read(&request.operations)?;
        serde_json::from_slice(&bytes)
            .map_err(|error| format!("{}: {error}", request.operations.display()))?
    };
    if set.schema_version != operations::SCHEMA_VERSION
        || set.kind != operations::OPERATION_SET_KIND
    {
        return Err(format!(
            "{} is not a version {} {}",
            request.operations.display(),
            operations::SCHEMA_VERSION,
            operations::OPERATION_SET_KIND
        ));
    }

    let mut document: Value = serde_json::from_slice(&base_bytes)
        .map_err(|error| format!("{}: {error}", master.display()))?;
    if let Err(error) = replay::replay(request.target.member, &mut document, &set.operations) {
        return Ok(refusal("replay", error));
    }

    let candidate_bytes = emit::json(&document)?;
    let candidate_sha256 = emit::digest(&candidate_bytes);
    let candidate_path = request.output_directory.join(CANDIDATE_NAME);
    std::fs::create_dir_all(&request.output_directory)
        .map_err(|error| format!("create {}: {error}", request.output_directory.display()))?;
    emit::write_or_check(&candidate_path, &candidate_bytes, false)?;

    let mut result = json!({
        "schema_version": 1,
        "kind": "workbench_replay_result",
        "land": request.target.land.id,
        "member": request.target.member.id,
        "base": {"path": base_path, "sha256": base_sha256},
        "candidate": {
            "path": addressed(&root, &candidate_path),
            "sha256": candidate_sha256,
        },
        "applied": set.operations.iter().map(|operation| json!({
            "record_id": operation.record_id,
            "author": operation.author,
            "verb": operation.verb,
        })).collect::<Vec<_>>(),
        "report": Value::Null,
        "projection": Value::Null,
    });

    let mut yes = true;
    if request.validate {
        let report = candidate::validate_candidate(
            request.target.land.id,
            request.target.member,
            &document,
        )?;
        yes = report.accepted;
        result["report"] = serde_json::to_value(&report).map_err(|error| error.to_string())?;
    }
    if request.project && yes {
        let member = match compile::compile_member(request.target.member, &document) {
            Ok(member) => member,
            // Reachable only when --project was asked for without --validate.
            // A candidate that does not compile has no view; the caller learns
            // that here rather than from a missing file.
            Err(diagnostic) => return Ok(refusal("project", diagnostic)),
        };
        let projection = export::candidate_document(
            request.target.land,
            &member,
            &addressed(&root, &candidate_path),
            &candidate_sha256,
        );
        let bytes = emit::json(&projection)?;
        let path = request.output_directory.join(CANDIDATE_PROJECTION_NAME);
        emit::write_or_check(&path, &bytes, false)?;
        result["projection"] = json!({
            "path": addressed(&root, &path),
            "sha256": emit::digest(&bytes),
        });
    }
    Ok(Verdict {
        document: result,
        yes,
    })
}
