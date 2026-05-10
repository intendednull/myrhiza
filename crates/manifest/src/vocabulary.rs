//! V1 capability vocabulary.
//!
//! Mirrors the architecture.md §3.5 host import table plus the v1 ui:*
//! minimum vocabulary from distribution.md §10.2. Adding an entry is
//! a kernel minor bump; if the entry is bindable by state-apply, it
//! is a kernel major bump.

/// Classification of capability strings per the architecture.md §3.5
/// table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CapabilityClass {
    /// Bound automatically for state-apply (deterministic helper set).
    DeterministicHelper,
    /// Capability-gated for the profile that may bind it.
    HostImport,
    /// Per-call gated; manifest re-checked at every invocation.
    HighValueOp,
    /// `ui:*` surface; declared by interaction profile.
    UiSurface,
}

const V1_VOCABULARY: &[(&str, CapabilityClass)] = &[
    // Deterministic helper set — state-apply may bind.
    ("host.hash", CapabilityClass::DeterministicHelper),
    (
        "host.verify-signature",
        CapabilityClass::DeterministicHelper,
    ),
    (
        "host.verify-payload-mac",
        CapabilityClass::DeterministicHelper,
    ),
    ("host.install-key", CapabilityClass::DeterministicHelper),
    (
        "host.now-hlc-from-event",
        CapabilityClass::DeterministicHelper,
    ),
    ("host.log", CapabilityClass::DeterministicHelper),
    // Non-deterministic host imports (state-propose, interaction, behavior).
    ("host.hlc", CapabilityClass::HostImport),
    ("host.random", CapabilityClass::HostImport),
    ("host.author-event", CapabilityClass::HostImport),
    // Capability key is `host.broadcast` per architecture.md §3.5; the
    // WIT wire name `broadcast-submit` (per abi.md §8.5) is the
    // kernel-side import binding and is unchanged.
    ("host.broadcast", CapabilityClass::HostImport),
    ("host.subscribe", CapabilityClass::HostImport),
    ("host.kv.get", CapabilityClass::HostImport),
    ("host.kv.put", CapabilityClass::HostImport),
    ("host.kv.delete", CapabilityClass::HostImport),
    ("host.kv.list-prefix", CapabilityClass::HostImport),
    ("host.user-prompt", CapabilityClass::HostImport),
    ("host.seal", CapabilityClass::HostImport),
    ("host.open", CapabilityClass::HostImport),
    ("host.can-open", CapabilityClass::HostImport),
    ("host.x25519-ecdh", CapabilityClass::HostImport),
    ("host.hkdf-derive", CapabilityClass::HostImport),
    ("host.timer.schedule", CapabilityClass::HostImport),
    ("host.timer.cancel", CapabilityClass::HostImport),
    // High-value ops — per-call gated.
    ("host.aead-seal", CapabilityClass::HighValueOp),
    ("host.aead-open", CapabilityClass::HighValueOp),
    ("host.http.request", CapabilityClass::HighValueOp),
    ("host.clipboard.write", CapabilityClass::HighValueOp),
    ("host.file-picker.show", CapabilityClass::HighValueOp),
    ("host.navigation.top-level", CapabilityClass::HighValueOp),
    ("host.push.register", CapabilityClass::HighValueOp),
    // V1 ui:* minimum vocabulary.
    ("ui:panel", CapabilityClass::UiSurface),
    ("ui:list", CapabilityClass::UiSurface),
    ("ui:message", CapabilityClass::UiSurface),
    ("ui:form", CapabilityClass::UiSurface),
    ("ui:menu", CapabilityClass::UiSurface),
    ("ui:button", CapabilityClass::UiSurface),
    ("ui:input", CapabilityClass::UiSurface),
    ("ui:dialog", CapabilityClass::UiSurface),
];

/// Returns true iff the capability string is part of the v1 vocabulary.
#[must_use]
pub fn known_capability(s: &str) -> bool {
    V1_VOCABULARY.iter().any(|(name, _)| *name == s)
}

/// Returns the class of a known capability, or `None` if unknown.
#[must_use]
pub fn classify(s: &str) -> Option<CapabilityClass> {
    V1_VOCABULARY
        .iter()
        .find(|(name, _)| *name == s)
        .map(|(_, c)| *c)
}

/// Iterator over every (name, class) pair in the v1 vocabulary.
pub fn iter_v1() -> impl Iterator<Item = (&'static str, CapabilityClass)> {
    V1_VOCABULARY.iter().copied()
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn known_host_imports_include_helper_set() {
        for cap in [
            "host.hash",
            "host.verify-signature",
            "host.verify-payload-mac",
            "host.install-key",
            "host.now-hlc-from-event",
            "host.log",
        ] {
            assert!(known_capability(cap), "{cap} must be in vocabulary");
        }
    }

    #[test]
    fn unknown_capability_rejected() {
        assert!(!known_capability("host.invented-by-app"));
        assert!(!known_capability("ui:not-real"));
    }

    #[test]
    fn ui_minimum_vocabulary_listed() {
        for cap in [
            "ui:panel",
            "ui:list",
            "ui:message",
            "ui:form",
            "ui:menu",
            "ui:button",
            "ui:input",
            "ui:dialog",
        ] {
            assert!(known_capability(cap), "{cap} must be in v1 ui vocabulary");
        }
    }

    #[test]
    fn deterministic_helpers_are_classified() {
        assert_eq!(
            classify("host.hash"),
            Some(CapabilityClass::DeterministicHelper)
        );
        assert_eq!(
            classify("host.broadcast"),
            Some(CapabilityClass::HostImport)
        );
        // Old name is no longer in vocabulary — capability key matches
        // architecture.md §3.5; WIT wire-name `broadcast-submit` is
        // separate (per abi.md §8.5).
        assert_eq!(classify("host.broadcast-submit"), None);
        assert_eq!(
            classify("host.clipboard.write"),
            Some(CapabilityClass::HighValueOp)
        );
        assert_eq!(classify("ui:panel"), Some(CapabilityClass::UiSurface));
        assert_eq!(classify("host.invented"), None);
    }
}
