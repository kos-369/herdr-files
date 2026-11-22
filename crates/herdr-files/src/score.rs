//! Matches paths against a query and ranks them: the logic half of `files`,
//! apart from the walk so it can be tested without files.

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

/// A path that matched the query. A higher score is a better match.
#[derive(Debug, PartialEq)]
pub struct Hit {
    pub path: String,
    pub score: u32,
}

/// The best hits, and whether more paths matched than the limit allowed.
pub struct Ranked {
    pub hits: Vec<Hit>,
    pub truncated: bool,
}

/// Returns the paths that match `query`, best first, at most `limit` of them.
///
/// The query works like fzf: every word must match, and `'`, `^`, `$` and `!`
/// change how a word matches. A word in lowercase ignores case; a word with
/// an uppercase letter doesn't. An empty query matches every path.
pub fn rank(query: &str, paths: Vec<String>, limit: usize) -> Ranked {
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    // The path settings score a match right after `/` higher, such as the
    // start of a file name.
    let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
    let mut buf = Vec::new();
    let mut hits: Vec<Hit> = paths
        .into_iter()
        .filter_map(|path| {
            let score = pattern.score(Utf32Str::new(&path, &mut buf), &mut matcher)?;
            Some(Hit { path, score })
        })
        .collect();
    // Equal scores go shorter path first, then by name, so the order never
    // depends on how the folder was walked.
    hits.sort_by(|a, b| {
        (b.score.cmp(&a.score))
            .then(a.path.len().cmp(&b.path.len()))
            .then_with(|| a.path.cmp(&b.path))
    });
    let truncated = hits.len() > limit;
    hits.truncate(limit);
    Ranked { hits, truncated }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The matching paths for `query`, in order, with no limit.
    fn ranked(query: &str, paths: &[&str]) -> Vec<String> {
        let paths = paths.iter().map(|p| p.to_string()).collect();
        let ranked = rank(query, paths, usize::MAX);
        ranked.hits.into_iter().map(|hit| hit.path).collect()
    }

    #[test]
    fn a_match_at_the_start_of_a_name_comes_first() {
        // "domain" contains "main" too, and its path is shorter.
        let paths = ["docs/domain.md", "crates/herdr-files/src/main.rs"];
        assert_eq!(
            ranked("main", &paths),
            ["crates/herdr-files/src/main.rs", "docs/domain.md"]
        );
    }

    #[test]
    fn a_name_in_a_folder_scores_as_high_as_one_at_the_top() {
        // Without the path settings, a match at the very start of the path
        // scores higher, so "tests/" would beat "src/test.rs".
        let paths = vec!["tests/cli.rs".to_string(), "src/test.rs".to_string()];
        let hits = rank("test", paths, usize::MAX).hits;
        assert_eq!(hits[0].score, hits[1].score);
    }

    #[test]
    fn letters_may_be_spread_out() {
        assert_eq!(
            ranked("hfmain", &["mods/herdr-files/main.js"]),
            ["mods/herdr-files/main.js"]
        );
    }

    #[test]
    fn paths_that_dont_match_are_left_out() {
        assert_eq!(
            ranked("main", &["README.md", "src/main.rs"]),
            ["src/main.rs"]
        );
        assert!(ranked("xyz", &["README.md", "src/main.rs"]).is_empty());
    }

    #[test]
    fn every_word_must_match() {
        let paths = ["src/main.rs", "docs/main.md", "src/lib.rs"];
        assert_eq!(ranked("src main", &paths), ["src/main.rs"]);
    }

    #[test]
    fn an_uppercase_letter_makes_a_word_case_sensitive() {
        let paths = ["src/main.rs", "src/Main.java"];
        assert_eq!(ranked("Main", &paths), ["src/Main.java"]);
        assert_eq!(ranked("main", &paths).len(), 2);
    }

    #[test]
    fn ties_go_to_the_shorter_path_then_by_name() {
        // An empty query gives every path the same score.
        assert_eq!(ranked("", &["b/x", "c", "a"]), ["a", "c", "b/x"]);
    }

    #[test]
    fn the_limit_cuts_the_list_and_says_so() {
        let paths = || vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let cut = rank("", paths(), 2);
        assert_eq!(cut.hits.len(), 2);
        assert!(cut.truncated);
        let all = rank("", paths(), 3);
        assert_eq!(all.hits.len(), 3);
        assert!(!all.truncated);
    }
}
