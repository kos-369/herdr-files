mod grep;
mod location;
mod score;
mod walk;

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use serde_json::{Value, json};

fn cli() -> Command {
    Command::new("herdr-files")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Searches a folder for the herdr-files mod and prints JSON")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(folder_args(
            Command::new("files")
                .about("Finds files by path, best match first")
                .arg(Arg::new("query").required(true).help(
                    "Words to look for in file paths, as in fzf. \
                             A trailing :LINE or :LINE:COLUMN is printed back as line and column",
                )),
        ))
        .subcommand(folder_args(
            Command::new("grep")
                .about("Finds lines that contain the pattern, going through folders in name order")
                .arg(
                    Arg::new("pattern")
                        .required(true)
                        .help("Text to look for in files"),
                )
                .arg(
                    Arg::new("regex")
                        .long("regex")
                        .action(ArgAction::SetTrue)
                        .help("Read the pattern as a regular expression"),
                )
                .arg(
                    Arg::new("case-sensitive")
                        .long("case-sensitive")
                        .action(ArgAction::SetTrue)
                        .help("Tell upper and lower case apart"),
                )
                .arg(
                    Arg::new("timeout")
                        .long("timeout")
                        .value_name("MS")
                        .value_parser(value_parser!(u64))
                        .help("Stop after this many milliseconds and print what was found"),
                ),
        ))
}

fn folder_args(command: Command) -> Command {
    command
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
                .help("Leave out what matches this gitignore-style pattern; repeat for more"),
        )
        .arg(
            Arg::new("limit")
                .long("limit")
                .value_name("N")
                .value_parser(value_parser!(usize))
                .default_value("200")
                .help("Print at most this many results"),
        )
}

fn main() -> ExitCode {
    let matches = cli().get_matches();
    let result = match matches.subcommand() {
        Some(("files", args)) => run_files(args),
        Some(("grep", args)) => run_grep(args),
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

struct Folder {
    cwd: PathBuf,
    excludes: Vec<String>,
    limit: usize,
}

fn folder(args: &ArgMatches) -> Result<Folder, String> {
    let cwd = match args.get_one::<PathBuf>("cwd") {
        Some(cwd) => cwd.clone(),
        None => {
            std::env::current_dir().map_err(|e| format!("cannot read the current folder: {e}"))?
        }
    };
    if !cwd.is_dir() {
        return Err(format!("{}: not a folder", cwd.display()));
    }
    let excludes = (args.get_many::<String>("exclude"))
        .unwrap_or_default()
        .cloned()
        .collect();
    let limit = *args
        .get_one::<usize>("limit")
        .expect("--limit has a default");
    Ok(Folder {
        cwd,
        excludes,
        limit,
    })
}

fn run_files(args: &ArgMatches) -> Result<Value, String> {
    let query = args
        .get_one::<String>("query")
        .expect("clap requires the query");
    let folder = folder(args)?;
    let paths = walk::files(&folder.cwd, &folder.excludes)?;
    let location = location::split(query);
    let ranked = score::rank(location.query, paths, folder.limit);
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

fn run_grep(args: &ArgMatches) -> Result<Value, String> {
    let pattern = args
        .get_one::<String>("pattern")
        .expect("clap requires the pattern");
    let deadline =
        (args.get_one::<u64>("timeout")).map(|ms| Instant::now() + Duration::from_millis(*ms));
    let matcher = grep::matcher(
        pattern,
        args.get_flag("regex"),
        args.get_flag("case-sensitive"),
    )?;
    let folder = folder(args)?;
    let mut results = Vec::new();
    let mut timed_out = false;
    walk::each_file(&folder.cwd, &folder.excludes, |path| {
        let room = folder.limit + 1 - results.len();
        if room == 0 {
            return false;
        }
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            timed_out = true;
            return false;
        }
        let Ok(content) = std::fs::read(folder.cwd.join(&path)) else {
            return true;
        };
        for hit in grep::search(&content, &matcher, room) {
            results.push(json!({
                "path": path,
                "line": hit.line,
                "column": hit.column,
                "text": hit.text,
            }));
        }
        true
    })?;
    let truncated = results.len() > folder.limit;
    results.truncate(folder.limit);
    Ok(json!({
        "results": results,
        "truncated": truncated,
        "timed_out": timed_out,
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
