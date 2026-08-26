mod cli;
mod commands;
mod fixture;
mod interactive;
mod loading;
mod render;
mod runner;
mod session;
mod trace;
mod trace_validation;
mod validator;

pub use cli::{CliAction, RunMode, RunOptions, parse_args, run, run_cli_with_io, run_from_args};
pub use interactive::run_interactive_with_io;
pub use runner::{load_engine_from_scenario, run_with_options};
pub use trace_validation::{TraceValidationError, TraceValidationReport, validate_trace_json};
pub use validator::{
    ContentValidationDiagnostic, ContentValidationInputResult, ContentValidationReport,
    validate_content_paths,
};

pub(crate) use render::render_events;
