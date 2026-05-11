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
pub(crate) fn code_location_id_for_materialized_ac_test(
    ac_id: &str,
    repo_path: &str,
    file_path: &str,
) -> String {
    let norm = normalize_materialize_file_path(file_path);
    let digest = sha256_hex_labeled(b"cl-v1", &[ac_id, repo_path, norm.as_str()]);
    format!("cl-{digest}")
}

#[must_use]
pub(crate) fn ac_link_id_for_verified_by_file(ac_id: &str, code_location_id: &str) -> String {
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
    fn golden_location_and_link_ids() {
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
    }
}
