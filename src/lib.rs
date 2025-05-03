//! `accept-language` is a tiny library for parsing the Accept-Language header from browsers (as defined [here](https://www.w3.org/Protocols/rfc2616/rfc2616-sec14.html)).
//!
//! It's intended to be used in a web server that supports some level of internationalization (i18n),
//! but can be used anytime an Accept-Language header string is available.
//!
//! In order to help facilitate better i18n, a function is provided to return the intersection of
//! the languages the user prefers and the languages your application supports.
//! You can try the `cargo test` and `cargo bench` to verify the behaviour.
//!
//! # Example
//!
//! ```
//! use accept_language::{intersection, parse};
//!
//! let user_languages = parse("en-US, en-GB;q=0.5");
//! let common_languages = intersection("en-US, en-GB;q=0.5", &["en-US", "de", "en-GB"]);
//! ```
use std::cmp::Ordering;
use std::str;
use std::str::FromStr;

use unicase::UniCase;

#[derive(Debug)]
struct Language<'a> {
    name: &'a str,
    quality: f32,
}

impl Eq for Language<'_> {}

impl Ord for Language<'_> {
    fn cmp(&self, other: &Language) -> Ordering {
        if self.quality > other.quality {
            Ordering::Less
        } else if self.quality < other.quality {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

impl PartialOrd for Language<'_> {
    fn partial_cmp(&self, other: &Language) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Language<'_> {
    fn eq(&self, other: &Language) -> bool {
        self.quality == other.quality && self.name.eq_ignore_ascii_case(other.name)
    }
}

impl Language<'_> {
    fn new(tag: &str) -> Language {
        let tag_parts: Vec<&str> = tag.split(';').collect();
        let name = tag_parts[0];
        let quality = match tag_parts.len() {
            1 => 1.0,
            _ => Language::quality_with_default(tag_parts[1]),
        };
        Language { name, quality }
    }

    fn quality_with_default(raw_quality: &str) -> f32 {
        let quality_parts: Vec<&str> = raw_quality.split('=').collect();
        match quality_parts.len() {
            2 => f32::from_str(quality_parts[1]).unwrap_or(0.0),
            _ => 0.0,
        }
    }
}

/// Parse a raw Accept-Language header value into an ordered list of language tags.
/// This should return the exact same list as `window.navigator.languages` in supported browsers.
///
/// # Example
///
/// ```
/// use accept_language::parse;
///
/// let user_languages = parse("en-US, en-GB;q=0.5");
/// ```
pub fn parse(raw_languages: &str) -> Vec<&str> {
    let language_strings = raw_languages.split(',').map(str::trim);
    let mut languages: Vec<Language> = language_strings.map(|l| Language::new(l)).collect();
    languages.sort();
    languages
        .iter()
        .map(|l| l.name)
        .filter(|l| !l.is_empty())
        .collect()
}

/// Similar to [`parse`](parse) but with quality `f32` appended to notice if it is a default value.
/// is used by [`intersection_with_quality`](intersection_with_quality) and
/// [`intersection_ordered_with_quality`](intersection_ordered_with_quality).
///
/// # Example
///
/// ```
/// use accept_language::parse_with_quality;
///
/// let user_languages = parse_with_quality("en-US, en-GB;q=0.5");
/// assert_eq!(user_languages,vec![("en-US", 1.0), ("en-GB", 0.5)])
/// ```
pub fn parse_with_quality(raw_languages: &str) -> Vec<(&str, f32)> {
    let language_strings = raw_languages.split(',').map(str::trim);
    let mut languages: Vec<Language> = language_strings.map(|l| Language::new(l)).collect();
    languages.sort();
    languages
        .iter()
        .map(|l| (l.name, l.quality))
        .filter(|l| !l.0.is_empty())
        .collect()
}

