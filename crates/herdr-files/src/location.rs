#[derive(Debug, PartialEq)]
pub struct Location<'a> {
    pub query: &'a str,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

pub fn split(text: &str) -> Location<'_> {
    let mut query = text.strip_suffix(':').unwrap_or(text);
    let mut numbers = Vec::new();
    while numbers.len() < 2 {
        let Some((head, tail)) = query.rsplit_once(':') else {
            break;
        };
        let Some(number) = number(tail) else {
            break;
        };
        numbers.insert(0, number);
        query = head;
    }
    Location {
        query,
        line: numbers.first().copied(),
        column: numbers.get(1).copied(),
    }
}

fn number(text: &str) -> Option<u32> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn location(query: &str, line: Option<u32>, column: Option<u32>) -> Location<'_> {
        Location {
            query,
            line,
            column,
        }
    }

    #[test]
    fn a_path_without_numbers_stays_as_it_is() {
        assert_eq!(split("src/main.rs"), location("src/main.rs", None, None));
    }

    #[test]
    fn a_number_after_a_colon_is_the_line() {
        assert_eq!(
            split("src/main.rs:42"),
            location("src/main.rs", Some(42), None)
        );
    }

    #[test]
    fn a_second_number_is_the_column() {
        assert_eq!(
            split("src/main.rs:42:5"),
            location("src/main.rs", Some(42), Some(5))
        );
    }

    #[test]
    fn a_trailing_colon_is_dropped() {
        assert_eq!(split("src/main.rs:"), location("src/main.rs", None, None));
        assert_eq!(
            split("src/main.rs:42:5:"),
            location("src/main.rs", Some(42), Some(5))
        );
    }

    #[test]
    fn only_the_last_two_numbers_count() {
        assert_eq!(split("a:1:2:3"), location("a:1", Some(2), Some(3)));
    }

    #[test]
    fn text_that_isnt_a_number_stays_in_the_query() {
        assert_eq!(split("notes:draft"), location("notes:draft", None, None));
        assert_eq!(split("a:4x"), location("a:4x", None, None));
        assert_eq!(split("a:+4"), location("a:+4", None, None));
    }

    #[test]
    fn a_number_too_big_for_a_line_stays_in_the_query() {
        assert_eq!(
            split("a:99999999999"),
            location("a:99999999999", None, None)
        );
    }

    #[test]
    fn words_keep_their_spaces() {
        assert_eq!(split("src main:42"), location("src main", Some(42), None));
    }
}
