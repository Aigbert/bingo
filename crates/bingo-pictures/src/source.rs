//! Where a picture comes from.
//!
//! An `@word` in a composed line, a `--image` argument: either a path on this
//! machine or a URL this machine fetches (ADR-0041 §3). Which of the two a
//! word is, is a question about the word alone — so it is answered here, by a
//! pure function, and the reading is [`crate::load`]'s.
//!
//! A `file:` URL names a path on this machine and is read as that path. A
//! Markdown destination ([`Source::link`]) is written as a URL is, so its
//! `%20` is a space; a word a person typed is taken as typed.

use std::fmt;
use std::path::{Path, PathBuf};

/// The two schemes a picture may be fetched over. A `file:` URL is a path on
/// this machine; anything else — a `data:`, an `ftp:`, a bare word — is a
/// path too, and a path that does not exist says so.
const SCHEMES: [&str; 2] = ["http://", "https://"];

/// The scheme of a URL that names a path on this machine.
const FILE: &str = "file:";

/// One picture's whereabouts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Path(PathBuf),
    Url(String),
}

impl Source {
    /// What `word` names, with a relative path taken from `cwd` — whose
    /// directory a path is in is the caller's to know, never this crate's.
    pub fn parse(word: &str, cwd: &Path) -> Source {
        if is_url(word) {
            return Source::Url(word.to_owned());
        }
        Source::Path(file_path(word).unwrap_or_else(|| cwd.join(word)))
    }

    /// What a Markdown destination names. A destination is spelled the way a
    /// URL is, so a path in one carries its spaces and its non-ASCII letters
    /// percent-encoded; they are decoded before the path is looked up.
    pub fn link(destination: &str, cwd: &Path) -> Source {
        match is_url(destination) || has_scheme(destination, FILE) {
            true => Source::parse(destination, cwd),
            false => Source::parse(&percent_decoded(destination), cwd),
        }
    }
}

/// The path a `file:` URL names, decoded — or nothing when the word is not
/// one, or names another machine's file.
fn file_path(word: &str) -> Option<PathBuf> {
    if !has_scheme(word, FILE) {
        return None;
    }
    reqwest::Url::parse(word).ok()?.to_file_path().ok()
}

/// `%XX` as the byte it stands for. A `%` that is not followed by two hex
/// digits is itself, and a decoding that is not UTF-8 leaves the word as it
/// was: a name that does not decode is more likely a name with a `%` in it.
fn percent_decoded(word: &str) -> String {
    let bytes = word.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match (bytes[at], escaped(bytes.get(at + 1..at + 3))) {
            (b'%', Some(byte)) => {
                decoded.push(byte);
                at += 3;
            }
            (byte, _) => {
                decoded.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8(decoded).unwrap_or_else(|_| word.to_owned())
}

/// The byte two hex digits spell.
fn escaped(digits: Option<&[u8]>) -> Option<u8> {
    let text = std::str::from_utf8(digits?).ok()?;
    u8::from_str_radix(text, 16).ok()
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::Path(path) => write!(f, "{}", path.display()),
            Source::Url(url) => f.write_str(url),
        }
    }
}

/// Whether a word could name a picture at all, which is what makes an `@word`
/// an attachment rather than prose. A URL is one whatever its path says — the
/// bytes are what decide, and a URL often ends in no name at all; a path is
/// one when its extension is a format the decoder knows by name.
pub fn names_a_picture(word: &str) -> bool {
    is_url(word) || has_a_pictures_extension(word)
}

fn is_url(word: &str) -> bool {
    SCHEMES.iter().any(|scheme| has_scheme(word, scheme))
}

fn has_scheme(word: &str, scheme: &str) -> bool {
    word.get(..scheme.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(scheme))
}

