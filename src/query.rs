use std::borrow::Cow;

use crate::Request;

/// Returns an iterator over the percent-decoded `name=value` pairs in the
/// request's query string. Follows the `application/x-www-form-urlencoded' rules.
pub fn query_pairs<'a>(
    request: &'a Request,
) -> impl Iterator<Item = (Cow<'a, str>, Cow<'a, str>)> + 'a {
    form_urlencoded::parse(request.uri().query().unwrap_or("").as_bytes())
}

/// Returns the percent-decoded value of the first query parameter named `name`,
/// decoded as described on [`query_pairs`].
pub fn query_param<'a>(request: &'a Request, name: &str) -> Option<Cow<'a, str>> {
    query_pairs(request)
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
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
    fn decodes_query_pairs() {
        let cases: &[(&str, &[(&str, &str)])] = &[
            ("/search", &[]),
            ("/search?q=plain", &[("q", "plain")]),
            ("/search?q=1&page=2", &[("q", "1"), ("page", "2")]),
            // `%XX` escapes, in names as well as values.
            (
                "/events?since=2026-02-05T12%3A34%3A56Z",
                &[("since", "2026-02-05T12:34:56Z")],
            ),
            ("/search?sort%5Fby=name", &[("sort_by", "name")]),
            // `+` is a space.
            ("/search?q=a+b", &[("q", "a b")]),
            // Present-but-empty, and no `=` at all.
            ("/search?q=", &[("q", "")]),
            ("/search?verbose", &[("verbose", "")]),
            // Empty segments are skipped.
            ("/search?&q=1&&page=2&", &[("q", "1"), ("page", "2")]),
            // Malformed escapes pass through unchanged.
            ("/search?q=%", &[("q", "%")]),
            ("/search?q=%2", &[("q", "%2")]),
            ("/search?q=%zz", &[("q", "%zz")]),
            ("/search?q=100%25x", &[("q", "100%x")]),
            // Bytes that are not valid UTF-8 once decoded are replaced.
            ("/search?q=%FF", &[("q", "\u{fffd}")]),
        ];

        for (uri, expected) in cases {
            let expected: Vec<(String, String)> = expected
                .iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect();
            assert_eq!(pairs(uri), expected, "uri: {uri}");
        }
    }

    #[test]
    fn query_param_takes_the_first_match() {
        let request = request("/search?q=first&q=second");

        assert_eq!(query_param(&request, "q").unwrap(), "first");
    }

    #[test]
    fn query_param_is_none_when_absent() {
        assert!(query_param(&request("/search"), "q").is_none());
        assert!(query_param(&request("/search?page=1"), "q").is_none());
    }

    #[test]
    fn query_param_borrows_when_nothing_to_decode() {
        let request = request("/search?q=plain");

        assert!(matches!(query_param(&request, "q"), Some(Cow::Borrowed(_))));
    }
}
