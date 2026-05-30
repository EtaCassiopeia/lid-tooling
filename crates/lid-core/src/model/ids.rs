//! Validated identifier newtypes: [`SpecId`] and [`GitSha`].
//!
//! Both types use the newtype pattern with a private constructor; the only
//! way to obtain an instance is through [`SpecId::parse`] / [`GitSha::parse`],
//! which enforce the lexical shape so invalid identifiers cannot exist at
//! runtime.

use std::fmt;
use std::str::FromStr;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};

use crate::LidError;

/// Generates the five boilerplate trait impls shared by every validated
/// ID newtype: `as_str`, `Display`, `FromStr`, `AsRef<str>`, `Deserialize`.
macro_rules! impl_id_boilerplate {
    ($T:ident) => {
        impl $T {
            /// Borrow the underlying string slice.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $T {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl FromStr for $T {
            type Err = LidError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::parse(s)
            }
        }

        impl AsRef<str> for $T {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $T {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let s = String::deserialize(d)?;
                Self::parse(&s).map_err(serde::de::Error::custom)
            }
        }
    };
}

/// Anchored pattern for a syntactically valid spec ID.
///
/// Shape: one uppercase letter, optionally followed by uppercase letters or
/// digits, then one or more `-{SEGMENT}` groups. A separate post-check
/// requires at least one digit anywhere in the string (this rules out
/// inputs like `A-Z` that match the shape but are clearly not spec IDs).
///
/// To find spec IDs *inside* arbitrary text (source code, prose) use the
/// unanchored pattern defined under `parse::source`, which adds a
/// `\b`-style boundary guard to avoid matching mid-identifier hits.
#[allow(clippy::expect_used)] // constant pattern; compilation is infallible
static SPEC_ID_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Z][A-Z0-9]*(?:-[A-Z0-9]+)+$").expect("SPEC_ID_PATTERN compiles")
});

/// Anchored pattern for a short-or-full git SHA-1 hash (7..=40 lowercase hex).
#[allow(clippy::expect_used)] // constant pattern; compilation is infallible
static GIT_SHA_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[0-9a-f]{7,40}$").expect("GIT_SHA_PATTERN compiles"));

/// Anchored pattern for a segment name (kebab-case, e.g. `linked-intent-dev`).
///
/// Lowercase letter start, optional body of lowercase alphanumerics or
/// hyphens, must end with an alphanumeric if longer than one character.
#[allow(clippy::expect_used)] // constant pattern; compilation is infallible
static SEGMENT_ID_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-z](?:[a-z0-9-]*[a-z0-9])?$").expect("SEGMENT_ID_PATTERN compiles")
});

/// A LID spec identifier such as `AUTH-UI-001`.
///
/// Stable once assigned; the methodology forbids reuse after deletion.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct SpecId(String);

impl SpecId {
    /// Parse a candidate string into a [`SpecId`].
    ///
    /// # Errors
    /// Returns [`LidError::InvalidSpecId`] if the input does not match the
    /// shape `^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$` or contains no digits.
    pub fn parse(value: &str) -> Result<Self, LidError> {
        if !SPEC_ID_PATTERN.is_match(value) {
            return Err(LidError::InvalidSpecId {
                value: value.to_owned(),
                reason: "expected pattern ^[A-Z][A-Z0-9]*(-[A-Z0-9]+)+$",
            });
        }
        if !value.bytes().any(|b| b.is_ascii_digit()) {
            return Err(LidError::InvalidSpecId {
                value: value.to_owned(),
                reason: "must contain at least one digit",
            });
        }
        Ok(Self(value.to_owned()))
    }
}

impl_id_boilerplate!(SpecId);

/// A git SHA-1 hash in its short or full lowercase-hex form (7..=40 chars).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct GitSha(String);