fn has_a_pictures_extension(word: &str) -> bool {
    Path::new(word)
        .extension()
        .is_some_and(|ext| image::ImageFormat::from_extension(ext).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_with_a_scheme_is_a_url_and_everything_else_is_a_path() {
        let cwd = Path::new("/work");
        assert_eq!(
            Source::parse("https://x/y.jpg", cwd),
            Source::Url("https://x/y.jpg".into())
        );
        assert_eq!(
            Source::parse("http://x/y", cwd),
            Source::Url("http://x/y".into())
        );
        assert_eq!(
            Source::parse("shot.png", cwd),
            Source::Path(PathBuf::from("/work/shot.png"))
        );
    }

    #[test]
    fn an_absolute_path_is_itself_and_a_relative_one_is_under_the_directory() {
        let cwd = Path::new("/work");
        assert_eq!(
            Source::parse("/tmp/shot.png", cwd),
            Source::Path(PathBuf::from("/tmp/shot.png"))
        );
        assert_eq!(
            Source::parse("sub/shot.png", cwd),
            Source::Path(PathBuf::from("/work/sub/shot.png"))
        );
    }

    /// A scheme is case-insensitive, and a word that only looks like one is
    /// still a path — nothing here reaches the network by accident.
    #[test]
    fn the_scheme_is_read_case_insensitively_and_nothing_near_it_counts() {
        let cwd = Path::new("/work");
        assert_eq!(
            Source::parse("HTTPS://x/y.png", cwd),
            Source::Url("HTTPS://x/y.png".into())
        );
        assert!(matches!(
            Source::parse("httpsomething.png", cwd),
            Source::Path(_)
        ));
        assert!(matches!(
            Source::parse("ftp://x/y.png", cwd),
            Source::Path(_)
        ));
        assert!(matches!(Source::parse("http:/x.png", cwd), Source::Path(_)));
    }

    /// A word shorter than a scheme, and a word whose first bytes are not a
    /// character boundary: neither may panic on the slice.
    #[test]
    fn a_short_or_wide_word_is_a_path() {
        let cwd = Path::new("/work");
        assert!(matches!(Source::parse("h", cwd), Source::Path(_)));
        assert!(matches!(Source::parse("", cwd), Source::Path(_)));
        assert!(matches!(Source::parse("图片.png", cwd), Source::Path(_)));
    }

    /// A `file:` URL is the path it names, decoded; one that names another
    /// machine, or does not parse, is a path that is not there.
    #[cfg(unix)]
    #[test]
    fn a_file_url_is_the_path_it_names() {
        let cwd = Path::new("/work");
        assert_eq!(
            Source::parse("file:///tmp/my%20shot.png", cwd),
            Source::Path(PathBuf::from("/tmp/my shot.png"))
        );
        assert_eq!(
            Source::parse("FILE:///tmp/%E6%A1%8C.png", cwd),
            Source::Path(PathBuf::from("/tmp/桌.png"))
        );
        assert_eq!(
            Source::parse("file://elsewhere/x.png", cwd),
            Source::Path(PathBuf::from("/work/file://elsewhere/x.png"))
        );
    }

    #[cfg(windows)]
    #[test]
    fn a_file_url_is_the_path_it_names() {
        assert_eq!(
            Source::parse("file:///C:/shots/my%20shot.png", Path::new("D:\\work")),
            Source::Path(PathBuf::from("C:\\shots\\my shot.png"))
        );
    }

    /// A Markdown destination is spelled as a URL: `%20` is a space there and
    /// not in a word a person typed.
    #[test]
    fn a_link_is_decoded_and_a_typed_word_is_not() {
        let cwd = Path::new("/work");
        assert_eq!(
            Source::link("my%20shot.png", cwd),
            Source::Path(PathBuf::from("/work/my shot.png"))
        );
        assert_eq!(
            Source::link("%E6%A1%8C%E9%9D%A2.png", cwd),
            Source::Path(PathBuf::from("/work/桌面.png"))
        );
        assert_eq!(
            Source::parse("my%20shot.png", cwd),
            Source::Path(PathBuf::from("/work/my%20shot.png"))
        );
        assert_eq!(
            Source::link("https://x.dev/a%20b.png", cwd),
            Source::Url("https://x.dev/a%20b.png".into()),
            "an address is fetched as written"
        );
    }

    /// A `%` that escapes nothing, or a decoding that is not text, leaves the
    /// destination as it was written.
    #[test]
    fn a_percent_that_escapes_nothing_is_itself() {
        assert_eq!(percent_decoded("100%.png"), "100%.png");
        assert_eq!(percent_decoded("a%2"), "a%2");
        assert_eq!(percent_decoded("a%zz.png"), "a%zz.png");
        assert_eq!(percent_decoded("%FF.png"), "%FF.png");
        assert_eq!(percent_decoded("%e6%a1%8c"), "桌");
        assert_eq!(percent_decoded("图%2F片"), "图/片");
    }

    #[test]
    fn every_extension_a_decoder_knows_names_a_picture_and_a_url_always_does() {
        for word in [
            "shot.png",
            "shot.JPEG",
            "a/b.gif",
            "x.webp",
            "x.bmp",
            "x.tiff",
            "x.ico",
            "x.qoi",
            "x.tga",
            "x.ppm",
        ] {
            assert!(names_a_picture(word), "{word}");
        }
        assert!(names_a_picture("https://x/y"), "a URL names no type");
        assert!(names_a_picture("http://x/page"));
    }

    #[test]
    fn prose_and_a_source_file_name_no_picture() {
        for word in ["src/lib.rs", "notes", "Cargo.toml", "shot.png.txt"] {
            assert!(!names_a_picture(word), "{word}");
        }
    }
}
