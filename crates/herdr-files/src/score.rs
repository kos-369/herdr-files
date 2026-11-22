use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

#[derive(Debug, PartialEq)]
pub struct Hit {
    pub path: String,
    pub score: u32,
}

pub struct Ranked {
    pub hits: Vec<Hit>,
    pub truncated: bool,
}

pub fn rank(query: &str, paths: Vec<String>, limit: usize) -> Ranked {
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
    let mut buf = Vec::new();
    let mut hits: Vec<Hit> = paths
        .into_iter()
        .filter_map(|path| {
            let score = pattern.score(Utf32Str::new(&path, &mut buf), &mut matcher)?;
            Some(Hit { path, score })
        })
        .collect();
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

    fn ranked(query: &str, paths: &[&str]) -> Vec<String> {
        let paths = paths.iter().map(|p| p.to_string()).collect();
        let ranked = rank(query, paths, usize::MAX);
        ranked.hits.into_iter().map(|hit| hit.path).collect()
    }

    #[test]
    fn a_match_at_the_start_of_a_name_comes_first() {
        let paths = ["docs/domain.md", "crates/herdr-files/src/main.rs"];
        assert_eq!(
            ranked("main", &paths),
            ["crates/herdr-files/src/main.rs", "docs/domain.md"]
        );
    }

    #[test]
    fn a_name_in_a_folder_scores_as_high_as_one_at_the_top() {
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
