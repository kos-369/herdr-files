//! The search engine behind the herdr-files mod. The mod runs it once per
//! query. It prints one JSON document to stdout, or a message to stderr and
//! a non-zero exit code.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Arg, ArgMatches, Command, value_parser};
use serde_json::{Value, json};

/// The command line: one subcommand per kind of search.
fn cli() -> Command {
    Command::new("herdr-files")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Searches a folder for the herdr-files mod and prints JSON")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("files")
                .about("Finds files by name")
                .arg(
                    Arg::new("query")
                        .required(true)
                        .help("Text to look for in file names"),
                )
                .arg(
                    Arg::new("cwd")
                        .long("cwd")
                        .value_parser(value_parser!(PathBuf))
                        .help("Folder to search [default: the current folder]"),
                ),
        )
}

fn main() -> ExitCode {
    // Usage errors print clap's message to stderr and exit with code 2.
    let matches = cli().get_matches();
    let result = match matches.subcommand() {
        Some(("files", args)) => run_files(args),
        _ => unreachable!("clap requires a known subcommand"),
    };
    match result {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("herdr-files: {message}");
            ExitCode::FAILURE
        }
    }
}

/// `files`: reads the arguments and checks the folder before searching it.
fn run_files(args: &ArgMatches) -> Result<Value, String> {
    let cwd = match args.get_one::<PathBuf>("cwd") {
        Some(cwd) => cwd.clone(),
        None => {
            std::env::current_dir().map_err(|e| format!("cannot read the current folder: {e}"))?
        }
    };
    let query = args
        .get_one::<String>("query")
        .expect("clap requires the query");
    if !cwd.is_dir() {
        return Err(format!("{}: not a folder", cwd.display()));
    }
    Ok(files(&cwd, query))
}

/// Finds files under `root` whose names match `query`. Walking and scoring
/// come in a later change, so for now the list is always empty.
fn files(_root: &Path, _query: &str) -> Value {
    json!({ "results": [], "truncated": false })
}

#[cfg(test)]
mod tests {
    use super::*;

    // clap's own checks of the command definition (duplicate names, bad settings).
    #[test]
    fn cli_is_valid() {
        cli().debug_assert();
    }
}
