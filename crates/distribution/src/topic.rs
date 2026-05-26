//! Per-author topic derivation for revocation + publication.
//!
//! `derive_revocation_topic(author) = BLAKE3("myrhiza/revocations/v1" || author_pubkey)`
//! `derive_publication_topic(author) = BLAKE3("myrhiza/publications/v1" || author_pubkey)`
//!
//! Topic derivation is PUBLIC — any peer that knows the author's
//! pubkey (e.g., from an installed manifest) can derive the topic.
//! Envelopes failing signature verification are dropped at the
//! subscription dispatch layer before reaching state machines
//! (per spec §3.3 + B-4.8 `PeerWarning::SignatureInvalid` pattern).
//!
//! Per B-10 spec §3.3 + §3.4.

use myrhiza_types::AuthorPubkey;

const REVOCATION_DOMAIN: &[u8] = b"myrhiza/revocations/v1";
const PUBLICATION_DOMAIN: &[u8] = b"myrhiza/publications/v1";

/// Derive the revocation topic for an author.
#[must_use]
pub fn derive_revocation_topic(author: AuthorPubkey) -> [u8; 32] {
    derive_topic(REVOCATION_DOMAIN, author)
}

/// Derive the publication topic for an author.
#[must_use]
pub fn derive_publication_topic(author: AuthorPubkey) -> [u8; 32] {
    derive_topic(PUBLICATION_DOMAIN, author)
}

fn derive_topic(domain: &[u8], author: AuthorPubkey) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    hasher.update(author.as_bytes());
    *hasher.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_author() -> AuthorPubkey {
        AuthorPubkey::from_bytes([0x42; 32])
    }

    #[test]
    fn revocation_and_publication_topics_differ_for_same_author() {
        let a = fixture_author();
        assert_ne!(derive_revocation_topic(a), derive_publication_topic(a));
    }

    #[test]
    fn revocation_topic_deterministic() {
        let a = fixture_author();
        assert_eq!(derive_revocation_topic(a), derive_revocation_topic(a));
    }

    #[test]
    fn revocation_topic_differs_per_author() {
        let a1 = AuthorPubkey::from_bytes([0x42; 32]);
        let a2 = AuthorPubkey::from_bytes([0x43; 32]);
        assert_ne!(derive_revocation_topic(a1), derive_revocation_topic(a2));
    }

    #[test]
    fn revocation_topic_matches_blake3_directly() {
        let a = fixture_author();
        let mut h = blake3::Hasher::new();
        h.update(REVOCATION_DOMAIN);
        h.update(a.as_bytes());
        assert_eq!(derive_revocation_topic(a), *h.finalize().as_bytes());
    }
}
