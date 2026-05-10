//! Kernel-side `IdentityScope` value type.
//!
//! This mirrors the WIT `identity-scope` record in [identity.md §6.1].
//! Components see opaque `identity-handle` resources at the WIT
//! boundary — never these structs. The structs are used in code paths
//! that don't cross WIT (manifest author-policy checks, kernel-side
//! signing key lookup).
//!
//! `long-term` MUST be Ed25519 per Cremers ETK 2025
//! ([identity.md §6.2]). The kernel does not expose any signing API
//! that takes an algorithm parameter.

use serde::{Deserialize, Serialize};

use crate::AuthorPubkey;

/// Identity scope: long-term author + optional instance binding.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct IdentityScope {
    /// Durable user/author/member identity. Always Ed25519.
    pub long_term: AuthorPubkey,
    /// Short-lived per-(peer, instance) signing scope nested under the
    /// long-term identity. `None` means the long-term identity signs
    /// directly.
    pub instance: Option<InstanceBinding>,
}

/// Per-(peer, instance) binding nested under a long-term identity.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct InstanceBinding {
    /// The peer this instance is bound to. For behaviors and devices,
    /// this is the per-peer keypair the kernel materialized at instance
    /// creation time.
    pub peer: AuthorPubkey,
    /// What kind of instance binding this is.
    pub kind: InstanceKind,
    /// Human-readable name for the instance (e.g. `"discord-bridge-1"`).
    pub name: String,
}

/// Kind of instance binding nested under a long-term identity.
#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum InstanceKind {
    /// Per-device sub-identity for multi-device users.
    Device,
    /// Behavior bot signing scope (behavior profile per architecture.md §3.4).
    Behavior,
    /// MLS leaf signing scope.
    MlsLeaf,
    /// Application-defined kind with a name.
    Custom(String),
}

/// Profile-level annotation used for author-policy checks per
/// [identity.md §6.1].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum CallingProfile {
    /// Strict pure-fn state mutation profile.
    StateApply,
    /// Loose intent-to-event proposal profile.
    StatePropose,
    /// Per-peer UI surface profile.
    Interaction,
    /// Bots, bridges, automations.
    Behavior,
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use bincode::Options;

    #[test]
    fn instance_kind_round_trips() {
        for kind in [
            InstanceKind::Device,
            InstanceKind::Behavior,
            InstanceKind::MlsLeaf,
            InstanceKind::Custom("epoch-42".into()),
        ] {
            let bytes = crate::canonical_bincode().serialize(&kind).expect("encode");
            let decoded: InstanceKind = crate::canonical_bincode()
                .deserialize(&bytes)
                .expect("decode");
            assert_eq!(kind, decoded);
        }
    }

    #[test]
    fn identity_scope_no_instance_serializes() {
        let scope = IdentityScope {
            long_term: AuthorPubkey::from_bytes([1; 32]),
            instance: None,
        };
        let bytes = crate::canonical_bincode()
            .serialize(&scope)
            .expect("encode");
        let decoded: IdentityScope = crate::canonical_bincode()
            .deserialize(&bytes)
            .expect("decode");
        assert_eq!(scope, decoded);
    }

    #[test]
    fn identity_scope_with_behavior_instance_serializes() {
        let scope = IdentityScope {
            long_term: AuthorPubkey::from_bytes([2; 32]),
            instance: Some(InstanceBinding {
                peer: AuthorPubkey::from_bytes([3; 32]),
                kind: InstanceKind::Behavior,
                name: "discord-bridge-1".into(),
            }),
        };
        let bytes = crate::canonical_bincode()
            .serialize(&scope)
            .expect("encode");
        let decoded: IdentityScope = crate::canonical_bincode()
            .deserialize(&bytes)
            .expect("decode");
        assert_eq!(scope, decoded);
    }
}