/// Compare an Accept-Language header value with your application's supported languages to find
/// the common languages that could be presented to a user.
///
/// # Example
///
/// ```
/// use accept_language::intersection;
///
/// let common_languages = intersection("en-US, en-GB;q=0.5", &["en-US", "de", "en-GB"]);
/// ```
pub fn intersection<'a>(raw_languages: &str, supported_languages: &[&'a str]) -> Vec<&'a str> {
    let user_languages = parse(raw_languages);
    user_languages
        .into_iter()
        .filter_map(|l| {
            supported_languages
                .iter()
                .find(|s| s.eq_ignore_ascii_case(l))
                .copied()
        })
        .collect()
}
/// Similar to [`intersection`](intersection) but using binary sort. The supported languages
/// MUST be in alphabetical order, to find the common languages that could be presented
/// to a user. Executes roughly 25% faster.
///
/// # Example
///
/// ```
/// use accept_language::intersection_ordered;
///
/// let common_languages = intersection_ordered("en-US, en-GB;q=0.5", &["de", "en-GB", "en-US"]);
/// ```
pub fn intersection_ordered<'a>(
    raw_languages: &str,
    supported_languages: &[&'a str],
) -> Vec<&'a str> {
    let user_languages = parse(raw_languages);
    user_languages
        .into_iter()
        .filter_map(|l| {
            let l = UniCase::unicode(l);
            let idx = supported_languages
                .binary_search_by(|val| {
                    let val = UniCase::unicode(*val);
                    val.cmp(&l)
                })
                .ok()?;
            Some(supported_languages[idx])
        })
        .collect()
}
/// Similar to [`intersection`](intersection) but with the quality as `f32` appended for each language.
/// This enables distinction between the default language of a user (value 1.0) and the
/// best match. If you don't want to assign your users immediatly to a non-default choice and you plan to add
/// more languages later on in your webserver.
///
/// # Example
///
/// ```
/// use accept_language::intersection_with_quality;
///
/// let common_languages = intersection_with_quality("en-US, en-GB;q=0.5", &["en-US", "de", "en-GB"]);
/// assert_eq!(common_languages,vec![("en-US", 1.0), ("en-GB", 0.5)])
/// ```
pub fn intersection_with_quality<'a>(
    raw_languages: &str,
    supported_languages: &[&'a str],
) -> Vec<(&'a str, f32)> {
    let user_languages = parse_with_quality(raw_languages);
    user_languages
        .into_iter()
        .filter_map(|l| {
            Some((
                supported_languages
                    .iter()
                    .find(|s| s.eq_ignore_ascii_case(l.0))
                    .copied()?,
                l.1,
            ))
        })
        .collect()
}