impl GitSha {
    /// Parse a candidate string into a [`GitSha`].
    ///
    /// # Errors
    /// Returns [`LidError::InvalidGitSha`] if the input is not 7..=40
    /// lowercase hexadecimal characters.
    pub fn parse(value: &str) -> Result<Self, LidError> {
        if !GIT_SHA_PATTERN.is_match(value) {
            return Err(LidError::InvalidGitSha {
                value: value.to_owned(),
                reason: "expected 7..=40 lowercase hexadecimal characters",
            });
        }
        Ok(Self(value.to_owned()))
    }
}

impl_id_boilerplate!(GitSha);

/// A LID arrow-segment name such as `linked-intent-dev` or `arrow-maintenance`.
///
/// Used as both the YAML key under `arrows:` in `index.yaml` and as the
/// reference target in `blocks` / `blockedBy` / `merged_into` edges.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct SegmentId(String);

impl SegmentId {
    /// Parse a candidate string into a [`SegmentId`].
    ///
    /// # Errors
    /// Returns [`LidError::InvalidSegmentId`] if the input is empty, starts
    /// with a non-lowercase-letter, ends with a hyphen, or contains
    /// characters outside `[a-z0-9-]`.
    pub fn parse(value: &str) -> Result<Self, LidError> {
        if !SEGMENT_ID_PATTERN.is_match(value) {
            return Err(LidError::InvalidSegmentId {
                value: value.to_owned(),
                reason: "expected kebab-case: ^[a-z]([a-z0-9-]*[a-z0-9])?$",
            });
        }
        Ok(Self(value.to_owned()))
    }
}

