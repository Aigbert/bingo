//! Credentials in custom headers and request bodies must stay at their origin.
use reqwest::{Url, redirect::Policy};

#[derive(Clone, Copy, Debug, Default)]
pub enum Redirects {
    #[default]
    SameOrigin,
    /// Only for clients that never attach credentials.
    FollowAny,
}

impl Redirects {
    pub(crate) fn policy(self) -> Policy {
        Policy::custom(move |attempt| {
            if attempt.previous().len() > 10 {
                return attempt.error("too many redirects");
            }
            if matches!(self, Self::SameOrigin)
                && !attempt
                    .previous()
                    .first()
                    .is_some_and(|first| same_origin(first, attempt.url()))
            {
                return attempt.error("cross-origin redirect refused");
            }
            attempt.follow()
        })
    }
}

fn same_origin(first: &Url, next: &Url) -> bool {
    first.origin() == next.origin()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn origins_include_scheme_host_and_effective_port() {
        let original = Url::parse("https://EXAMPLE.com/path").unwrap();
        assert!(same_origin(
            &original,
            &Url::parse("https://example.com:443/other").unwrap()
        ));
        for address in [
            "http://example.com",
            "https://example.com:444",
            "https://other.example",
        ] {
            assert!(!same_origin(&original, &Url::parse(address).unwrap()));
        }
        assert!(same_origin(
            &Url::parse("http://example.com").unwrap(),
            &Url::parse("http://example.com:80").unwrap()
        ));
    }
}
