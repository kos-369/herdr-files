mod location;
mod score;
mod walk;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use serde_json::{Value, json};

fn cli() -> Command {
    Command::new("herdr-files")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Searches a folder for the herdr-files mod and prints JSON")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("files")
                .about("Finds files by path, best match first")
                .arg(Arg::new("query").required(true).help(
                    "Words to look for in file paths, as in fzf. \
                             A trailing :LINE or :LINE:COLUMN is printed back as line and column",
                ))
                .arg(
                    Arg::new("cwd")
                        .long("cwd")
                        .value_parser(value_parser!(PathBuf))
                        .help("Folder to search [default: the current folder]"),
                )
                .arg(
                    Arg::new("exclude")
                        .long("exclude")
                        .value_name("PATTERN")
                        .action(ArgAction::Append)
                        .help(
                            "Leave out what matches this gitignore-style pattern; repeat for more",
                        ),
                )
                .arg(
                    Arg::new("limit")
                        .long("limit")
                        .value_name("N")
                        .value_parser(value_parser!(usize))
                        .default_value("200")
                        .help("Print at most this many results"),
                ),
        )
}

fn main() -> ExitCode {
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
    let excludes: Vec<String> = args
        .get_many::<String>("exclude")
        .unwrap_or_default()
        .cloned()
        .collect();
    let limit = *args
        .get_one::<usize>("limit")
        .expect("--limit has a default");
    if !cwd.is_dir() {
        return Err(format!("{}: not a folder", cwd.display()));
    }
    let paths = walk::files(&cwd, &excludes)?;
    let location = location::split(query);
    let ranked = score::rank(location.query, paths, limit);
    let results: Vec<Value> = (ranked.hits.iter())
        .map(|hit| json!({ "path": hit.path, "score": hit.score }))
        .collect();
    Ok(json!({
        "results": results,
        "truncated": ranked.truncated,
        "line": location.line,
        "column": location.column,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_is_valid() {
        cli().debug_assert();
    }
}
