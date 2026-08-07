use std::borrow::Cow;

use crate::Request;

/// Iterator over the percent-decoded `name=value` pairs of a request's query string.
///
/// Created by [`query_pairs`].
pub struct QueryPairs<'a> {
    remaining: &'a str,
}

/// Returns an iterator over the percent-decoded `name=value` pairs in the
/// request's query string.
///
/// Decoding follows the `application/x-www-form-urlencoded` rules that browsers
/// and HTTP clients apply to query strings: `%XX` escapes are decoded and `+` is
/// decoded as a space. Empty segments are skipped, and a segment without `=`
/// yields an empty value. Bytes that do not form valid UTF-8 after decoding are
/// replaced with U+FFFD rather than reported as an error.
///
/// No allocation happens for values that contain no `%` or `+`.
///
/// Note the `+` rule: a value that must carry a literal `+` — a timestamp with a
/// `+01:00` UTC offset, for instance — has to arrive as `%2B`, or it will decode
/// to a space.
///
/// ```
/// # fn main() {
/// # let mut request = synchttp::Request::new(Vec::new());
/// # *request.uri_mut() = "/search?q=hello+world&page=2".parse().unwrap();
/// let pairs: Vec<_> = synchttp::query_pairs(&request).collect();
/// assert_eq!(pairs[0].0, "q");
/// assert_eq!(pairs[0].1, "hello world");
/// assert_eq!(pairs[1].1, "2");
/// # }
/// ```
pub fn query_pairs(request: &Request) -> QueryPairs<'_> {
    QueryPairs {
        remaining: request.uri().query().unwrap_or(""),
    }
}

/// Returns the percent-decoded value of the first query parameter named `name`.
///
/// Decoding rules are the same as [`query_pairs`]. Returns `None` when the
/// request has no query string or no parameter with that name; returns
/// `Some("")` for a parameter that is present but empty.
///
/// ```
/// # fn main() {
/// # let mut request = synchttp::Request::new(Vec::new());
/// # *request.uri_mut() = "/events?since=2026-02-05T12%3A34%3A56Z".parse().unwrap();
/// let since = synchttp::query_param(&request, "since").unwrap();
/// assert_eq!(since, "2026-02-05T12:34:56Z");
/// assert!(synchttp::query_param(&request, "missing").is_none());
/// # }
/// ```
pub fn query_param<'a>(request: &'a Request, name: &str) -> Option<Cow<'a, str>> {
    query_pairs(request)
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

impl<'a> Iterator for QueryPairs<'a> {
    type Item = (Cow<'a, str>, Cow<'a, str>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.remaining.is_empty() {
                return None;
            }

            let segment = match self.remaining.find('&') {
                Some(index) => {
                    let (segment, rest) = self.remaining.split_at(index);
                    self.remaining = &rest[1..];
                    segment
                }
                None => std::mem::take(&mut self.remaining),
            };

            if segment.is_empty() {
                continue;
            }

            let (name, value) = match segment.find('=') {
                Some(index) => (&segment[..index], &segment[index + 1..]),
                None => (segment, ""),
            };

            return Some((decode(name), decode(value)));
        }
    }
}

fn decode(input: &str) -> Cow<'_, str> {
    if !input.bytes().any(|byte| byte == b'%' || byte == b'+') {
        return Cow::Borrowed(input);
    }

    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            b'%' if index + 2 < bytes.len() => {
                match (hex_value(bytes[index + 1]), hex_value(bytes[index + 2])) {
                    (Some(high), Some(low)) => {
                        out.push(high << 4 | low);
                        index += 3;
                    }
                    _ => {
                        out.push(b'%');
                        index += 1;
                    }
                }
            }
            byte => {
                out.push(byte);
                index += 1;
            }
        }
    }

    Cow::Owned(match String::from_utf8(out) {
        Ok(text) => text,
        Err(error) => String::from_utf8_lossy(error.as_bytes()).into_owned(),
    })
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{query_pairs, query_param};
    use crate::Request;
    use std::borrow::Cow;

    fn request(uri: &str) -> Request {
        let mut request = Request::new(Vec::new());
        *request.uri_mut() = uri.parse().unwrap();
        request
    }

    fn pairs(uri: &str) -> Vec<(String, String)> {
        query_pairs(&request(uri))
            .map(|(name, value)| (name.into_owned(), value.into_owned()))
            .collect()
    }

    #[test]
    fn decodes_percent_encoded_value() {
        let request = request("/events?since=2026-02-05T12%3A34%3A56Z");

        assert_eq!(
            query_param(&request, "since").unwrap(),
            "2026-02-05T12:34:56Z"
        );
    }

    #[test]
    fn decodes_plus_as_space() {
        assert_eq!(pairs("/search?q=a+b"), vec![("q".into(), "a b".into())]);
    }

    #[test]
    fn borrows_when_nothing_to_decode() {
        let request = request("/search?q=plain");

        assert!(matches!(query_param(&request, "q"), Some(Cow::Borrowed(_))));
    }

    #[test]
    fn returns_none_without_query_string() {
        assert!(query_param(&request("/search"), "q").is_none());
        assert_eq!(pairs("/search"), Vec::new());
    }

    #[test]
    fn returns_none_for_missing_parameter() {
        assert!(query_param(&request("/search?page=1"), "q").is_none());
    }

    #[test]
    fn distinguishes_empty_value_from_missing() {
        let request = request("/search?q=");

        assert_eq!(query_param(&request, "q").unwrap(), "");
    }

    #[test]
    fn parameter_without_equals_has_empty_value() {
        assert_eq!(
            pairs("/search?verbose"),
            vec![("verbose".into(), String::new())]
        );
    }

    #[test]
    fn skips_empty_segments() {
        assert_eq!(
            pairs("/search?&q=1&&page=2&"),
            vec![("q".into(), "1".into()), ("page".into(), "2".into())]
        );
    }

    #[test]
    fn first_occurrence_wins() {
        let request = request("/search?q=first&q=second");

        assert_eq!(query_param(&request, "q").unwrap(), "first");
    }

    #[test]
    fn decodes_encoded_names() {
        let request = request("/search?sort%5Fby=name");

        assert_eq!(query_param(&request, "sort_by").unwrap(), "name");
    }

    #[test]
    fn passes_through_malformed_escapes() {
        assert_eq!(pairs("/search?q=%"), vec![("q".into(), "%".into())]);
        assert_eq!(pairs("/search?q=%2"), vec![("q".into(), "%2".into())]);
        assert_eq!(pairs("/search?q=%zz"), vec![("q".into(), "%zz".into())]);
        assert_eq!(
            pairs("/search?q=100%25x"),
            vec![("q".into(), "100%x".into())]
        );
    }

    #[test]
    fn replaces_invalid_utf8() {
        assert_eq!(
            pairs("/search?q=%FF"),
            vec![("q".into(), "\u{fffd}".into())]
        );
    }
}