/// Similar to [`intersection_with_quality`](intersection_with_quality). The supported languages MUST
/// be in alphabetical order, to find the common languages that could be presented to a user.
/// Executes roughly 25% faster.
///
/// # Example
///
/// ```
/// use accept_language::intersection_ordered_with_quality;
///
/// let common_languages = intersection_ordered_with_quality("en-US, en-GB;q=0.5", &["de", "en-GB", "en-US"]);
/// assert_eq!(common_languages,vec![("en-US", 1.0), ("en-GB", 0.5)])
/// ```
pub fn intersection_ordered_with_quality<'a>(
    raw_languages: &str,
    supported_languages: &[&'a str],
) -> Vec<(&'a str, f32)> {
    let user_languages = parse_with_quality(raw_languages);
    user_languages
        .into_iter()
        .filter_map(|l| {
            let q = l.1;
            let l = UniCase::unicode(l.0);
            let idx = supported_languages
                .binary_search_by(|val| {
                    let val = UniCase::unicode(*val);
                    val.cmp(&l)
                })
                .ok()?;
            Some((supported_languages[idx], q))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        intersection, intersection_ordered, intersection_ordered_with_quality,
        intersection_with_quality, parse, Language,
    };

    static MOCK_ACCEPT_LANGUAGE: &str = "en-US, de;q=0.7, zh-Hant, jp;q=0.1";
    static AVIALABLE_LANGUAGES: &[&str] =
        &["da", "de", "en-US", "it", "jp", "zh", "zh-Hans", "zh-Hant"];

    #[test]
    fn it_creates_a_new_language_from_a_string() {
        let language = Language::new("en-US;q=0.7");
        assert_eq!(
            language,
            Language {
                name: "en-US",
                quality: 0.7,
            }
        )
    }

    #[test]
    fn it_creates_a_new_language_from_a_string_with_lowercase_country() {
        let language = Language::new("en-us;q=0.7");
        assert_eq!(
            language,
            Language {
                name: "en-US",
                quality: 0.7,
            }
        )
    }

    #[test]
    fn it_creates_a_new_language_from_a_string_with_a_default_quality() {
        let language = Language::new("en-US");
        assert_eq!(
            language,
            Language {
                name: "en-US",
                quality: 1.0,
            }
        )
    }

    #[test]
    fn it_parses_quality() {
        let quality = Language::quality_with_default("q=0.5");
        assert_eq!(quality, 0.5)
    }

    #[test]
    fn it_parses_an_invalid_quality() {
        let quality = Language::quality_with_default("q=yolo");
        assert_eq!(quality, 0.0)
    }

    #[test]
    fn it_parses_a_valid_accept_language_header() {
        let user_languages = parse(MOCK_ACCEPT_LANGUAGE);
        assert_eq!(user_languages, vec!["en-US", "zh-Hant", "de", "jp"])
    }

    #[test]
    fn it_parses_an_empty_accept_language_header() {
        let user_languages = parse("");
        assert_eq!(user_languages.len(), 0)
    }

    #[test]
    fn it_parses_an_invalid_accept_language_header() {
        let user_languages_one = parse("q");
        let user_languages_two = parse(";q");
        let user_languages_three = parse("q-");
        let user_languages_four = parse("en;q=");
        assert_eq!(user_languages_one, vec!["q"]);
        assert_eq!(user_languages_two.len(), 0);
        assert_eq!(user_languages_three, vec!["q-"]);
        assert_eq!(user_languages_four, vec!["en"])
    }

    #[test]
    fn it_sorts_languages_by_quality() {
        let user_languages = parse("en-US, de;q=0.1, jp;q=0.7");
        assert_eq!(user_languages, vec!["en-US", "jp", "de"])
    }

    #[test]
    fn it_returns_language_intersection() {
        let common_languages = intersection(MOCK_ACCEPT_LANGUAGE, AVIALABLE_LANGUAGES);
        assert_eq!(common_languages, vec!["en-US", "zh-Hant", "de", "jp"])
    }

    #[test]
    fn it_returns_language_intersection_case_insensitive() {
        let common_languages =
            intersection(&MOCK_ACCEPT_LANGUAGE.to_uppercase(), AVIALABLE_LANGUAGES);
        assert_eq!(common_languages, vec!["en-US", "zh-Hant", "de", "jp"])
    }

    #[test]
    fn it_returns_language_intersection_ordered() {
        let common_languages = intersection_ordered(MOCK_ACCEPT_LANGUAGE, AVIALABLE_LANGUAGES);
        assert_eq!(common_languages, vec!["en-US", "zh-Hant", "de", "jp"])
    }

    #[test]
    fn it_returns_language_intersection_ordered_case_insensitive() {
        let common_languages =
            intersection_ordered(&MOCK_ACCEPT_LANGUAGE.to_uppercase(), AVIALABLE_LANGUAGES);
        assert_eq!(common_languages, vec!["en-US", "zh-Hant", "de", "jp"])
    }

    #[test]
    fn it_returns_language_intersection_with_quality() {
        let common_languages = intersection_with_quality(MOCK_ACCEPT_LANGUAGE, &["en-US", "jp"]);
        assert_eq!(common_languages, vec![("en-US", 1.0), ("jp", 0.1)])
    }

    #[test]
    fn it_returns_language_intersection_with_quality_case_insensitive() {
        let common_languages =
            intersection_with_quality(&MOCK_ACCEPT_LANGUAGE.to_uppercase(), &["en-US", "jp"]);
        assert_eq!(common_languages, vec![("en-US", 1.0), ("jp", 0.1)])
    }

    #[test]
    fn it_returns_language_intersection_ordered_with_quality() {
        let common_languages =
            intersection_ordered_with_quality(MOCK_ACCEPT_LANGUAGE, &["en-US", "jp"]);
        assert_eq!(common_languages, vec![("en-US", 1.0), ("jp", 0.1)])
    }

    #[test]
    fn it_returns_language_intersection_ordered_with_quality_case_insensitive() {
        let common_languages = intersection_ordered_with_quality(
            &MOCK_ACCEPT_LANGUAGE.to_uppercase(),
            &["en-US", "jp"],
        );
        assert_eq!(common_languages, vec![("en-US", 1.0), ("jp", 0.1)])
    }

    #[test]
    fn it_returns_an_empty_array_when_no_intersection() {
        let common_languages = intersection(MOCK_ACCEPT_LANGUAGE, &["fr", "en-GB"]);
        assert_eq!(common_languages.len(), 0)
    }

    #[test]
    fn it_parses_traditional_chinese() {
        assert_eq!(parse("zh-Hant"), &["zh-Hant"]);
    }

    #[test]
    fn it_implements_case_insensitive_equality() {
        assert_eq!(Language::new("en-US"), Language::new("en-us"));
        assert_eq!(Language::new("en-US;q=0.7"), Language::new("en-us;q=0.7"));
        assert_ne!(Language::new("en"), Language::new("en-US"));
        assert_ne!(Language::new("en;q=0.7"), Language::new("en;q=0.8"));
        assert_ne!(Language::new("en;q=0.7"), Language::new("en-US;q=0.7"));
    }
}