impl_id_boilerplate!(SegmentId);

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ── SpecId — valid shapes ────────────────────────────────────────────

    #[test]
    fn parses_two_segment_id() {
        let id = SpecId::parse("AUTH-001").unwrap();
        assert_eq!(id.as_str(), "AUTH-001");
        assert_eq!(id.to_string(), "AUTH-001");
    }

    #[test]
    fn parses_three_segment_id() {
        assert!(SpecId::parse("AUTH-UI-001").is_ok());
    }

    #[test]
    fn parses_four_segment_id() {
        assert!(SpecId::parse("AUTH-LOGIN-UI-001").is_ok());
    }

    #[test]
    fn parses_real_world_examples_from_lid_upstream() {
        for s in [
            "ARROW-MAINT-001",
            "LID-COACH-053",
            "BIDIFF-007",
            "MKT-SITE-047",
            "PROJ-STRUCT-041",
        ] {
            assert!(SpecId::parse(s).is_ok(), "should parse: {s}");
        }
    }

    // ── SpecId — invalid shapes ─────────────────────────────────────────

    #[test]
    fn rejects_lowercase() {
        assert!(SpecId::parse("auth-001").is_err());
        assert!(SpecId::parse("Auth-001").is_err());
    }

    #[test]
    fn rejects_no_hyphen() {
        assert!(SpecId::parse("AUTH001").is_err());
        assert!(SpecId::parse("AUTH").is_err());
    }

    #[test]
    fn rejects_no_digit() {
        // Shape matches but the digit post-filter trips
        assert!(SpecId::parse("A-Z").is_err());
        assert!(SpecId::parse("AUTH-UI").is_err());
    }

    #[test]
    fn rejects_trailing_or_leading_hyphen() {
        assert!(SpecId::parse("-AUTH-001").is_err());
        assert!(SpecId::parse("AUTH-001-").is_err());
        assert!(SpecId::parse("AUTH--001").is_err());
    }

    #[test]
    fn rejects_leading_digit() {
        assert!(SpecId::parse("1AUTH-001").is_err());
    }

    #[test]
    fn rejects_empty() {
        assert!(SpecId::parse("").is_err());
    }

    // ── GitSha ──────────────────────────────────────────────────────────

    #[test]
    fn parses_short_sha() {
        assert!(GitSha::parse("a1b2c3d").is_ok());
    }

    #[test]
    fn parses_full_sha() {
        let full = "a1b2c3d4e5f6789012345678901234567890abcd";
        assert_eq!(full.len(), 40);
        assert!(GitSha::parse(full).is_ok());
    }

    #[test]
    fn rejects_uppercase_sha() {
        assert!(GitSha::parse("A1B2C3D").is_err());
    }

    #[test]
    fn rejects_non_hex() {
        assert!(GitSha::parse("ghijklm").is_err());
    }

    #[test]
    fn rejects_too_short() {
        assert!(GitSha::parse("a1b2c3").is_err());
    }

    #[test]
    fn rejects_too_long() {
        let too_long = "a".repeat(41);
        assert!(GitSha::parse(&too_long).is_err());
    }

    // ── Serde round-trips ──────────────────────────────────────────────

    #[test]
    fn spec_id_serde_roundtrip() {
        let id = SpecId::parse("AUTH-UI-001").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"AUTH-UI-001\"");
        let back: SpecId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn spec_id_deserialize_validates() {
        assert!(serde_json::from_str::<SpecId>("\"not-valid\"").is_err());
        assert!(serde_json::from_str::<SpecId>("\"AUTH-UI\"").is_err());
    }

    #[test]
    fn git_sha_serde_roundtrip() {
        let sha = GitSha::parse("abc1234").unwrap();
        let json = serde_json::to_string(&sha).unwrap();
        assert_eq!(json, "\"abc1234\"");
        let back: GitSha = serde_json::from_str(&json).unwrap();
        assert_eq!(back, sha);
    }

    // ── SegmentId ──────────────────────────────────────────────────────

    #[test]
    fn parses_upstream_segment_names() {
        for s in [
            "linked-intent-dev",
            "lid-coach",
            "arrow-maintenance",
            "lid-experimental",
            "bidirectional-differential",
            "marketing-site",
            "project-structure",
        ] {
            assert!(SegmentId::parse(s).is_ok(), "should parse: {s}");
        }
    }

    #[test]
    fn parses_single_letter_segment() {
        assert!(SegmentId::parse("a").is_ok());
    }

    #[test]
    fn rejects_uppercase_segment() {
        assert!(SegmentId::parse("Auth").is_err());
        assert!(SegmentId::parse("AUTH").is_err());
    }

    #[test]
    fn rejects_segment_edge_hyphens() {
        assert!(SegmentId::parse("-auth").is_err());
        assert!(SegmentId::parse("auth-").is_err());
    }

    #[test]
    fn rejects_segment_underscores_or_spaces() {
        assert!(SegmentId::parse("auth_login").is_err());
        assert!(SegmentId::parse("auth login").is_err());
    }

    #[test]
    fn segment_id_serde_roundtrip() {
        let seg = SegmentId::parse("arrow-maintenance").unwrap();
        let json = serde_json::to_string(&seg).unwrap();
        assert_eq!(json, "\"arrow-maintenance\"");
        let back: SegmentId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, seg);
    }

    // ── Property: well-shaped inputs parse ─────────────────────────────

    proptest! {
        #[test]
        fn well_shaped_spec_ids_parse(s in r"[A-Z][A-Z]{0,5}(-[A-Z]{1,5}){0,2}-[0-9]{1,4}") {
            prop_assert!(SpecId::parse(&s).is_ok(), "should parse: {s}");
        }

        #[test]
        fn well_shaped_short_shas_parse(s in r"[0-9a-f]{7,40}") {
            prop_assert!(GitSha::parse(&s).is_ok(), "should parse: {s}");
        }

        #[test]
        fn well_shaped_segment_ids_parse(s in r"[a-z][a-z0-9-]{0,8}[a-z0-9]") {
            prop_assert!(SegmentId::parse(&s).is_ok(), "should parse: {s}");
        }

        #[test]
        fn parse_never_panics(s in ".*") {
            // Any UTF-8 input: parse returns Ok or Err, never panics.
            let _ = SpecId::parse(&s);
            let _ = GitSha::parse(&s);
            let _ = SegmentId::parse(&s);
        }
    }
}
