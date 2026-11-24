use regex::bytes::{Regex, RegexBuilder};

#[derive(Debug, PartialEq)]
pub struct Hit {
    pub line: usize,
    pub column: usize,
    pub text: String,
}

pub fn matcher(pattern: &str, regex: bool, case_sensitive: bool) -> Result<Regex, String> {
    let pattern = if regex {
        pattern.to_owned()
    } else {
        regex::escape(pattern)
    };
    RegexBuilder::new(&pattern)
        .case_insensitive(!case_sensitive)
        .multi_line(true)
        .crlf(true)
        .build()
        .map_err(|e| format!("bad pattern: {e}"))
}

pub fn search(content: &[u8], matcher: &Regex, limit: usize) -> Vec<Hit> {
    let mut hits = Vec::new();
    let mut line = 1;
    let mut from = 0;
    while hits.len() < limit && from <= content.len() {
        let Some(found) = matcher.find_at(content, from) else {
            break;
        };
        let start = (content[from..found.start()].iter())
            .rposition(|&b| b == b'\n')
            .map_or(from, |i| from + i + 1);
        if start == content.len() {
            break;
        }
        let end = (content[start..].iter())
            .position(|&b| b == b'\n')
            .map_or(content.len(), |i| start + i);
        let number = line + newlines(&content[from..start]);
        let text = &content[start..end];
        hits.push(Hit {
            line: number,
            column: String::from_utf8_lossy(&content[start..found.start()])
                .chars()
                .count()
                + 1,
            text: String::from_utf8_lossy(text.strip_suffix(b"\r").unwrap_or(text)).into_owned(),
        });
        line = number + 1;
        from = end + 1;
    }
    hits
}

fn newlines(bytes: &[u8]) -> usize {
    bytes.iter().filter(|&&b| b == b'\n').count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(content: &str, pattern: &str, regex: bool, case_sensitive: bool) -> Vec<Hit> {
        let matcher = matcher(pattern, regex, case_sensitive).expect("the pattern is valid");
        search(content.as_bytes(), &matcher, 100)
    }

    fn literal(content: &str, pattern: &str) -> Vec<Hit> {
        found(content, pattern, false, false)
    }

    fn lines(hits: &[Hit]) -> Vec<usize> {
        hits.iter().map(|hit| hit.line).collect()
    }

    fn hit(line: usize, column: usize, text: &str) -> Hit {
        Hit {
            line,
            column,
            text: text.to_owned(),
        }
    }

    #[test]
    fn a_hit_has_the_line_column_and_text() {
        let content = "fn main() {\n    println!(\"hi\");\n}\n";
        assert_eq!(
            literal(content, "println"),
            [hit(2, 5, "    println!(\"hi\");")]
        );
    }

    #[test]
    fn a_line_with_several_matches_is_one_hit_at_the_first() {
        assert_eq!(literal("a b a b\n", "b"), [hit(1, 3, "a b a b")]);
    }

    #[test]
    fn hits_come_in_line_order() {
        assert_eq!(lines(&literal("x\ny\nx\n\nx", "x")), [1, 3, 5]);
    }

    #[test]
    fn a_literal_pattern_keeps_regex_characters() {
        assert_eq!(lines(&literal("axb\na.b\n", "a.b")), [2]);
    }

    #[test]
    fn a_regex_pattern_is_read_as_a_regex() {
        assert_eq!(lines(&found("axb\na.b\nab\n", "a.b", true, false)), [1, 2]);
    }

    #[test]
    fn case_is_ignored_unless_asked() {
        assert_eq!(lines(&found("Main\nmain\n", "main", false, false)), [1, 2]);
        assert_eq!(lines(&found("Main\nmain\n", "main", false, true)), [2]);
    }

    #[test]
    fn line_anchors_match_at_every_line() {
        assert_eq!(lines(&found("ab\nba\nb\r\n", "^b", true, false)), [2, 3]);
        assert_eq!(lines(&found("ab\nba\nb\r\n", "b$", true, false)), [1, 3]);
    }

    #[test]
    fn crlf_endings_stay_out_of_the_text() {
        assert_eq!(literal("a\r\nb\r\n", "b"), [hit(2, 1, "b")]);
    }

    #[test]
    fn the_column_counts_characters() {
        assert_eq!(literal("日本語 main\n", "main"), [hit(1, 5, "日本語 main")]);
    }

    #[test]
    fn empty_matches_find_only_real_lines() {
        assert_eq!(lines(&found("a\nb\n", "x*", true, false)), [1, 2]);
        assert!(found("a\n", "^$", true, false).is_empty());
        assert!(found("", "x*", true, false).is_empty());
    }

    #[test]
    fn invalid_bytes_still_give_a_hit() {
        let matcher = matcher("main", false, false).unwrap();
        let hits = search(b"\xff main\n", &matcher, 100);
        assert_eq!(hits, [hit(1, 3, "\u{fffd} main")]);
    }

    #[test]
    fn search_stops_at_the_limit() {
        let matcher = matcher("x", false, false).unwrap();
        assert_eq!(lines(&search(b"x\nx\nx\n", &matcher, 2)), [1, 2]);
    }

    #[test]
    fn a_bad_regex_is_an_error() {
        let error = matcher("a(", true, false).unwrap_err();
        assert!(error.starts_with("bad pattern:"), "error: {error}");
    }
}
