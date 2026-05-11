//! Topic-id derivation with UTF-8 NFC normalization per convergence.md §4.6.
//!
//! Lives in `crates/manifest` (not `crates/types`) to keep
//! `unicode-normalization` out of the types crate's dep graph.

use myrhiza_types::{BundleHash, Topic};
use unicode_normalization::UnicodeNormalization;

/// Derive a topic ID, NFC-normalizing `name` before hashing.
///
/// Apps reaching this helper through `myrhiza_manifest` get correct
/// cross-peer topic-id agreement regardless of input Unicode form.
/// Use [`Topic::derive`] directly only if the name is already
/// NFC-normalized.
#[must_use]
pub fn derive_topic_normalized(app_bundle_hash: &BundleHash, seed: &[u8; 32], name: &str) -> Topic {
    let normalized: String = name.nfc().collect();
    Topic::derive(app_bundle_hash, seed, &normalized)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn nfc_canonicalizes_combining_marks() {
        let bh = BundleHash::from_bytes([0; 32]);
        let seed = [0; 32];
        // "café" in two Unicode forms:
        let composed = "caf\u{00E9}"; // NFC: precomposed é
        let decomposed = "cafe\u{0301}"; // NFD: e + combining acute
        assert_eq!(
            derive_topic_normalized(&bh, &seed, composed),
            derive_topic_normalized(&bh, &seed, decomposed),
            "NFC normalization must yield the same topic for equivalent encodings"
        );
    }

    #[test]
    fn matches_direct_derive_for_ascii() {
        let bh = BundleHash::from_bytes([0x11; 32]);
        let seed = [0x22; 32];
        assert_eq!(
            derive_topic_normalized(&bh, &seed, "main"),
            Topic::derive(&bh, &seed, "main")
        );
    }
}
