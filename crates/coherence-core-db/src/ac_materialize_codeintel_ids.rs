//! Deterministic primary keys for codeintel rows written by `ac-tests materialize-rust`.
//!
//! # ID formulas (bounded for `VARCHAR(191)` PKs)
//!
//! Locations: `cl-{digest}` where `digest` is the 64-char hex **SHA-256** of the UTF-8 payload:
//!
//! ```text
//! "cl-v1\0" ++ ac_id ++ "\0" ++ repo_path ++ "\0" ++ file_path_normalized
//! ```
//!
//! `file_path_normalization` replaces `\` with `/` (no other mutation).
//!
//! `verified_by` links: `acl-{digest}` over:
//!
//! ```text
//! "acl-vb-v1\0" ++ ac_id ++ "\0" ++ code_location_id
//! ```
//!
//! Collision risk: treated as negligible for M1 (256-bit hash); distinct tuples practically never
//! collide.

use sha2::{Digest, Sha256};

fn sha256_hex_labeled(label: &[u8], parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(label);
    for p in parts {
        hasher.update([0u8]);
        hasher.update(p.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

#[must_use]
pub(crate) fn normalize_materialize_file_path(file_path: &str) -> String {
    file_path.replace('\\', "/")
}

#[must_use]
pub fn code_location_id_for_materialized_ac_test(
    ac_id: &str,
    repo_path: &str,
    file_path: &str,
) -> String {
    let norm = normalize_materialize_file_path(file_path);
    let digest = sha256_hex_labeled(b"cl-v1", &[ac_id, repo_path, norm.as_str()]);
    format!("cl-{digest}")
}

#[must_use]
pub fn ac_link_id_for_verified_by_file(ac_id: &str, code_location_id: &str) -> String {
    let digest = sha256_hex_labeled(b"acl-vb-v1", &[ac_id, code_location_id]);
    format!("acl-{digest}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_stable_called_twice() {
        let a = code_location_id_for_materialized_ac_test("AC-1", ".", "tests/ac/x/y.rs");
        let b = code_location_id_for_materialized_ac_test("AC-1", ".", "tests/ac/x/y.rs");
        assert_eq!(a, b);
        let la = ac_link_id_for_verified_by_file("AC-1", &a);
        let lb = ac_link_id_for_verified_by_file("AC-1", &a);
        assert_eq!(la, lb);
    }

    #[test]
    fn distinct_paths_distinct_location_ids() {
        let a = code_location_id_for_materialized_ac_test("AC-1", ".", "tests/ac/a.rs");
        let b = code_location_id_for_materialized_ac_test("AC-1", ".", "tests/ac/b.rs");
        assert_ne!(a, b);
    }

    #[test]
    fn distinct_ac_ids_distinct_location_ids_same_path() {
        let a = code_location_id_for_materialized_ac_test("AC-1", ".", "tests/ac/s.rs");
        let b = code_location_id_for_materialized_ac_test("AC-2", ".", "tests/ac/s.rs");
        assert_ne!(a, b);
    }

    #[test]
    fn backslash_normalizes_like_slash() {
        let unix = code_location_id_for_materialized_ac_test("AC-1", ".", "tests/ac/x/y.rs");
        let win = code_location_id_for_materialized_ac_test("AC-1", ".", r"tests\ac\x\y.rs");
        assert_eq!(unix, win);
        let mix_id = code_location_id_for_materialized_ac_test("AC-1", ".", r"tests\ac/x/y.rs");
        assert_eq!(mix_id, unix);
    }

    #[test]
    fn golden_location_and_link_ids_two_ac_path_pairs() {
        let loc =
            code_location_id_for_materialized_ac_test("AC-GOLD", ".", "tests/ac/golden/path.rs");
        assert_eq!(
            loc,
            "cl-987fa92ffd675e4269402c78eab3543ed2ca983004b558dd475cd6c52930f229"
        );
        let link = ac_link_id_for_verified_by_file("AC-GOLD", &loc);
        assert_eq!(
            link,
            "acl-ea12800667a00040675e4e1505f68a3599e8d74d0274ccc121621589b9210ac3"
        );

        let loc2 = code_location_id_for_materialized_ac_test(
            "AC-SECOND",
            "repo/sub",
            "tests/ac/second/case.rs",
        );
        assert_eq!(
            loc2,
            "cl-08800b01bba85f9e4cb28c583c9f80b791a12b3f21d722ddaf121b59c40858f9"
        );
        let link2 = ac_link_id_for_verified_by_file("AC-SECOND", &loc2);
        assert_eq!(
            link2,
            "acl-f1b689e132a1fb0e990f0753c3ea31b5a6908e3ccf2b04f962facbe3dcfa25e4"
        );

        assert_ne!(loc, loc2);
        assert_ne!(link, link2);
    }

    /// `codeintel_*` PKs are `VARCHAR(191)`; ids are `cl-` / `acl-` plus 64 hex digits (fixed width).
    #[test]
    fn ids_fit_codeintel_varchar191_pk_without_truncation() {
        let long_path = format!("tests/ac/{}/tail.rs", "x".repeat(500));
        let loc = code_location_id_for_materialized_ac_test("AC-LONG", ".", &long_path);
        let link = ac_link_id_for_verified_by_file("AC-LONG", &loc);
        assert_eq!(loc.len(), 3 + 64);
        assert_eq!(link.len(), 4 + 64);
        assert!(loc.len() <= 191);
        assert!(link.len() <= 191);
    }
}
