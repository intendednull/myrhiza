**Date:** 2026-05-26
**Status:** active
**Spec:** [docs/specs/2026-05-26-b-10-bundle-distribution-design.md](../specs/2026-05-26-b-10-bundle-distribution-design.md)
**Subject:** Plan B-10 — Bundle distribution + iroh-blobs fetch implementation

# Plan B-10 implementation — Bundle distribution + iroh-blobs

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Spec at [`docs/specs/2026-05-26-b-10-bundle-distribution-design.md`](../specs/2026-05-26-b-10-bundle-distribution-design.md) is the design contract; this plan is the execution order. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close `mvp.md` §15.1 criterion #1 against the iroh-blobs wire shape (vs the disk-only proxy currently passing) and finish `implementation.md` §20 item 14 by landing (a) `iroh-blobs 0.101.0` as a feature-gated dependency, (b) a new `crates/distribution/` workspace member owning the `BundleDistribution` handle (publish + fetch), (c) per-author revocation + publication topics with monotonic-seq state machines, (d) a `BlobHash` newtype + `*_hash: Option<BlobHash>` manifest fields, (e) a `BundleAddress::{Disk,IrohBlob}` enum (additive — Disk path unchanged), and (f) one kernel-tier acceptance test exercising the full publish-on-A → fetch-on-B → instantiate → assert-state loop through real iroh-blobs over loopback QUIC via an extended `IrohHarness`.

**Architecture:** 13 tasks T0–T12. Each task is one commit producing a buildable tree. State-tier tests (T5–T6 revocation + publication purity) precede the iroh-blobs publish/fetch impls (T7–T9). Kernel-tier acceptance (T10–T11) follows infra. New crate `crates/distribution/` (feature `network-iroh`) owns iroh-blobs + topic schema + log state machines; `crates/kernel` adds it as a dep and dispatches the `IrohBlob` `BundleAddress` variant through `BundleDistribution::fetch → MaterializedBundle::Disk` then through the existing `InstallFlow::load`. `crates/types::BlobHash` is a 32-byte newtype mirroring `EventHash`; lives in `crates/types` so `crates/manifest` carries it without taking on iroh.

**Tech Stack:** Rust 2024, existing iroh `=1.0.0-rc.0` + iroh-gossip `=0.99.0`, new workspace dep `iroh-blobs = "=0.101.0"` (declares `iroh = "=1.0.0-rc.0"` per spec §4.7 — compatibility verified against crates.io 2026-05-26; T0 re-verifies at execution time). Existing `tempfile`, `blake3`, `ed25519-dalek`, `bincode`, `serde`, `thiserror`, `async-trait`, `tokio`, `futures-lite`, `bytes`.

**Spec:** `docs/specs/2026-05-26-b-10-bundle-distribution-design.md` (review-clean per `beba744`).

**Branch:** `feat/b-10-bundle-distribution` (already exists; spec is committed there).

**Common workflow per task:**

1. Dispatch implementer subagent with the task's *Files touched* + *Implementation notes* + *Verification commands* sections + spec-section refs.
2. After implementer completes: dispatch fresh spec-compliance reviewer.
3. After spec compliance is met: dispatch fresh code-quality reviewer.
4. After both are met: commit and proceed.
5. After T12 lands: dispatch a fresh final-review agent across the entire branch before opening the PR.

---

## Pre-flight

- Worktree: `/mnt/storage/projects/myrhiza/.claude/worktrees/b-10-bundle-distribution/`
- Branch: `feat/b-10-bundle-distribution`
- Base: `main` (the spec sits at `beba744` on this branch; HEAD of `main` at `5999c9e`).
- Confirm tree clean (`git status` clean) before each task. If a task's verification commands fail, fix root cause — never `--no-verify`.
- All `cargo` commands run from workspace root (`/mnt/storage/projects/myrhiza/.claude/worktrees/b-10-bundle-distribution/`).

---

## Task T0 — Workspace `iroh-blobs = "=0.101.0"` dep + transitive version reconciliation

**Spec ref:** §4.7 (version-compat note); §5 risk row 1 (API churn under exact-version pin).

**Subject:** Add `iroh-blobs` to `[workspace.dependencies]` at `=0.101.0` and verify the transitive `iroh = =1.0.0-rc.0` declaration matches the workspace's existing iroh pin — re-verify at execution time per spec §4.7 contingency.

**Files touched:**
- Modify: `Cargo.toml` (workspace `[workspace.dependencies]`)
- Lockfile: `Cargo.lock` (regenerated)

**Implementation notes:**

In workspace `Cargo.toml`'s `[workspace.dependencies]`, after the existing `iroh-gossip = "=0.99.0"` line (currently line 62 per the worktree state), add:

```toml
# iroh-blobs is the data-plane sibling of iroh-gossip. Pinned to
# 0.101.0 (latest as of 2026-05-08); aware that this is the
# "rewrite line" the iroh team flags as "not yet production
# quality" — see prior-art/iroh/blobs.md §"What's actually
# shipping right now". Tight-pinned in lockstep with the rest of
# the iroh ecosystem per prior-art/iroh/lessons.md §Avoid row 1.
# iroh-blobs 0.101.0 declares `iroh = "=1.0.0-rc.0"` transitively,
# matching the workspace's iroh pin (verified against crates.io
# 2026-05-26; see B-10 spec §4.7). Bump deliberately.
iroh-blobs = "=0.101.0"
```

No crate-level use yet — that comes in T1. This task is the workspace-level pin + lockfile update only.

**Verification commands:**

```bash
cargo metadata --format-version=1 > /dev/null

# Assert iroh-blobs is pinned to exactly 0.101.0.
[ "$(cargo tree -p iroh-blobs 2>&1 | grep -E '^iroh-blobs v0\.101\.0' | wc -l)" = "1" ] \
    && echo "iroh-blobs 0.101.0 OK" \
    || { echo "MISMATCH: iroh-blobs not at 0.101.0"; exit 1; }

# Assert iroh transitively resolves to exactly 1.0.0-rc.0, single version.
[ "$(cargo tree -p iroh 2>&1 | grep -E '^iroh v=?1\.0\.0-rc\.0' | wc -l)" = "1" ] \
    && echo "iroh 1.0.0-rc.0 OK" \
    || { echo "MISMATCH: iroh not exactly 1.0.0-rc.0 or duplicated"; exit 1; }

cargo check --workspace
cargo check --workspace --all-features
```

**Risk:** If `cargo tree` surfaces conflicting iroh versions (e.g. iroh-blobs 0.101.0 transitively pulled `iroh = 0.98.x`), the spec §4.7 contingency path (a) is to bump both pins together. If iroh-blobs 0.101.0 has been yanked from crates.io between spec-write (2026-05-26) and execution time, escalate to the spec author — do NOT silently bump to a different version, as that would invalidate the spec's API-compat verification.

**Commit message:**
```
chore(deps): add iroh-blobs 0.101.0 to workspace (B-10 §4.7)

iroh-blobs 0.101.0 declares iroh = "=1.0.0-rc.0" transitively,
matching workspace pin. T0 re-verifies via cargo tree.
```

---

## Task T1 — `crates/types::BlobHash` 32-byte newtype

**Spec ref:** §4.2 (`BlobHash` newtype + orphan-rule conversions); §4.6 (crate dependency direction — `BlobHash` lives in `crates/types` so `crates/manifest` carries it without iroh).

**Subject:** Add `BlobHash` to `crates/types/src/hash.rs` as a 32-byte newtype mirroring `EventHash`'s shape; expose at crate root. State-tier unit tests confirm canonical-bincode round-trip + display + size invariants.

**Files touched:**
- Modify: `crates/types/src/hash.rs` (add `BlobHash` struct + impls + tests)
- Modify: `crates/types/src/lib.rs` (re-export)

**Implementation notes:**

`BlobHash` shape mirrors `BundleHash` and `EventHash` already in `hash.rs`:

```rust
/// 32-byte BLAKE3 hash addressing a blob in an iroh-blobs store.
///
/// Thin newtype over `[u8; 32]` matching `EventHash` / `BundleHash`
/// shape so the type system distinguishes "blob content addressing"
/// from "event hash" from "bundle identity" even though all three
/// are BLAKE3-sized. Lives in `myrhiza-types` (not `myrhiza-network`
/// nor `myrhiza-distribution`) so `myrhiza-manifest` can declare
/// `Option<BlobHash>` fields without taking on iroh as a dependency.
///
/// Conversion to/from `iroh_blobs::Hash` lives in
/// `myrhiza-distribution` as free fns (orphan rule — same shape as
/// `peer_pubkey_from_iroh` in B-4.0).
///
/// Per B-10 spec §4.2 + §4.6.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BlobHash(#[serde(with = "crate::hash::serde_bytes_32_pub")] [u8; 32]);

impl BlobHash {
    /// Sentinel zero hash (32 zero bytes). Used by tests; not a
    /// valid iroh-blobs address.
    pub const ZERO: BlobHash = BlobHash([0u8; 32]);

    /// Wrap a 32-byte array as a `BlobHash`. No validation —
    /// caller must have computed it via BLAKE3 over the blob bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Borrow the raw 32 bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Compute the `BlobHash` from arbitrary bytes via BLAKE3.
    ///
    /// Convenience for tests + publish-side code that has the
    /// blob bytes in hand. Production fetch path receives the hash
    /// from the wire and never recomputes it (iroh-blobs verifies).
    #[must_use]
    pub fn blake3(bytes: &[u8]) -> Self {
        let hash = blake3::hash(bytes);
        Self(*hash.as_bytes())
    }
}

impl fmt::Display for BlobHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for BlobHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BlobHash({self})")
    }
}
```

Top of `hash.rs` already imports `serde` + `std::fmt`. The `serde_bytes_32_pub` module is already used by `EventHash` / `BundleHash`.

In `crates/types/src/lib.rs` line 12, extend the existing re-export:

```rust
pub use hash::{BlobHash, BundleHash, EventHash};
```

**State-tier tests** (in `hash.rs::tests`, mirroring existing tests):

```rust
#[test]
fn blob_hash_size_is_32() {
    assert_eq!(core::mem::size_of::<BlobHash>(), 32);
}

#[test]
fn blob_hash_from_bytes_roundtrips_through_canonical_bincode() {
    let raw = [0x42; 32];
    let h = BlobHash::from_bytes(raw);
    let encoded = crate::canonical_bincode().serialize(&h).expect("encode");
    let decoded: BlobHash = crate::canonical_bincode()
        .deserialize(&encoded)
        .expect("decode");
    assert_eq!(h, decoded);
    assert_eq!(decoded.as_bytes(), &raw);
}

#[test]
fn blob_hash_blake3_matches_blake3_crate() {
    let h = BlobHash::blake3(b"hello");
    let expected = blake3::hash(b"hello");
    assert_eq!(h.as_bytes(), expected.as_bytes());
}

#[test]
fn blob_hash_display_is_hex() {
    let h = BlobHash::from_bytes([0xDE; 32]);
    let s = format!("{h}");
    assert_eq!(s.len(), 64);
    assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn blob_hash_zero_sentinel() {
    assert_eq!(BlobHash::ZERO.as_bytes(), &[0u8; 32]);
}
```

**Verification commands:**

```bash
cargo test -p myrhiza-types --lib hash::tests
cargo clippy -p myrhiza-types --all-targets -- -D warnings
cargo build --workspace
```

**Risk:** None — pure additive change, mechanically obvious.

**Commit message:**
```
feat(types): add BlobHash 32-byte newtype (B-10 §4.2)

Mirrors EventHash/BundleHash shape. Lives in myrhiza-types so
myrhiza-manifest can carry Option<BlobHash> fields without iroh.
Conversion to/from iroh_blobs::Hash lands in myrhiza-distribution
(T2) as orphan-rule free fns.
```

---

## Task T2 — `crates/distribution/` skeleton + Cargo wiring + orphan-rule conversions

**Spec ref:** §3.7 (new `crates/distribution/` workspace member); §4.6 (dependency direction); §4.7 (Cargo.toml shape — `network-iroh` feature, optional deps).

**Subject:** Create the new workspace member `crates/distribution/` with a `network-iroh`-feature-gated module shell, the `BlobHash ↔ iroh_blobs::Hash` orphan-rule conversion free fns, and `derive_revocation_topic` + `derive_publication_topic` topic-derivation helpers.

**Files touched:**
- Modify: `Cargo.toml` (workspace `members` list)
- Create: `crates/distribution/Cargo.toml`
- Create: `crates/distribution/src/lib.rs`
- Create: `crates/distribution/src/conversions.rs`
- Create: `crates/distribution/src/topic.rs`

**Implementation notes:**

`Cargo.toml`'s `members` array (currently 8 entries, last is `crates/myrhiza-cli`) gains one entry:

```toml
members = [
    "crates/types",
    "crates/manifest",
    "crates/backend",
    "crates/wasmtime-backend",
    "crates/kernel",
    "crates/network",
    "crates/distribution",
    "crates/test-utils",
    "crates/myrhiza-cli",
]
```

`crates/distribution/Cargo.toml`:

```toml
[package]
name = "myrhiza-distribution"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true

[dependencies]
myrhiza-types = { path = "../types" }
myrhiza-manifest = { path = "../manifest" }
myrhiza-network = { path = "../network" }
async-trait = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
thiserror = { workspace = true }
bincode = { workspace = true }
tempfile = { workspace = true }
blake3 = { workspace = true }
ed25519-dalek = { workspace = true }
iroh = { workspace = true, optional = true }
iroh-blobs = { workspace = true, optional = true }

[features]
default = []
# Iroh-blobs publish + fetch + revocation/publication topic dispatch.
# Default-off to keep `cargo test --workspace` fast; the iroh-acceptance
# tests in crates/kernel/tests opt in via `network-iroh`. Per B-10
# spec §4.7.
network-iroh = ["dep:iroh", "dep:iroh-blobs", "myrhiza-network/network-iroh"]

[lints.clippy]
# Test-only convenience escapes don't apply here — distribution is
# library code that may run in app-facing paths. Workspace-wide
# warns stay.
```

`crates/distribution/src/lib.rs` skeleton:

```rust
//! Bundle distribution: iroh-blobs-backed publish + fetch + per-author
//! revocation and publication topic schema + monotonic-seq state
//! machines.
//!
//! Per B-10 design at
//! `docs/specs/2026-05-26-b-10-bundle-distribution-design.md`.
//!
//! ## Feature gates
//!
//! - `network-iroh` (default-off): pulls in `iroh` + `iroh-blobs`
//!   and unlocks `BundleDistribution` (publish + fetch). The pure-
//!   function state machines (`RevocationLog`, `PublicationLog`)
//!   compile feature-free — they're used by every install regardless
//!   of transport.
//!
//! ## Crate dependency direction (per spec §4.6)
//!
//! ```text
//! crates/distribution/
//!     depends-on: crates/types, crates/manifest, crates/network
//!     feature-gate: network-iroh (pulls in iroh + iroh-blobs)
//!
//! crates/kernel/
//!     depends-on: crates/distribution (new dep — kernel dispatches
//!                 BundleAddress::IrohBlob through BundleDistribution
//!                 before falling back to InstallFlow::load on the
//!                 materialized Disk variant)
//! ```
//!
//! No circular deps. `crates/manifest` does NOT depend on
//! `crates/distribution` — the `BlobHash` newtype lives in
//! `crates/types` (per spec §4.6).

#![deny(missing_docs)]

pub mod conversions;
pub mod topic;

// State machines land in T5 (revocation) + T6 (publication).
// pub mod revocation;
// pub mod publication;

// Iroh-blobs publish + fetch lands in T7.
// #[cfg(feature = "network-iroh")]
// pub mod blobs;
// #[cfg(feature = "network-iroh")]
// pub use blobs::{BundleDistribution, FetchError, MaterializedBundle, PublishError};

pub use topic::{derive_publication_topic, derive_revocation_topic};

#[cfg(feature = "network-iroh")]
pub use conversions::{blob_hash_from_iroh, iroh_hash_from_blob_hash};
```

`crates/distribution/src/conversions.rs`:

```rust
//! `BlobHash` <-> `iroh_blobs::Hash` orphan-rule conversions.
//!
//! Free functions (not `From`/`Into` impls) because neither type is
//! local to this crate: `BlobHash` lives in `myrhiza-types` and
//! `iroh_blobs::Hash` is foreign. Same shape as
//! `peer_pubkey_from_iroh` in B-4.0 (`crates/network/src/iroh_transport.rs`).
//!
//! Feature-gated on `network-iroh` because they reference `iroh_blobs`.
//! Per B-10 spec §4.2.

#![cfg(feature = "network-iroh")]

use myrhiza_types::BlobHash;

/// Convert an `iroh_blobs::Hash` to a `myrhiza_types::BlobHash`.
///
/// Both wrap a 32-byte BLAKE3 hash. Zero-cost.
#[must_use]
pub fn blob_hash_from_iroh(h: iroh_blobs::Hash) -> BlobHash {
    BlobHash::from_bytes(*h.as_bytes())
}

/// Convert a `myrhiza_types::BlobHash` to an `iroh_blobs::Hash`.
///
/// Both wrap a 32-byte BLAKE3 hash. Zero-cost. Infallible — both
/// types accept arbitrary 32-byte arrays (BLAKE3 is not curve-typed).
#[must_use]
pub fn iroh_hash_from_blob_hash(h: BlobHash) -> iroh_blobs::Hash {
    iroh_blobs::Hash::from_bytes(*h.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_via_iroh_blobs_hash() {
        let original = BlobHash::blake3(b"counter-state-apply.wasm");
        let iroh = iroh_hash_from_blob_hash(original);
        let back = blob_hash_from_iroh(iroh);
        assert_eq!(original, back);
        assert_eq!(original.as_bytes(), back.as_bytes());
    }
}
```

`crates/distribution/src/topic.rs`:

```rust
//! Topic-id derivation for revocation + publication per-author topics.
//!
//! Both topics are derived from the author's pubkey + a domain-
//! separator string. Mirrors the per-author-Merkle-DAG topic shape
//! from `myrhiza-manifest::derive_topic_normalized` (which derives
//! per-bundle topics).
//!
//! Per B-10 spec §3.3 (revocation) + §3.4 (publication).

use myrhiza_types::{AuthorPubkey, Topic};

/// Domain-separator string for revocation topics. Per
/// `docs/specs/2026-05-09-myrhiza-master-design/distribution.md`
/// §10.7. Framed alongside the author pubkey via BLAKE3.
pub const REVOCATION_TOPIC_DOMAIN: &[u8] = b"myrhiza/revocations/v1";

/// Domain-separator string for publication topics. Per B-10 spec §3.4.
pub const PUBLICATION_TOPIC_DOMAIN: &[u8] = b"myrhiza/publications/v1";

/// Derive the per-author revocation topic id.
///
/// `topic_id = BLAKE3("myrhiza/revocations/v1" || author_pubkey)`.
/// Per B-10 spec §3.3.
#[must_use]
pub fn derive_revocation_topic(author: AuthorPubkey) -> Topic {
    derive_per_author_topic(REVOCATION_TOPIC_DOMAIN, author)
}

/// Derive the per-author publication topic id.
///
/// `topic_id = BLAKE3("myrhiza/publications/v1" || author_pubkey)`.
/// Per B-10 spec §3.4.
#[must_use]
pub fn derive_publication_topic(author: AuthorPubkey) -> Topic {
    derive_per_author_topic(PUBLICATION_TOPIC_DOMAIN, author)
}

fn derive_per_author_topic(domain: &[u8], author: AuthorPubkey) -> Topic {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain);
    hasher.update(author.as_bytes());
    let hash = hasher.finalize();
    Topic::from_bytes(*hash.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_author() -> AuthorPubkey {
        AuthorPubkey::from_bytes([0x7Au8; 32])
    }

    #[test]
    fn revocation_topic_is_deterministic_per_author() {
        let a = derive_revocation_topic(fixture_author());
        let b = derive_revocation_topic(fixture_author());
        assert_eq!(a, b);
    }

    #[test]
    fn publication_and_revocation_topics_differ_for_same_author() {
        let r = derive_revocation_topic(fixture_author());
        let p = derive_publication_topic(fixture_author());
        assert_ne!(
            r, p,
            "domain separators must differentiate revocation from publication"
        );
    }

    #[test]
    fn different_authors_get_different_revocation_topics() {
        let a = derive_revocation_topic(AuthorPubkey::from_bytes([0x11; 32]));
        let b = derive_revocation_topic(AuthorPubkey::from_bytes([0x22; 32]));
        assert_ne!(a, b);
    }
}
```

Verify `AuthorPubkey::from_bytes` + `AuthorPubkey::as_bytes` exist (per `crates/types/src/author.rs` — they do by the existing pattern of `PeerPubkey` shape; if a constructor differs in name, adapt). `Topic::from_bytes` exists per B-1 (verified at `crates/types/src/topic.rs`).

**Verification commands:**

```bash
cargo build -p myrhiza-distribution
cargo build -p myrhiza-distribution --features network-iroh
cargo test -p myrhiza-distribution --lib
cargo test -p myrhiza-distribution --lib --features network-iroh
cargo clippy -p myrhiza-distribution --all-targets -- -D warnings
cargo clippy -p myrhiza-distribution --all-targets --features network-iroh -- -D warnings
cargo build --workspace
```

**Risk:** Workspace-member addition must propagate to all `cargo` workspace commands. If `cargo check --workspace` previously skipped a path-dep crate, adding `crates/distribution/` to the members list now exposes it. Mitigation: the dep on `myrhiza-network` is path-local + workspace-feature-controlled; no risk of pulling iroh into the default `cargo build --workspace` path.

**Commit message:**
```
feat(distribution): scaffold crates/distribution/ with conversions + topic derivation (B-10 §3.7 + §4.2)

New workspace member. Feature-gated on network-iroh. Provides:
- BlobHash <-> iroh_blobs::Hash orphan-rule conversions
- derive_revocation_topic + derive_publication_topic helpers
State machines (RevocationLog, PublicationLog) + iroh-blobs
publish/fetch land in later tasks.
```

---

## Task T3 — `Manifest::ComponentsSection::*_hash` fields (additive)

**Spec ref:** §4.1 (manifest schema delta — pick option 2: parallel hash fields alongside path fields, additive).

**Subject:** Add four `Option<BlobHash>` fields to `ComponentsSection` parallel to the existing four `Option<String>` path fields (`state_apply_hash`, `state_propose_hash`, `interaction_hash`, `behavior_hash`). Existing manifest signing target is unchanged by intent — canonical-bincode includes the new fields automatically, so byte-stability of existing fixture signatures must be re-verified.

**Files touched:**
- Modify: `crates/manifest/Cargo.toml` (add `myrhiza-types` if missing — it's already there)
- Modify: `crates/manifest/src/schema.rs` (add 4 hash fields to `ComponentsSection`)
- Modify: `crates/manifest/src/canonical.rs::tests` (extend `sample_manifest` to populate new fields with `None`)
- Modify: any in-crate `ComponentsSection { ... }` literal construction (likely just `tests`)
- Modify: `crates/test-utils/src/bundle.rs` — extend `build_signed_counter_bundle` + `build_signed_counter_bundle_three_components` to compute `BlobHash::blake3` over each on-disk component's bytes and populate the corresponding `*_hash` field before signing. State-tier test asserts the populated hashes match the on-disk blob bytes.

**Implementation notes:**

In `crates/manifest/src/schema.rs`, locate the `ComponentsSection` struct definition (currently around line 151). Add four new `Option<BlobHash>` fields parallel to the existing path fields, immediately after each path field:

```rust
use myrhiza_types::BlobHash;

// ... existing imports ...

/// `[components]` section: per-profile artifact file paths and
/// content-addressed blob hashes.
///
/// Each profile has TWO fields:
/// - `<profile>: Option<String>` — file path within the bundle dir
///   (used for disk-loaded bundles). Existing behavior; unchanged.
/// - `<profile>_hash: Option<BlobHash>` — iroh-blobs content hash
///   (used for iroh-blobs-fetched bundles). Per B-10 spec §4.1.
///
/// **Both may be `Some` simultaneously** — a publish-side bundle
/// populates the path (informative filename in the canonical
/// layout) and the hash (load-bearing for iroh-blobs fetch). Disk-
/// only fixtures may leave `_hash` as `None`; iroh-loaded bundles
/// reject manifests missing `_hash` for any declared component
/// (enforced in T8's BundleDistribution::fetch, NOT in
/// myrhiza-manifest itself — schema stays universal).
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComponentsSection {
    /// Path to the state-apply component within the bundle dir.
    pub state_apply: Option<String>,
    /// BLAKE3 (iroh-blobs) hash of the state-apply component bytes.
    /// Per B-10 spec §4.1.
    pub state_apply_hash: Option<BlobHash>,
    /// Path to the state-propose component.
    pub state_propose: Option<String>,
    /// BLAKE3 (iroh-blobs) hash of the state-propose component bytes.
    /// Per B-10 spec §4.1.
    pub state_propose_hash: Option<BlobHash>,
    /// Path to the interaction component.
    pub interaction: Option<String>,
    /// BLAKE3 (iroh-blobs) hash of the interaction component bytes.
    /// Per B-10 spec §4.1.
    pub interaction_hash: Option<BlobHash>,
    /// Path to the behavior component.
    pub behavior: Option<String>,
    /// BLAKE3 (iroh-blobs) hash of the behavior component bytes.
    /// Per B-10 spec §4.1.
    pub behavior_hash: Option<BlobHash>,
}
```

> **CALLOUT — byte-stability concession (do NOT skim past):**
>
> Field order matters for canonical-bincode determinism: add each
> `_hash` field IMMEDIATELY after its sibling path. This grouping is
> the convention; canonical-bincode is positional so adding fields at
> the end vs interleaved both produce different bytes — pick
> interleaved (matches the spec's diagram in §4.1).
>
> A schema delta to `ComponentsSection` IS a manifest-bytes break:
> any pre-existing on-disk `manifest.bincode` from before B-10 no
> longer decodes under the new schema, and any pre-signed manifest
> fixture's signature no longer re-verifies under the new field
> layout. This is acceptable because B-10 has not yet shipped any
> persisted manifests outside test tempdirs (see Risk block below for
> the grep verification step). Future schema deltas after B-10 ships
> production manifests will require a versioned envelope; that's out
> of scope here.

**Existing fixtures that construct `ComponentsSection` literally:**

Update each to add the four new fields as `None` (disk-only path bundles) OR with computed `BlobHash::blake3` (iroh-path bundles). Per `grep -rn "ComponentsSection {" crates/`:

- `crates/manifest/src/canonical.rs::sample_manifest` — add 4 `None`s after each `state_apply`/`state_propose`/etc.
- `crates/kernel/src/install.rs` (lines ~277-281, ~358-364) — extend test fixtures `write_fixture_bundle` + `write_two_component_fixture_bundle`. These remain disk-only paths; `*_hash` fields stay `None`.
- `crates/test-utils/src/bundle.rs` — `helpers_only_state_apply_manifest()`, `helpers_only_three_component_manifest()`, `helpers_only_state_apply_manifest_with_extra_cap()` — extend each.

**Fixture-builder extension (the iroh-path bundles):** `build_signed_counter_bundle` and `build_signed_counter_bundle_three_components` in `crates/test-utils/src/bundle.rs` are the fixtures used by T11's kernel-tier acceptance test for the publish-on-A → fetch-on-B loop. They MUST populate the `*_hash` fields with the actual BLAKE3 of each on-disk component before signing the manifest, so the manifest signature commits to the hash claim and `BundleDistribution::fetch` can cross-check.

Concretely, after building each `Vec<u8>` of component bytes and before constructing the `ComponentsSection`:

```rust
let state_apply_bytes: Vec<u8> = std::fs::read(&state_apply_path).expect("read");
let state_apply_hash = myrhiza_types::BlobHash::blake3(&state_apply_bytes);
// repeat for state_propose, interaction, behavior (if present)

let components = ComponentsSection {
    state_apply: Some("components/state-apply.wasm".into()),
    state_apply_hash: Some(state_apply_hash),
    state_propose: Some("components/state-propose.wasm".into()),
    state_propose_hash: Some(state_propose_hash),
    // ...
};
```

The signing path is unchanged — `signed_body_bytes` already serializes the entire `ComponentsSection` and the signature commits to it; the only delta is that the populated `*_hash` fields now carry load-bearing values instead of `None`.

**Signing-target byte stability check:** the manifest signature commits to `canonical_bincode(SignedBody)` per `crates/manifest/src/canonical.rs::signed_body_bytes`, where `SignedBody` includes `components`. Adding the new hash fields to `ComponentsSection` means **existing test fixtures' signatures will not re-verify** after the schema update — every fixture that hard-codes a manifest and signs it must be re-signed with the new schema.

**Test coverage to add:** in `crates/manifest/src/canonical.rs::tests`, add a regression test that the canonical-bincode of a `ComponentsSection` with ALL `*_hash` fields `None` is BYTE-IDENTICAL to the canonical-bincode of the **pre-B-10 shape** would have been — NO. That's impossible (bincode 1.3.x positional encoding makes adding fields a wire break). Instead, add a forward-compat test that explicitly populates the new hash fields and verifies they round-trip:

```rust
#[test]
fn components_section_hash_fields_roundtrip() {
    use crate::schema::ComponentsSection;
    use myrhiza_types::BlobHash;

    let cs = ComponentsSection {
        state_apply: Some("components/state-apply.wasm".into()),
        state_apply_hash: Some(BlobHash::from_bytes([0xAA; 32])),
        state_propose: None,
        state_propose_hash: None,
        interaction: Some("components/interaction.wasm".into()),
        interaction_hash: Some(BlobHash::from_bytes([0xBB; 32])),
        behavior: None,
        behavior_hash: None,
    };
    let bytes = myrhiza_types::canonical_bincode().serialize(&cs).expect("encode");
    let decoded: ComponentsSection = myrhiza_types::canonical_bincode()
        .deserialize(&bytes)
        .expect("decode");
    assert_eq!(cs, decoded);
}
```

**Fixture-builder hash-matching state-tier test** (in `crates/test-utils/src/bundle.rs::tests` or `crates/test-utils/tests/`, whichever the existing test pattern uses):

```rust
#[test]
fn build_signed_counter_bundle_populates_hashes_matching_on_disk_bytes() {
    use myrhiza_manifest::canonical;
    use myrhiza_types::{BlobHash, canonical_bincode};

    let bundle = build_signed_counter_bundle();
    // Decode the signed manifest written by the fixture.
    let manifest_bytes = std::fs::read(
        bundle.bundle_dir.join(&bundle.manifest_path),
    ).expect("read manifest");
    let manifest: myrhiza_manifest::schema::Manifest = canonical_bincode()
        .deserialize(&manifest_bytes)
        .expect("decode manifest");
    let on_disk_bytes = std::fs::read(
        bundle.bundle_dir.join(manifest.components.state_apply.as_deref().unwrap()),
    ).expect("read state-apply");

    assert_eq!(
        manifest.components.state_apply_hash,
        Some(BlobHash::blake3(&on_disk_bytes)),
        "fixture builder must populate state_apply_hash matching the on-disk bytes",
    );
}

#[test]
fn build_signed_counter_bundle_three_components_populates_all_hashes() {
    // Same shape, but also asserts state_propose_hash + interaction_hash
    // match their respective on-disk bytes.
    // (Body elided — same pattern as the two-component test above.)
}
```

This test fails on the unmodified fixture (which produces `state_apply_hash: None`) and passes only after the fixture-builder extension above lands. It catches any future regression where the fixture builder forgets to recompute the hash after changing a component file.

**Verification commands:**

```bash
cargo test -p myrhiza-manifest --lib
cargo test -p myrhiza-manifest --lib canonical::tests::components_section_hash_fields_roundtrip
cargo build --workspace
cargo test --workspace --lib
# Tests in crates/kernel/src/install.rs::tests (write_fixture_bundle +
# write_two_component_fixture_bundle) must still pass — they re-sign
# the fixture after the schema update. If signatures fail to verify,
# the fixture-build code is wrong; root-cause + fix.
cargo clippy --workspace --all-targets -- -D warnings
```

**Risk:** A schema break to `ComponentsSection` invalidates any pre-existing manifest fixture in `target/` or in test fixtures on disk. Mitigation: every `write_*_bundle` helper rebuilds the manifest from struct literals + re-signs at test time — no on-disk persisted manifest needs migration. Confirm by grepping `tests/fixtures/` for any pre-baked `manifest.bincode`; if found, regenerate. The bundle dir under `tests/fixtures/built/` contains only `.wasm` files (per `crates/test-utils/src/bundle.rs`), so manifests are always built in-test — no on-disk migration risk.

**Commit message:**
```
feat(manifest): add Option<BlobHash> hash fields to ComponentsSection (B-10 §4.1)

Additive schema change: four parallel _hash fields interleaved with
the existing path fields. Disk bundles set them to None; iroh-blobs
publish populates them and the install-time fetch verifies. Manifest
signing target absorbs the new fields automatically (canonical-bincode
is positional). Existing fixture builders + tests updated to construct
the new shape; all signature paths re-verified.
```

---

## Task T4 — `BundleAddress` becomes enum: `Disk { ... } | IrohBlob { manifest_hash }`

**Spec ref:** §3.5 (the enum decision + backwards-compat); §5 risk row 5 (the 9 migration sites enumerated).

**Subject:** Convert `BundleAddress` from struct to two-variant enum. Existing constructor sites mechanically rewrite to `BundleAddress::Disk { ... }`. `InstallFlow::load` keeps its signature (`&BundleAddress`) and pattern-matches: `Disk` runs the existing impl; `IrohBlob` returns an error variant explicitly punted to T8 ("not yet wired — call BundleDistribution::fetch first").

**Files touched:**
- Modify: `crates/kernel/src/install.rs` (enum definition + `InstallFlow::load` dispatch + 2 internal fixture-builder sites at ~298 + ~380)
- Modify: `crates/myrhiza-cli/src/lib.rs:109` (1 site)
- Modify: `crates/kernel/tests/helpers/mod.rs:113` (1 site)
- Modify: `crates/kernel/tests/acceptance.rs:269,435` (2 sites)
- Modify: `crates/test-utils/src/bundle.rs:140,176,319` (3 sites)

Total: 9 mechanical migrations + 1 enum-shape change + 1 dispatch + 1 new error variant. Atomic commit — partial migration breaks the tree.

**Implementation notes:**

In `crates/kernel/src/install.rs`, replace the existing struct at line 39:

```rust
/// Locator for a bundle, either on disk or addressed via an
/// iroh-blobs content hash.
///
/// **Two variants** (per B-10 spec §3.5):
///
/// - [`BundleAddress::Disk`]: the existing on-disk layout. Used by
///   tests, by the dev workflow, and as the materialization target
///   for the iroh-blobs fetch path (a `Disk` variant is constructed
///   pointing at a tempdir after blob fetch + decode completes).
/// - [`BundleAddress::IrohBlob`]: production fetch path. Carries
///   the iroh-blobs hash of the canonical-bincode manifest. Cannot
///   be loaded directly via [`InstallFlow::load`] — the caller must
///   first call `BundleDistribution::fetch(manifest_hash, peers)`
///   to materialize a [`MaterializedBundle`] (in
///   `myrhiza-distribution`), then pass the resulting
///   `BundleAddress::Disk` into `InstallFlow::load`.
///
/// The dispatch is at the *embedder* level (runtime / CLI / future
/// SDK), not inside `InstallFlow` itself — `myrhiza-kernel` knows
/// about both variants but does NOT depend on `iroh-blobs`. The
/// `IrohBlob` variant carries enough information (`manifest_hash`)
/// for the embedder to fan out to the right fetcher.
#[derive(Debug, Clone)]
pub enum BundleAddress {
    /// On-disk bundle. Existing v1 behavior; unchanged.
    Disk {
        /// Root of the bundle directory.
        bundle_dir: PathBuf,
        /// Path of the manifest file relative to `bundle_dir`.
        manifest_path: PathBuf,
    },
    /// iroh-blobs-addressed bundle. Not loadable directly via
    /// [`InstallFlow::load`] — caller must first materialize via
    /// `myrhiza_distribution::BundleDistribution::fetch`. Per
    /// B-10 spec §3.5.
    IrohBlob {
        /// iroh-blobs hash of the canonical-bincode-encoded manifest.
        manifest_hash: myrhiza_types::BlobHash,
    },
}
```

In `InstallFlow::load`, pattern-match the variant and return a new error for the IrohBlob arm:

```rust
pub fn load(&self, addr: &BundleAddress) -> Result<LoadedBundle, InstallError> {
    let (bundle_dir, manifest_path) = match addr {
        BundleAddress::Disk { bundle_dir, manifest_path } => (bundle_dir, manifest_path),
        BundleAddress::IrohBlob { .. } => {
            return Err(InstallError::IrohBlobNotMaterialized);
        }
    };

    let manifest_bytes = std::fs::read(bundle_dir.join(manifest_path))?;
    // ... rest of existing body, using `bundle_dir` in place of
    // `addr.bundle_dir` everywhere ...
}
```

Add the new error variant to `InstallError`:

```rust
/// The caller passed a `BundleAddress::IrohBlob` but the install flow
/// does not materialize iroh-blobs bundles itself — the embedder must
/// first call `BundleDistribution::fetch` to produce a materialized
/// `BundleAddress::Disk` and pass *that* to `load`. Per B-10 spec §3.5.
#[error("BundleAddress::IrohBlob must be materialized via BundleDistribution::fetch before InstallFlow::load")]
IrohBlobNotMaterialized,
```

**Migration of 9 call sites** — mechanical:

```rust
// before:
let addr = BundleAddress {
    bundle_dir,
    manifest_path,
};

// after:
let addr = BundleAddress::Disk {
    bundle_dir,
    manifest_path,
};
```

For each site listed in the spec's §5 risk row 5:

1. `crates/kernel/src/install.rs:298` — inside `write_fixture_bundle`'s return.
2. `crates/kernel/src/install.rs:380` — inside `write_two_component_fixture_bundle`'s return.
3. `crates/kernel/tests/helpers/mod.rs:113` — inside `build_signed_pre_check_rejector_bundle`'s return.
4. `crates/kernel/tests/acceptance.rs:269` — inside fixture builder.
5. `crates/kernel/tests/acceptance.rs:435` — inside fixture builder.
6. `crates/test-utils/src/bundle.rs:140` — `build_signed_counter_bundle` return.
7. `crates/test-utils/src/bundle.rs:176` — `build_signed_echo_bundle` return.
8. `crates/test-utils/src/bundle.rs:319` — `build_signed_counter_bundle_three_components` return.
9. `crates/myrhiza-cli/src/lib.rs:109` — CLI's `addr` construction.

Use:

```bash
grep -rn "BundleAddress {" crates/
```

after migration to confirm zero remaining struct-shape constructors (the `enum BundleAddress {` definition itself is the only allowed match).

**Test coverage:** in `crates/kernel/src/install.rs::tests`, add a regression test that `InstallFlow::load(&BundleAddress::IrohBlob { manifest_hash: BlobHash::ZERO })` returns `InstallError::IrohBlobNotMaterialized`:

```rust
#[test]
fn install_rejects_iroh_blob_without_materialization() {
    use myrhiza_types::BlobHash;

    let addr = BundleAddress::IrohBlob {
        manifest_hash: BlobHash::ZERO,
    };
    let flow = InstallFlow::new();
    let err = flow.load(&addr).expect_err("IrohBlob direct-load must reject");
    assert!(
        matches!(err, InstallError::IrohBlobNotMaterialized),
        "expected IrohBlobNotMaterialized, got {err:?}"
    );
}
```

**Verification commands:**

```bash
cargo build --workspace
cargo test --workspace --lib
cargo test --workspace --tests
cargo clippy --workspace --all-targets -- -D warnings
grep -rn "BundleAddress {" crates/ tests/ 2>/dev/null
# Expected: ONLY the `pub enum BundleAddress {` line in install.rs.
# Any other match indicates an unmigrated site.
```

**Risk:** Missing a call site leaves the tree red. The 9-site enumeration in spec §5 is exhaustive per `grep`; verify post-migration with the grep above. If a new site has been added between spec write (2026-05-26) and execution time, the grep will catch it.

**Commit message:**
```
refactor(kernel): BundleAddress becomes enum Disk | IrohBlob (B-10 §3.5)

Disk variant: existing on-disk shape, unchanged behavior. IrohBlob
variant: production fetch path, carries manifest_hash. InstallFlow::load
returns InstallError::IrohBlobNotMaterialized for IrohBlob — embedder
must first call BundleDistribution::fetch (T8).

9 mechanical call sites migrated atomically:
- crates/kernel/src/install.rs (2 internal fixture builders + new
  dispatch + new error variant)
- crates/kernel/tests/helpers/mod.rs (pre-check-rejector fixture)
- crates/kernel/tests/acceptance.rs (2 fixture sites)
- crates/test-utils/src/bundle.rs (3 fixture builders)
- crates/myrhiza-cli/src/lib.rs (CLI install path)
```

---

## Task T5 — `RevocationLog` state machine: pure-function apply + state-tier tests

**Spec ref:** §4.4 (revocation schema + state machine — `RevocationLog::apply(prior, event) -> RevocationLog` purity); §6.1 (state-tier test enumeration).

**Subject:** Implement `RevocationEvent`, `RevocationLog`, `RevocationError` in `crates/distribution/src/revocation.rs`. The state machine is a pure function of `(prior state, signed event)`: no clock, no network, no I/O. State-tier tests cover signature verification, seq monotonicity, `MAX_REVOCATION_JUMP` boundary, double-revoke semantic idempotence.

> **MANDATORY (purity invariant):** the `revoked_at` field is
> informational only. `RevocationLog::apply` MUST NOT consult
> `SystemTime::now`, any clock, or any non-deterministic source.
> Cross-peer convergence depends on this — purity invariant per
> CLAUDE.md ("State-apply components must be pure functions of
> `(prior state, event)` plus the deterministic helper set"). The
> revocation log is a kernel-resident state machine analog and
> inherits the same contract.

**Files touched:**
- Create: `crates/distribution/src/revocation.rs`
- Modify: `crates/distribution/src/lib.rs` (uncomment `pub mod revocation;` + re-exports)

**Implementation notes:**

```rust
//! Revocation event schema + log state machine.
//!
//! Per B-10 spec §4.4. Pure-function `apply` — no clock, no network.
//! Mirrors the determinism discipline from `state-apply` components
//! per CLAUDE.md ("State-apply components must be pure functions of
//! `(prior state, event)` plus the deterministic helper set"). The
//! revocation log is a kernel-resident analog with the same purity
//! contract.

use std::collections::BTreeSet;

use bincode::Options;
use ed25519_dalek::{Signature as DalekSignature, VerifyingKey};
use myrhiza_types::{AuthorPubkey, BlobHash, canonical_bincode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Domain-separator string for revocation signatures. Mirrors the
/// manifest-signature framing in `crates/manifest/src/canonical.rs`
/// (where `DOMAIN_SEP = "myrhiza/manifest/v1"`); the domain prefix
/// defends against key-reuse across envelope types if the same author
/// key ever signs heterogeneous payloads. Per B-10 spec §4.4.
pub const DOMAIN_SEP_REVOCATION: &[u8] = b"myrhiza/revocation/v1";

/// Maximum revocation-seq jump per 24h window. Per
/// `docs/specs/2026-05-09-myrhiza-master-design/distribution.md` §10.7.
/// Acts as a flood-protection bound; legitimate authors should never
/// approach this in normal use.
pub const MAX_REVOCATION_JUMP: u64 = 1024;

/// Maximum bytes of `reason` text. Per B-10 spec §4.4. Truncated
/// (not rejected) on encode at the publish side; the receive side
/// just enforces the bound on decode.
pub const MAX_REASON_LEN: usize = 256;

/// Signed revocation envelope.
///
/// Per B-10 spec §4.4. Gossipped on the per-author revocation topic
/// derived by [`derive_revocation_topic`](crate::derive_revocation_topic).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevocationEvent {
    /// The bundle hash being revoked.
    pub revoked_bundle_hash: BlobHash,
    /// Human-readable reason (≤ `MAX_REASON_LEN` bytes).
    pub reason: String,
    /// Author-asserted timestamp (informational; NOT trusted for
    /// ordering — `revocation_seq` is the authority).
    pub revoked_at: u64,
    /// Monotonic-per-author. Kernel rejects out-of-order or duplicate.
    pub revocation_seq: u64,
    /// Ed25519 signature by the author over `DOMAIN_SEP_REVOCATION ||
    /// canonical_bincode(SignedFields)` where `SignedFields` is the
    /// envelope minus the `signature` field.
    #[serde(with = "serde_bytes_64")]
    pub signature: [u8; 64],
}

mod serde_bytes_64 {
    use serde::{Deserialize, Deserializer, Serializer, ser::SerializeTuple};

    pub fn serialize<S: Serializer>(bytes: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        let mut t = s.serialize_tuple(64)?;
        for b in bytes {
            t.serialize_element(b)?;
        }
        t.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        <[u8; 64] as Deserialize>::deserialize(d)
    }
}

#[derive(Serialize)]
struct RevocationSignedFields<'a> {
    revoked_bundle_hash: &'a BlobHash,
    reason: &'a str,
    revoked_at: u64,
    revocation_seq: u64,
}

impl RevocationEvent {
    /// Bytes the signature commits to: `DOMAIN_SEP_REVOCATION ||
    /// canonical_bincode(signed_fields)`. Public so publish-side
    /// authors can construct the same bytes for signing.
    #[must_use]
    pub fn signing_target(&self) -> Vec<u8> {
        let signed = RevocationSignedFields {
            revoked_bundle_hash: &self.revoked_bundle_hash,
            reason: &self.reason,
            revoked_at: self.revoked_at,
            revocation_seq: self.revocation_seq,
        };
        let mut out = Vec::with_capacity(DOMAIN_SEP_REVOCATION.len() + 128);
        out.extend_from_slice(DOMAIN_SEP_REVOCATION);
        let body = canonical_bincode().serialize(&signed).expect("encode signed fields");
        out.extend_from_slice(&body);
        out
    }
}

/// Errors `RevocationLog::apply` can return.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum RevocationError {
    /// Ed25519 signature verification failed.
    #[error("revocation signature verification failed")]
    SignatureInvalid,
    /// Author pubkey was not a valid Ed25519 curve point.
    #[error("author pubkey is not a valid Ed25519 verifying key")]
    AuthorPubkeyMalformed,
    /// `revocation_seq` is not strictly greater than `last_observed_seq`.
    #[error("revocation_seq {got} not greater than last_observed_seq {last_observed}")]
    SeqNotMonotonic {
        /// The seq in the event.
        got: u64,
        /// The last accepted seq for this author.
        last_observed: u64,
    },
    /// `revocation_seq` exceeds `last_observed_seq + MAX_REVOCATION_JUMP`.
    #[error("revocation_seq jump {jump} exceeds MAX_REVOCATION_JUMP={MAX_REVOCATION_JUMP}")]
    SeqJumpTooLarge {
        /// `event.revocation_seq - last_observed_seq`.
        jump: u64,
    },
    /// `reason` exceeds `MAX_REASON_LEN` bytes.
    #[error("reason length {got} exceeds MAX_REASON_LEN={MAX_REASON_LEN}")]
    ReasonTooLong {
        /// Observed length.
        got: usize,
    },
}

/// Per-author revocation log state.
///
/// Per B-10 spec §4.4. Pure-function state machine — `apply`
/// consumes `(self, event)` and returns the next state with no
/// side effects beyond the structurally returned diff.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RevocationLog {
    /// Highest `revocation_seq` accepted so far. Starts at 0;
    /// first accepted event must have `revocation_seq >= 1`.
    pub last_observed_seq: u64,
    /// Set of bundle hashes revoked by this author.
    pub revoked_bundles: BTreeSet<BlobHash>,
}

impl RevocationLog {
    /// Construct an empty log.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a signed `RevocationEvent` to the log.
    ///
    /// Pure function of `(self, event, author)`. Returns either the
    /// updated log (which structurally includes the new revoked
    /// hash) or a `RevocationError` indicating why the event was
    /// rejected. The author pubkey is taken as a parameter rather
    /// than carried in the event because the gossip dispatch layer
    /// already knows which author's topic the event arrived on
    /// (per spec §3.3 — topic is `BLAKE3("myrhiza/revocations/v1"
    /// || author_pubkey)`); requiring the caller to pass it makes
    /// signature-cross-checking impossible to forget.
    ///
    /// # Errors
    ///
    /// Returns `RevocationError::SignatureInvalid`,
    /// `RevocationError::AuthorPubkeyMalformed`,
    /// `RevocationError::SeqNotMonotonic`,
    /// `RevocationError::SeqJumpTooLarge`, or
    /// `RevocationError::ReasonTooLong`. None mutate state — on
    /// `Err`, the caller's `RevocationLog` is unchanged.
    pub fn apply(
        mut self,
        event: &RevocationEvent,
        author: &AuthorPubkey,
    ) -> Result<Self, RevocationError> {
        if event.reason.len() > MAX_REASON_LEN {
            return Err(RevocationError::ReasonTooLong {
                got: event.reason.len(),
            });
        }
        if event.revocation_seq <= self.last_observed_seq {
            return Err(RevocationError::SeqNotMonotonic {
                got: event.revocation_seq,
                last_observed: self.last_observed_seq,
            });
        }
        let jump = event.revocation_seq - self.last_observed_seq;
        if jump > MAX_REVOCATION_JUMP {
            return Err(RevocationError::SeqJumpTooLarge { jump });
        }

        let vk = VerifyingKey::from_bytes(author.as_bytes())
            .map_err(|_| RevocationError::AuthorPubkeyMalformed)?;
        let sig = DalekSignature::from_bytes(&event.signature);
        let target = event.signing_target();
        vk.verify_strict(&target, &sig)
            .map_err(|_| RevocationError::SignatureInvalid)?;

        self.revoked_bundles.insert(event.revoked_bundle_hash);
        self.last_observed_seq = event.revocation_seq;
        Ok(self)
    }

    /// True if `bundle` has been revoked.
    #[must_use]
    pub fn is_revoked(&self, bundle: &BlobHash) -> bool {
        self.revoked_bundles.contains(bundle)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn sign_event(
        sk: &SigningKey,
        revoked: BlobHash,
        reason: &str,
        seq: u64,
    ) -> RevocationEvent {
        let mut ev = RevocationEvent {
            revoked_bundle_hash: revoked,
            reason: reason.into(),
            revoked_at: 0,
            revocation_seq: seq,
            signature: [0u8; 64],
        };
        let target = ev.signing_target();
        let sig = sk.sign(&target);
        ev.signature = sig.to_bytes();
        ev
    }

    fn fixture() -> (SigningKey, AuthorPubkey) {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let pk = AuthorPubkey::from_bytes(sk.verifying_key().to_bytes());
        (sk, pk)
    }

    #[test]
    fn applies_first_revocation() {
        let (sk, pk) = fixture();
        let ev = sign_event(&sk, BlobHash::from_bytes([0xAA; 32]), "compromised", 1);
        let log = RevocationLog::new().apply(&ev, &pk).expect("apply ok");
        assert_eq!(log.last_observed_seq, 1);
        assert!(log.is_revoked(&BlobHash::from_bytes([0xAA; 32])));
    }

    #[test]
    fn rejects_signature_mismatch() {
        let (_, pk) = fixture();
        let (wrong_sk, _) = (SigningKey::from_bytes(&[42u8; 32]), ());
        let ev = sign_event(&wrong_sk, BlobHash::from_bytes([0xAA; 32]), "x", 1);
        let err = RevocationLog::new().apply(&ev, &pk).expect_err("must reject");
        assert!(matches!(err, RevocationError::SignatureInvalid));
    }

    #[test]
    fn rejects_out_of_order_seq() {
        let (sk, pk) = fixture();
        let ev1 = sign_event(&sk, BlobHash::from_bytes([0xAA; 32]), "x", 3);
        let ev2 = sign_event(&sk, BlobHash::from_bytes([0xBB; 32]), "y", 2);
        let log = RevocationLog::new().apply(&ev1, &pk).expect("apply 1");
        let err = log.apply(&ev2, &pk).expect_err("seq 2 < 3 must reject");
        assert!(matches!(err, RevocationError::SeqNotMonotonic { .. }));
    }

    #[test]
    fn rejects_duplicate_seq() {
        let (sk, pk) = fixture();
        let ev1 = sign_event(&sk, BlobHash::from_bytes([0xAA; 32]), "x", 2);
        let ev2 = sign_event(&sk, BlobHash::from_bytes([0xBB; 32]), "y", 2);
        let log = RevocationLog::new().apply(&ev1, &pk).expect("apply 1");
        let err = log.apply(&ev2, &pk).expect_err("duplicate seq must reject");
        assert!(matches!(err, RevocationError::SeqNotMonotonic { .. }));
    }

    #[test]
    fn rejects_jump_exceeds_max() {
        let (sk, pk) = fixture();
        let ev1 = sign_event(&sk, BlobHash::from_bytes([0xAA; 32]), "x", 1);
        let ev2 = sign_event(
            &sk,
            BlobHash::from_bytes([0xBB; 32]),
            "y",
            1 + MAX_REVOCATION_JUMP + 1,
        );
        let log = RevocationLog::new().apply(&ev1, &pk).expect("apply 1");
        let err = log.apply(&ev2, &pk).expect_err("jump+1 must reject");
        assert!(matches!(err, RevocationError::SeqJumpTooLarge { .. }));
    }

    #[test]
    fn accepts_jump_at_max() {
        let (sk, pk) = fixture();
        let ev1 = sign_event(&sk, BlobHash::from_bytes([0xAA; 32]), "x", 1);
        let ev2 = sign_event(
            &sk,
            BlobHash::from_bytes([0xBB; 32]),
            "y",
            1 + MAX_REVOCATION_JUMP,
        );
        let log = RevocationLog::new().apply(&ev1, &pk).expect("apply 1");
        let log = log.apply(&ev2, &pk).expect("max-jump must accept");
        assert_eq!(log.last_observed_seq, 1 + MAX_REVOCATION_JUMP);
    }

    #[test]
    fn idempotent_double_revoke_same_bundle() {
        let (sk, pk) = fixture();
        let hash = BlobHash::from_bytes([0xAA; 32]);
        let ev1 = sign_event(&sk, hash, "first", 1);
        let ev2 = sign_event(&sk, hash, "second", 2);
        let log = RevocationLog::new().apply(&ev1, &pk).expect("apply 1");
        let log = log.apply(&ev2, &pk).expect("apply 2");
        // Bundle still revoked (semantic idempotence — the set
        // doesn't track multi-revoke; a re-revoke is a no-op for
        // membership but still bumps seq).
        assert!(log.is_revoked(&hash));
        assert_eq!(log.last_observed_seq, 2);
    }

    #[test]
    fn rejects_reason_too_long() {
        let (sk, pk) = fixture();
        let too_long = "x".repeat(MAX_REASON_LEN + 1);
        let ev = sign_event(&sk, BlobHash::from_bytes([0xAA; 32]), &too_long, 1);
        let err = RevocationLog::new().apply(&ev, &pk).expect_err("must reject");
        assert!(matches!(err, RevocationError::ReasonTooLong { .. }));
    }
}
```

In `crates/distribution/src/lib.rs`, uncomment + extend:

```rust
pub mod revocation;
pub use revocation::{
    DOMAIN_SEP_REVOCATION, MAX_REASON_LEN, MAX_REVOCATION_JUMP, RevocationError, RevocationEvent,
    RevocationLog,
};
```

Confirm `AuthorPubkey::as_bytes() -> &[u8; 32]` exists (per `crates/types/src/author.rs`); if it returns `&[u8]`, adapt.

**Verification commands:**

```bash
cargo test -p myrhiza-distribution --lib revocation::tests
cargo clippy -p myrhiza-distribution --all-targets -- -D warnings
cargo build --workspace
```

**Risk:** `bincode::Options::serialize` infallibility for the signed-fields struct is implicit. Use `.expect("encode signed fields")` — matches the precedent in `crates/manifest/src/canonical.rs::signed_body_bytes`. Workspace-wide `expect_used = "warn"` is the documented escape per CLAUDE.md / `Cargo.toml` workspace lints; mark with `#[allow(clippy::expect_used)]` on the `signing_target` method if clippy fires.

**Commit message:**
```
feat(distribution): RevocationLog + RevocationEvent state machine (B-10 §4.4)

Pure-function apply((prior log, signed event)) -> log. No clock, no
network. State-tier tests cover signature verification, seq
monotonicity, MAX_REVOCATION_JUMP boundary (accept at max, reject
at max+1), duplicate-seq, signature mismatch, reason-length cap,
double-revoke semantic idempotence.

Mirrors state-apply purity per CLAUDE.md "State-apply components
must be pure functions of (prior state, event)".
```

---

## Task T6 — `PublicationLog` state machine: parallel to revocation, same purity

**Spec ref:** §3.4 (publication topic + event shape); §6.2 (state-tier test outline).

**Subject:** Implement `PublicationEvent`, `PublicationLog`, `PublicationError` in `crates/distribution/src/publication.rs`. Structurally parallel to `RevocationLog` (T5): pure function, monotonic-seq, signature-verified. The state tracks `last_observed_seq` per author and emits a notification on each accepted publication.

**Files touched:**
- Create: `crates/distribution/src/publication.rs`
- Modify: `crates/distribution/src/lib.rs` (uncomment + re-export)

**Implementation notes:**

Mirror the T5 shape:

```rust
//! Publication event schema + log state machine.
//!
//! Per B-10 spec §3.4. Pure-function `apply` — no clock, no network.
//! Structurally parallel to the revocation log (T5) so the gossip
//! dispatch layer treats both as "monotonic-seq, signature-verified,
//! per-author topic" envelopes.

use bincode::Options;
use ed25519_dalek::{Signature as DalekSignature, VerifyingKey};
use myrhiza_types::{AuthorPubkey, BlobHash, canonical_bincode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Domain-separator string for publication signatures. Mirrors the
/// revocation domain separator (`DOMAIN_SEP_REVOCATION =
/// "myrhiza/revocation/v1"`) so the framing is consistent across
/// envelope types. Per B-10 spec §3.4.
pub const DOMAIN_SEP_PUBLICATION: &[u8] = b"myrhiza/publication/v1";

/// Maximum version-string length. Per B-10 spec §3.4.
pub const MAX_VERSION_LEN: usize = 64;

/// Maximum publication-seq jump per author (same shape as
/// `MAX_REVOCATION_JUMP`).
pub const MAX_PUBLICATION_JUMP: u64 = 1024;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicationEvent {
    pub manifest_hash: BlobHash,
    pub version: String,
    pub publication_seq: u64,
    #[serde(with = "crate::revocation::serde_bytes_64")]
    pub signature: [u8; 64],
}
```

Re-exporting `serde_bytes_64` from `revocation` requires it to be `pub(crate)`; alternatively, duplicate the helper. Given the small surface and that B-10 only has two envelopes, **duplicate the helper inline** in `publication.rs` — cleaner than cross-module crate-internal exposure for a 12-line module. Then the publication module is fully independent of revocation at the source level.

The signing-fields struct, the signing target derivation, the `apply` method, the error enum (with `SeqNotMonotonic`, `SeqJumpTooLarge`, `SignatureInvalid`, `AuthorPubkeyMalformed`, `VersionTooLong`), and the `PublicationLog` struct (with `last_observed_seq` plus optionally a `latest_announcement: Option<(BlobHash, String)>` for the kernel UI surface) all mirror revocation exactly.

State-tier tests in `publication.rs::tests`:

- `applies_first_publication` — empty log + signed event seq=1 → accepted, `latest_announcement = Some(...)`.
- `rejects_signature_mismatch` — wrong-key signature → `PublicationError::SignatureInvalid`.
- `rejects_out_of_order_seq` — seq 3 then seq 2 → second rejected.
- `rejects_duplicate_seq` — seq 2 then seq 2 → second rejected.
- `rejects_jump_exceeds_max` — boundary at `MAX_PUBLICATION_JUMP + 1`.
- `accepts_jump_at_max` — boundary at `MAX_PUBLICATION_JUMP`.
- `rejects_version_too_long` — `MAX_VERSION_LEN + 1` chars → rejected.
- `latest_announcement_updates_on_accept` — applying a fresh event updates `latest_announcement`.

In `crates/distribution/src/lib.rs`:

```rust
pub mod publication;
pub use publication::{
    DOMAIN_SEP_PUBLICATION, MAX_PUBLICATION_JUMP, MAX_VERSION_LEN, PublicationError,
    PublicationEvent, PublicationLog,
};
```

**Verification commands:**

```bash
cargo test -p myrhiza-distribution --lib publication::tests
cargo clippy -p myrhiza-distribution --all-targets -- -D warnings
cargo build --workspace
```

**Risk:** Avoid pattern divergence from `RevocationLog` — if seq-monotonicity logic drifts between the two, gossip dispatch's "treat both uniformly" assumption (used in T9) breaks. Keep the apply-method skeleton byte-for-byte parallel where the spec allows. The intentional divergence is the `latest_announcement` field (publication is informational + cumulative; revocation is presence-or-absence in a set).

**Commit message:**
```
feat(distribution): PublicationLog + PublicationEvent state machine (B-10 §3.4)

Structurally parallel to RevocationLog (T5). Pure function apply.
State-tier tests cover signature verification, seq monotonicity,
MAX_PUBLICATION_JUMP boundary, duplicate-seq, version-length cap.

Mirrors RevocationLog by design — gossip dispatch treats both as
monotonic-seq + signature-verified per-author envelopes.
```

---

## Task T7 — `BundleDistribution::publish` (in-memory iroh-blobs store)

**Spec ref:** §3.2 (publish-side hash semantics + auth chain); §4.3 (`BundleDistribution` API sketch); §5 risk row 7 (`MemStore` vs `FsStore` — pick `MemStore` per spec §12 question 1, FsStore deferred to B-9).

**Subject:** Implement `BundleDistribution::publish` in `crates/distribution/src/blobs.rs`. Holds an `iroh_blobs::store::MemStore` + `iroh_blobs::BlobsProtocol` + an `iroh::Endpoint`. Imports manifest + each declared component blob; returns the manifest hash as the `BundleAddress::IrohBlob` identifier.

**Files touched:**
- Create: `crates/distribution/src/blobs.rs`
- Modify: `crates/distribution/src/lib.rs` (feature-gate + re-export)

**Implementation notes:**

```rust
//! iroh-blobs-backed publish + fetch.
//!
//! Per B-10 spec §3.2 + §4.3. Feature-gated on `network-iroh`.
//!
//! ## Store choice: MemStore
//!
//! B-10 wires `iroh_blobs::store::MemStore` (per spec §12 open
//! question 1; FsStore wiring is a B-9-adjacent follow-up).
//! `MemStore` is faster for tests and avoids touching the filesystem
//! during publish. Production deployments will swap to `FsStore`
//! through embedder configuration without changing this crate's
//! public API.

#![cfg(feature = "network-iroh")]

use std::sync::Arc;

use bincode::Options;
use myrhiza_manifest::schema::Manifest;
use myrhiza_types::{BlobHash, canonical_bincode};
use thiserror::Error;

use crate::conversions::{blob_hash_from_iroh, iroh_hash_from_blob_hash};

/// Errors `BundleDistribution::publish` can return.
#[derive(Debug, Error)]
pub enum PublishError {
    /// Encoding the manifest to canonical bincode failed.
    #[error("encode manifest: {0}")]
    EncodeManifest(String),
    /// iroh-blobs `add_bytes` failed.
    #[error("iroh-blobs add: {0}")]
    BlobsAdd(String),
    /// The manifest carries no `state_apply` component slot. At
    /// least state-apply must be present per
    /// `docs/specs/2026-05-09-myrhiza-master-design/distribution.md`
    /// §10.2.
    #[error("manifest declares no state-apply component — invalid bundle")]
    MissingStateApply,
    /// A `*_hash` field in the manifest does not match the bytes
    /// supplied to `publish`. Publish-side defense-in-depth — the
    /// author wrote the hash into the manifest and signed it; if
    /// the bytes don't match, the fetch path will reject the
    /// bundle and the author's signature is structurally invalid.
    #[error(
        "component hash mismatch: manifest declares {expected}, actual bytes hash to {actual}"
    )]
    ComponentHashMismatch {
        expected: BlobHash,
        actual: BlobHash,
    },
}

/// Holds a local iroh-blobs store + the iroh::Endpoint already
/// constructed by the kernel embedder. Constructed once at kernel
/// boot, shared across all fetch + publish operations.
///
/// Per B-10 spec §4.3.
pub struct BundleDistribution {
    store: Arc<iroh_blobs::store::MemStore>,
    blobs_protocol: iroh_blobs::BlobsProtocol,
    // endpoint is held for fetch (T8) — kept here so publish + fetch
    // share the same iroh transport stack.
    #[allow(dead_code)]
    endpoint: iroh::Endpoint,
}

impl BundleDistribution {
    /// Construct from a pre-built `iroh::Endpoint`. Spins up a new
    /// `MemStore` + `BlobsProtocol`. The caller is responsible for
    /// registering `iroh_blobs::BlobsProtocol::ALPN` against the
    /// router that owns this endpoint (per spec §4.3 wiring
    /// preconditions).
    #[must_use]
    pub fn new(endpoint: iroh::Endpoint) -> Self {
        let store = Arc::new(iroh_blobs::store::MemStore::new());
        // Pass `None` for the EventSender per spec §12 question 2 —
        // tests don't need progress notifications.
        let blobs_protocol = iroh_blobs::BlobsProtocol::new(&store, None);
        Self {
            store,
            blobs_protocol,
            endpoint,
        }
    }

    /// Borrow the `BlobsProtocol` for router registration.
    ///
    /// Caller registers `iroh_blobs::BlobsProtocol::ALPN` against
    /// this handler in their `iroh::protocol::Router::builder`
    /// before spawning the router.
    #[must_use]
    pub fn protocol_handler(&self) -> &iroh_blobs::BlobsProtocol {
        &self.blobs_protocol
    }

    /// Publish: import the manifest + all declared components into
    /// the local iroh-blobs store. Returns the manifest hash (the
    /// `BundleAddress::IrohBlob::manifest_hash` identifier).
    ///
    /// Defense-in-depth: cross-checks that each provided component's
    /// actual BLAKE3 hash matches the `*_hash` field in the manifest.
    /// Mismatch is a publish-side author error (the signature would
    /// not validate downstream).
    ///
    /// Per B-10 spec §3.2 + §4.3.
    ///
    /// # Errors
    ///
    /// Returns `PublishError::EncodeManifest` if canonical-bincode
    /// fails, `PublishError::BlobsAdd` if iroh-blobs rejects an
    /// import, `PublishError::MissingStateApply` if the manifest has
    /// no state-apply component, or `PublishError::ComponentHashMismatch`
    /// if a provided bytes/hash pair disagrees.
    pub async fn publish(
        &self,
        manifest: &Manifest,
        manifest_bytes: &[u8],
        state_apply_bytes: &[u8],
        state_propose_bytes: Option<&[u8]>,
        interaction_bytes: Option<&[u8]>,
        behavior_bytes: Option<&[u8]>,
    ) -> Result<BlobHash, PublishError> {
        // state-apply is mandatory.
        if manifest.components.state_apply.is_none()
            && manifest.components.state_apply_hash.is_none()
        {
            return Err(PublishError::MissingStateApply);
        }

        Self::check_hash(state_apply_bytes, manifest.components.state_apply_hash)?;
        if let Some(b) = state_propose_bytes {
            Self::check_hash(b, manifest.components.state_propose_hash)?;
        }
        if let Some(b) = interaction_bytes {
            Self::check_hash(b, manifest.components.interaction_hash)?;
        }
        if let Some(b) = behavior_bytes {
            Self::check_hash(b, manifest.components.behavior_hash)?;
        }

        // Import each blob into the local store.
        self.add_bytes(state_apply_bytes).await?;
        if let Some(b) = state_propose_bytes {
            self.add_bytes(b).await?;
        }
        if let Some(b) = interaction_bytes {
            self.add_bytes(b).await?;
        }
        if let Some(b) = behavior_bytes {
            self.add_bytes(b).await?;
        }
        let manifest_iroh_hash = self.add_bytes(manifest_bytes).await?;
        Ok(blob_hash_from_iroh(manifest_iroh_hash))
    }

    async fn add_bytes(&self, bytes: &[u8]) -> Result<iroh_blobs::Hash, PublishError> {
        // **Impl-time API verification**: iroh-blobs 0.101.0
        // `MemStore::add_bytes` signature varies between minor
        // versions (per prior-art/iroh/blobs.md). Verify the exact
        // call at impl time — likely `self.store.add_bytes(Bytes::copy_from_slice(bytes)).await`.
        // Adapt to current API; document deviation in commit body.
        let tag = self
            .store
            .add_bytes(bytes.to_vec())
            .await
            .map_err(|e| PublishError::BlobsAdd(format!("add_bytes: {e}")))?;
        Ok(tag.hash)
    }

    fn check_hash(
        bytes: &[u8],
        declared: Option<BlobHash>,
    ) -> Result<(), PublishError> {
        let actual = BlobHash::blake3(bytes);
        match declared {
            None => Ok(()), // disk-only bundle — no iroh hash declared
            Some(d) if d == actual => Ok(()),
            Some(d) => Err(PublishError::ComponentHashMismatch {
                expected: d,
                actual,
            }),
        }
    }
}
```

In `crates/distribution/src/lib.rs`:

```rust
#[cfg(feature = "network-iroh")]
pub mod blobs;
#[cfg(feature = "network-iroh")]
pub use blobs::{BundleDistribution, PublishError};
```

**API verification at impl time** (per prior-art/iroh/lessons.md §Avoid row 1 + spec §9 "remaining gaps"):

The exact `iroh_blobs` API names will differ from this sketch. Spec §12 question 1 + this task accept any of:

- `MemStore::new()` → may be `MemStore::default()` or `MemStore::builder().spawn()`.
- `MemStore::add_bytes(bytes)` → may take `Bytes::from(...)` or `impl Into<Bytes>` or return a different tag type.
- `BlobsProtocol::new(&store, None)` → may not exist; alternatives: `BlobsProtocol::new(store.clone())` (with progress builder pattern).
- `iroh_blobs::BlobsProtocol::ALPN` constant exists per spec §6.3.

When adapting, follow the existing precedent from `crates/network/src/iroh_transport.rs` (which adapted plan-B-4.0's hypothetical names against rc.0 reality) and document in the commit body.

**Unit tests at T7:**

Keep any pure-data unit tests (e.g. `check_hash` matching / mismatching with synthetic byte arrays — no endpoint required) inline in the `blobs.rs::tests` module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_hash_accepts_none_declared() {
        // disk-only bundles: no declared hash → trivially OK
        assert!(BundleDistribution::check_hash(b"any", None).is_ok());
    }

    #[test]
    fn check_hash_accepts_matching_declared() {
        let bytes = b"\x00asm\x01\x00\x00\x00";
        let h = BlobHash::blake3(bytes);
        assert!(BundleDistribution::check_hash(bytes, Some(h)).is_ok());
    }

    #[test]
    fn check_hash_rejects_mismatched_declared() {
        let bytes = b"\x00asm\x01\x00\x00\x00";
        let wrong = BlobHash::from_bytes([0xAA; 32]);
        let err = BundleDistribution::check_hash(bytes, Some(wrong)).expect_err("must reject");
        assert!(matches!(err, PublishError::ComponentHashMismatch { .. }));
    }
}
```

These exercise `BundleDistribution::check_hash` purely — no `iroh::Endpoint`, no `MemoryLookup`, no test-utils dep on `crates/distribution/`.

**iroh-blobs integration is verified at T11 (kernel-tier e2e), not T7.** The publish+fetch round-trip through real iroh-blobs over loopback QUIC requires the `IrohHarness` from `crates/test-utils/` (extended at T10). `crates/distribution/` does NOT depend on `crates/test-utils/` (dep direction per spec §4.6 is the other way — test-utils gains `myrhiza-distribution` at T10). Constructing an iroh `Endpoint` standalone inside `crates/distribution/`'s unit tests would either (a) duplicate harness logic in the production crate, or (b) introduce a cyclic dev-dep through test-utils. Both are wrong; defer to T11.

**Verification commands:**

```bash
cargo build -p myrhiza-distribution --features network-iroh
cargo test -p myrhiza-distribution --features network-iroh --lib blobs::tests
cargo clippy -p myrhiza-distribution --features network-iroh --all-targets -- -D warnings
cargo build --workspace --all-features
```

**Risk:** iroh-blobs API surface uncertainty is the highest-risk item in B-10. If the API names rotated such that the `add_bytes` signature differs significantly, surface that in the commit body and adapt — do NOT use `unimplemented!()`. If a single hour of investigation cannot resolve the call, escalate to the spec author (the spec §9 gap explicitly flags this).

**Commit message:**
```
feat(distribution): BundleDistribution::publish via iroh-blobs MemStore (B-10 §3.2 + §4.3)

In-memory iroh-blobs store; imports manifest + each declared component
blob. Returns the iroh-blobs hash of the canonical-bincode manifest
as the BundleAddress::IrohBlob identifier. Publish-side defense-in-
depth: cross-checks each component bytes' BLAKE3 against the manifest's
*_hash field (catches author errors before signing).

API adaptations from spec §4.3 sketch documented inline (per
prior-art/iroh/lessons.md §Avoid row 1 — pre-1.0 API churn).
```

---

## Task T8 — `BundleDistribution::fetch` + `MaterializedBundle` (kernel-side dispatch wiring)

**Spec ref:** §3.2 (fetch-side auth chain — 6 steps); §4.3 (`BundleDistribution::fetch` API + `MaterializedBundle` shape); §3.5 (the `IrohBlob → Disk` materialization story).

**Subject:** Implement `BundleDistribution::fetch(manifest_hash, peers) → MaterializedBundle`. Fetches the manifest blob, decodes it, fetches each declared component blob, materializes everything into a tempdir mirroring the disk-bundle layout, returns a `MaterializedBundle { _tempdir, address: BundleAddress::Disk }`. Then wire `crates/kernel` to call `BundleDistribution::fetch` when `Runtime` or the CLI installs an `IrohBlob` address.

**Files touched:**
- Modify: `crates/distribution/src/blobs.rs` (add `fetch` method + `MaterializedBundle` struct + `FetchError`)
- Modify: `crates/distribution/src/lib.rs` (re-export `FetchError`, `MaterializedBundle`)
- Modify: `crates/kernel/Cargo.toml` (add `myrhiza-distribution = { path = "../distribution" }`; propagate `network-iroh` feature)
- Modify: `crates/kernel/src/lib.rs` (re-export `MaterializedBundle` if needed for CLI)
- (Possibly) `crates/myrhiza-cli/src/lib.rs` — surface the iroh-blob path through a CLI subcommand smoke test (per spec §11 day-7 polish; can defer to a separate task if scope grows)

**Implementation notes:**

```rust
/// Output of `BundleDistribution::fetch`: a verified bundle
/// materialized into a tempdir, addressable as
/// `BundleAddress::Disk` for `InstallFlow::load`.
///
/// The tempdir is owned via RAII — the bundle bytes live only as
/// long as `MaterializedBundle` lives. The kernel embedder must
/// keep the `MaterializedBundle` alive across the call to
/// `InstallFlow::load`.
pub struct MaterializedBundle {
    /// RAII tempdir holding the materialized bundle layout
    /// (`manifest.bincode` + `components/state-apply.wasm` etc.).
    pub _tempdir: tempfile::TempDir,
    /// `BundleAddress::Disk` pointing into `_tempdir`.
    pub address: myrhiza_kernel::BundleAddress,
}

#[derive(Debug, Error)]
pub enum FetchError {
    /// iroh-blobs fetch failed (no peer hosting the hash;
    /// connectivity; verified-streaming integrity error).
    #[error("iroh-blobs fetch: {0}")]
    BlobsFetch(String),
    /// Decoding the canonical-bincode manifest failed.
    #[error("decode manifest: {0}")]
    DecodeManifest(String),
    /// The manifest declares a component path/hash combination that
    /// the fetch cannot resolve (no `*_hash` field set).
    #[error("manifest component {profile} missing iroh-blobs hash — required for IrohBlob fetch")]
    ComponentMissingHash {
        profile: &'static str,
    },
    /// I/O writing the materialized bundle to the tempdir failed.
    #[error("write tempdir: {0}")]
    WriteTempdir(#[from] std::io::Error),
}

impl BundleDistribution {
    /// Fetch: pull the manifest + all declared components from peers,
    /// verify each blob's BLAKE3 hash, materialize into a tempdir,
    /// return a `MaterializedBundle` ready for `InstallFlow::load`.
    ///
    /// Per B-10 spec §3.2 + §4.3 + §3.5.
    ///
    /// `peers` provides bootstrap hints — at least one peer in this
    /// list MUST host the bundle for the fetch to succeed. Empty
    /// vec is legal only if the local store already has the bundle
    /// (e.g. publish-then-fetch on the same node).
    ///
    /// # Errors
    ///
    /// Returns `FetchError::BlobsFetch` for transport-layer failures,
    /// `FetchError::DecodeManifest` if the manifest blob bytes do
    /// not decode under canonical bincode,
    /// `FetchError::ComponentMissingHash` if the manifest references
    /// a profile (e.g. state-apply) without populating its `*_hash`
    /// field, or `FetchError::WriteTempdir` for I/O errors writing
    /// the materialized layout.
    pub async fn fetch(
        &self,
        manifest_hash: BlobHash,
        peers: &[myrhiza_types::PeerPubkey],
    ) -> Result<MaterializedBundle, FetchError> {
        // 1. Pull manifest bytes. iroh-blobs BLAKE3+Bao verifies the
        //    fetched bytes hash to manifest_hash.
        let manifest_bytes = self.fetch_blob(manifest_hash, peers).await?;

        // 2. Decode the canonical-bincode manifest.
        let manifest: Manifest = canonical_bincode()
            .deserialize(&manifest_bytes)
            .map_err(|e| FetchError::DecodeManifest(e.to_string()))?;

        // 3. For each declared component slot, fetch the referenced blob.
        let state_apply_hash = manifest
            .components
            .state_apply_hash
            .ok_or(FetchError::ComponentMissingHash {
                profile: "state-apply",
            })?;
        let state_apply_bytes = self.fetch_blob(state_apply_hash, peers).await?;

        let state_propose_bytes = match (
            &manifest.components.state_propose,
            manifest.components.state_propose_hash,
        ) {
            (Some(_), Some(h)) => Some(self.fetch_blob(h, peers).await?),
            (Some(_), None) => {
                return Err(FetchError::ComponentMissingHash {
                    profile: "state-propose",
                });
            }
            _ => None,
        };
        let interaction_bytes = match (
            &manifest.components.interaction,
            manifest.components.interaction_hash,
        ) {
            (Some(_), Some(h)) => Some(self.fetch_blob(h, peers).await?),
            (Some(_), None) => {
                return Err(FetchError::ComponentMissingHash {
                    profile: "interaction",
                });
            }
            _ => None,
        };
        let behavior_bytes = match (
            &manifest.components.behavior,
            manifest.components.behavior_hash,
        ) {
            (Some(_), Some(h)) => Some(self.fetch_blob(h, peers).await?),
            (Some(_), None) => {
                return Err(FetchError::ComponentMissingHash {
                    profile: "behavior",
                });
            }
            _ => None,
        };

        // 4. Write the materialized layout into a tempdir.
        let tempdir = tempfile::TempDir::new()?;
        let bundle_dir = tempdir.path().to_path_buf();
        let manifest_path = std::path::PathBuf::from("manifest.bincode");
        std::fs::write(bundle_dir.join(&manifest_path), &manifest_bytes)?;

        let components_dir = bundle_dir.join("components");
        std::fs::create_dir_all(&components_dir)?;
        if let Some(rel) = manifest.components.state_apply.as_deref() {
            std::fs::write(bundle_dir.join(rel), &state_apply_bytes)?;
        }
        if let (Some(rel), Some(bytes)) = (
            manifest.components.state_propose.as_deref(),
            state_propose_bytes.as_ref(),
        ) {
            std::fs::write(bundle_dir.join(rel), bytes)?;
        }
        if let (Some(rel), Some(bytes)) = (
            manifest.components.interaction.as_deref(),
            interaction_bytes.as_ref(),
        ) {
            std::fs::write(bundle_dir.join(rel), bytes)?;
        }
        if let (Some(rel), Some(bytes)) = (
            manifest.components.behavior.as_deref(),
            behavior_bytes.as_ref(),
        ) {
            std::fs::write(bundle_dir.join(rel), bytes)?;
        }

        let address = myrhiza_kernel::BundleAddress::Disk {
            bundle_dir,
            manifest_path,
        };
        Ok(MaterializedBundle {
            _tempdir: tempdir,
            address,
        })
    }

    async fn fetch_blob(
        &self,
        hash: BlobHash,
        peers: &[myrhiza_types::PeerPubkey],
    ) -> Result<Vec<u8>, FetchError> {
        // **Impl-time API verification**: the exact iroh-blobs 0.101.0
        // downloader API for "fetch by hash from these peers" varies
        // by version. Likely shapes:
        //   - self.blobs_protocol.downloader().download(hash, peers).await
        //   - self.store.fetch_from(...).await
        // Use whichever the rc-pinned crate exposes; adapt and
        // document in commit body.

        let iroh_hash = iroh_hash_from_blob_hash(hash);
        let _ = (iroh_hash, peers); // placeholder until adapted at impl time
        Err(FetchError::BlobsFetch("API adapted at impl time".into()))
    }
}
```

**Crate-boundary handling — `BundleAddress` move (known-up-front per spec §4.6):**

Move `BundleAddress` from `crates/kernel/src/install.rs` to `crates/types/src/bundle_address.rs` (a known-up-front step per spec §4.6's declared dep direction).

Spec §4.6 declares `distribution → types, manifest, network` and `kernel → distribution`. `MaterializedBundle::address: BundleAddress` (returned from `BundleDistribution::fetch`) requires `BundleAddress` to be reachable from `crates/distribution/`. Per the declared dep direction, the only place it can live without inducing a circular dep is `crates/types/` — alongside `BlobHash`, `EventHash`, `BundleHash`, all pure-data locator newtypes.

The move is reserved for T8 (not T4) because the dependency the move enables — `distribution` returning `BundleAddress` from `fetch` — only manifests at T8; relocating in T4 would be premature.

The clean dep direction after the move:

```
distribution → types (for BundleAddress)
kernel       → types (for BundleAddress)
kernel       → distribution (for BundleDistribution + MaterializedBundle)
```

`crates/kernel/src/lib.rs` retains `pub use myrhiza_types::BundleAddress` as the backwards-compat re-export so external code referring to `myrhiza_kernel::BundleAddress` continues to compile.

**Concrete change at T8:** create `crates/types/src/bundle_address.rs`:

```rust
// crates/types/src/bundle_address.rs
use std::path::PathBuf;
use crate::BlobHash;

#[derive(Debug, Clone)]
pub enum BundleAddress {
    Disk { bundle_dir: PathBuf, manifest_path: PathBuf },
    IrohBlob { manifest_hash: BlobHash },
}
```

`crates/types/src/lib.rs` adds `pub mod bundle_address; pub use bundle_address::BundleAddress;`. `crates/kernel/src/install.rs` removes the local definition; `crates/kernel/src/lib.rs` `pub use` from `myrhiza_types::BundleAddress` for backwards compat (the kernel's public surface still exposes it from its current path).

**Kernel-side wiring:** Add the dep in `crates/kernel/Cargo.toml`:

```toml
myrhiza-distribution = { path = "../distribution" }
```

Propagate the `network-iroh` feature:

```toml
network-iroh = [
    "myrhiza-network/network-iroh",
    "myrhiza-test-utils/network-iroh",
    "myrhiza-distribution/network-iroh",
    "dep:iroh",
    "dep:iroh-gossip",
    "dep:iroh-blobs",  # NEW transitively
]
```

(Adjust based on the current `crates/kernel/Cargo.toml` shape; the feature already exists per B-4.x.)

**Verification commands:**

```bash
cargo build --workspace
cargo build --workspace --all-features
cargo test --workspace --lib
cargo clippy --workspace --all-targets -- -D warnings
cargo tree -p myrhiza-distribution | head -20
cargo tree -p myrhiza-kernel | head -20
# Confirm no circular deps (cargo tree fails fast on circular).
```

**Risk:** The `BundleAddress` move is a public-surface migration affecting every user of the type. Mitigation: keep `pub use myrhiza_types::BundleAddress` in `crates/kernel/src/lib.rs` so external code referring to `myrhiza_kernel::BundleAddress` continues to work (re-export, no rename). The 9 call sites updated in T4 are already on the new enum shape; the type-location move is invisible to them.

**Commit message:**
```
feat(distribution,kernel): BundleDistribution::fetch + materialization + crate wiring (B-10 §3.2 + §4.3 + §3.5)

Fetch path: pull manifest blob (BLAKE3-verified by iroh-blobs);
decode; pull each declared component blob; verify each *_hash matches
manifest claim; materialize into tempdir; return BundleAddress::Disk.

Crate-boundary: move BundleAddress from crates/kernel/src/install.rs
to crates/types/src/bundle_address.rs (a known-up-front step per spec
§4.6 dep direction; myrhiza_kernel re-exports for backwards compat).
distribution returns BundleAddress; kernel imports BundleDistribution.

myrhiza-kernel gains dep on myrhiza-distribution; network-iroh feature
propagates transitively.
```

---

## Task T9 — Subscription dispatch: bad-sig drop at gossip edge + `PeerWarning::SignatureInvalid` surfacing

**Spec ref:** §3.3 paragraph 3 (envelopes failing signature verify dropped at subscription dispatch BEFORE reaching `RevocationLog::apply`); §6.4 (kernel-tier revocation propagation test).

**Subject:** Wire the gossip subscription dispatch in `crates/distribution/src/dispatch.rs` (or in `crates/kernel/src/runtime.rs` if simpler) such that revocation + publication envelopes failing Ed25519 verification against the author pubkey are dropped at the network boundary and surface `PeerWarning::SignatureInvalid` consistent with B-4.8's existing pattern. `RevocationLog::apply` never sees forged envelopes.

**Files touched:**
- Create: `crates/distribution/src/dispatch.rs` (a thin wrapper that takes a `RevocationEvent` envelope-bytes + author pubkey, returns either a verified `RevocationEvent` or a typed reason for the drop)
- Modify: `crates/distribution/src/lib.rs` (re-export the dispatch helpers)
- Modify: `crates/kernel/src/runtime.rs` (consume the dispatch in the revocation + publication subscribe loops; emit `PeerWarning::SignatureInvalid` on rejection)

**Implementation notes:**

The dispatch helper does **only** signature verification + reason-length cap; it does NOT touch the log state machine. The state machine's existing `RevocationLog::apply` covers seq monotonicity etc. — but `apply` should NEVER be called with a forged signature, so the verify-at-edge step is structurally required.

```rust
//! Subscription dispatch for revocation + publication envelopes.
//!
//! Verifies Ed25519 signatures at the gossip-receive boundary before
//! the envelope reaches the state machine. Forged envelopes are
//! dropped; the kernel emits `PeerWarning::SignatureInvalid` so the
//! warning is observable, consistent with B-4.8's existing pattern.
//!
//! Per B-10 spec §3.3 paragraph 3.

use ed25519_dalek::{Signature as DalekSignature, VerifyingKey};
use myrhiza_types::AuthorPubkey;

use crate::publication::{MAX_VERSION_LEN, PublicationEvent};
use crate::revocation::{MAX_REASON_LEN, RevocationEvent};

/// Reasons a dispatch verification can fail. These map 1:1 to
/// `PeerWarning::SignatureInvalid { reason }` so the kernel can
/// surface them with the existing B-4.8 observability surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchReject {
    /// Ed25519 signature verification failed against the author key.
    SignatureInvalid,
    /// Author pubkey was not a valid Ed25519 curve point.
    AuthorPubkeyMalformed,
    /// `reason` or `version` field exceeded its cap.
    FieldTooLong,
}

/// Verify a revocation envelope at the dispatch boundary.
///
/// On `Ok`, the envelope's signature is valid against `author` and
/// the kernel may proceed to call `RevocationLog::apply`. On `Err`,
/// drop the envelope and surface `PeerWarning::SignatureInvalid`.
///
/// # Errors
///
/// See [`DispatchReject`].
pub fn verify_revocation(
    event: &RevocationEvent,
    author: &AuthorPubkey,
) -> Result<(), DispatchReject> {
    if event.reason.len() > MAX_REASON_LEN {
        return Err(DispatchReject::FieldTooLong);
    }
    let vk = VerifyingKey::from_bytes(author.as_bytes())
        .map_err(|_| DispatchReject::AuthorPubkeyMalformed)?;
    let sig = DalekSignature::from_bytes(&event.signature);
    let target = event.signing_target();
    vk.verify_strict(&target, &sig)
        .map_err(|_| DispatchReject::SignatureInvalid)?;
    Ok(())
}

/// Verify a publication envelope at the dispatch boundary.
///
/// See [`verify_revocation`] for semantics.
pub fn verify_publication(
    event: &PublicationEvent,
    author: &AuthorPubkey,
) -> Result<(), DispatchReject> {
    if event.version.len() > MAX_VERSION_LEN {
        return Err(DispatchReject::FieldTooLong);
    }
    let vk = VerifyingKey::from_bytes(author.as_bytes())
        .map_err(|_| DispatchReject::AuthorPubkeyMalformed)?;
    let sig = DalekSignature::from_bytes(&event.signature);
    let target = event.signing_target();
    vk.verify_strict(&target, &sig)
        .map_err(|_| DispatchReject::SignatureInvalid)?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use myrhiza_types::BlobHash;

    #[test]
    fn verify_revocation_accepts_genuine_signature() {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let pk = AuthorPubkey::from_bytes(sk.verifying_key().to_bytes());
        let mut ev = RevocationEvent {
            revoked_bundle_hash: BlobHash::from_bytes([0xAA; 32]),
            reason: "x".into(),
            revoked_at: 0,
            revocation_seq: 1,
            signature: [0u8; 64],
        };
        let target = ev.signing_target();
        ev.signature = sk.sign(&target).to_bytes();
        verify_revocation(&ev, &pk).expect("genuine sig accepted");
    }

    #[test]
    fn verify_revocation_rejects_forged_signature() {
        let real_sk = SigningKey::from_bytes(&[7u8; 32]);
        let pk = AuthorPubkey::from_bytes(real_sk.verifying_key().to_bytes());
        let attacker_sk = SigningKey::from_bytes(&[42u8; 32]);
        let mut ev = RevocationEvent {
            revoked_bundle_hash: BlobHash::from_bytes([0xAA; 32]),
            reason: "x".into(),
            revoked_at: 0,
            revocation_seq: 1,
            signature: [0u8; 64],
        };
        let target = ev.signing_target();
        ev.signature = attacker_sk.sign(&target).to_bytes();
        let err = verify_revocation(&ev, &pk).expect_err("forged must reject");
        assert_eq!(err, DispatchReject::SignatureInvalid);
    }
}
```

**Runtime wiring sketch** (in `crates/kernel/src/runtime.rs`):

The runtime gains a subscribe-to-revocation-topic helper run at install time (per spec §10.5 step 7 + §4.5 step 7-8). When a `RevocationEvent` arrives:

```rust
let envelope: RevocationEvent = canonical_bincode().deserialize(&raw)?;
match myrhiza_distribution::dispatch::verify_revocation(&envelope, &author_pubkey) {
    Ok(()) => {
        // Hand to the state machine.
        match revocation_log.clone().apply(&envelope, &author_pubkey) {
            Ok(new) => { revocation_log = new; /* emit RevocationApplied */ }
            Err(e) => { /* observability — drift in seq cap, etc. */ }
        }
    }
    Err(DispatchReject::SignatureInvalid) => {
        peer_warnings.lock().expect("…").push(
            PeerWarning::SignatureInvalid { peer: last_hop_peer, reason: /* tag */ }
        );
    }
    Err(_) => { /* observability for malformed pubkey / too-long fields */ }
}
```

**Important:** the spec's "minimum-viable" wiring for B-10 is that:

- The dispatch helpers exist and are unit-tested in `crates/distribution/`.
- The `Runtime::start` integration (subscribing to per-author revocation topics on install) **may be deferred** to a follow-up slice IF kernel-side integration is too large to land in B-10's 5-7 day envelope. The kernel-tier acceptance test in T11 only requires the publish-then-fetch loop; revocation propagation is the OPTIONAL second kernel-tier test (spec §6.4). If T9 + T11 alone fit in the day envelope, defer §6.4 to a B-10 polish PR.

The plan landing rule: T9 ships the dispatch helpers + unit tests. The kernel runtime-level revocation subscription is wired ONLY if T11 fits with budget remaining. Document the choice in T9's commit body.

**Verification commands:**

```bash
cargo test -p myrhiza-distribution --features network-iroh --lib dispatch::tests
cargo build --workspace --all-features
cargo clippy --workspace --all-features --all-targets -- -D warnings
```

**Risk:** Surfacing `PeerWarning::SignatureInvalid` requires the existing variant from B-4.8 to be reusable for "envelope sig wrong" not just "drift sig wrong." Inspect the variant shape (`crates/kernel/src/runtime.rs:240`) — if its fields don't fit, extend with a new variant (e.g. `PeerWarning::RevocationSignatureInvalid`) rather than overload `SignatureInvalid`. Stay within spec scope: B-10 §3.3 says "consistent with the `PeerWarning::SignatureInvalid` path (already shipped in B-4.8)"; if "consistent" requires a new variant, add it — that's structurally needed, not scope creep.

**Commit message:**
```
feat(distribution): subscription dispatch verifies envelope signatures at gossip edge (B-10 §3.3)

verify_revocation + verify_publication: Ed25519 signature check
against author pubkey before envelope reaches state machine. Forged
envelopes are dropped; observability via PeerWarning::SignatureInvalid
consistent with B-4.8 pattern.

State machines never see forged envelopes — `apply` invariants
preserved structurally, not by trust.
```

---

## Task T10 — `IrohHarness` extends to register `iroh_blobs::BlobsProtocol::ALPN`

**Spec ref:** §3.6 paragraph 5 ("iroh-blobs adds the BlobsProtocol ALPN registration on the router — one additional ALPN per peer — which is uncontroversial alongside the existing gossip + heads-request ALPN registrations"); §6.3 (kernel-tier acceptance test setup).

**Subject:** Extend `crates/test-utils/src/iroh_harness.rs` so each spawned iroh peer also registers `iroh_blobs::BlobsProtocol::ALPN`. Mechanical: one additional `.accept(...)` line in the `iroh::protocol::Router::builder` chain. Add an `IrohPeerStack::distribution: Option<BundleDistribution>` field so kernel-tier tests can publish + fetch through the harness.

**Files touched:**
- Modify: `crates/test-utils/Cargo.toml` (add `myrhiza-distribution` dep under `network-iroh` feature)
- Modify: `crates/test-utils/src/iroh_harness.rs` (extend `IrohPeerStack`, `spawn_iroh_peer`, `IrohHarness::spawn_peer`)

**Implementation notes:**

`crates/test-utils/Cargo.toml` `[dependencies]` already has `iroh = { workspace = true, optional = true }`. Add:

```toml
myrhiza-distribution = { path = "../distribution", optional = true }
iroh-blobs = { workspace = true, optional = true }
```

Extend the `network-iroh` feature:

```toml
network-iroh = [
    "myrhiza-network/network-iroh",
    "myrhiza-distribution/network-iroh",
    "dep:iroh",
    "dep:iroh-gossip",
    "dep:iroh-blobs",
    "dep:myrhiza-distribution",
]
```

In `iroh_harness.rs`, extend `IrohPeerStack`:

```rust
pub struct IrohPeerStack {
    pub endpoint: iroh::Endpoint,
    pub gossip: iroh_gossip::Gossip,
    pub router: iroh::protocol::Router,
    pub network: IrohNetwork,
    /// BundleDistribution handle for iroh-blobs publish + fetch.
    /// Per B-10 spec §3.6 / §6.3. Lives on the stack so the local
    /// `MemStore` + `BlobsProtocol` outlive any test borrowing the
    /// peer.
    pub distribution: myrhiza_distribution::BundleDistribution,
}
```

Extend `spawn_iroh_peer`:

```rust
pub async fn spawn_iroh_peer(
    lookup: &MemoryLookup,
    iroh_secret: Option<[u8; 32]>,
    register_heads_alpn: bool,
) -> IrohPeerStack {
    let mut endpoint_builder = iroh::Endpoint::builder(iroh::endpoint::presets::Minimal)
        .address_lookup(lookup.clone());
    if let Some(bytes) = iroh_secret {
        endpoint_builder = endpoint_builder.secret_key(iroh::SecretKey::from_bytes(&bytes));
    }
    let endpoint = endpoint_builder.bind().await.expect("iroh endpoint bind");
    lookup.add_endpoint_info(endpoint.addr());
    let gossip = iroh_gossip::Gossip::builder().spawn(endpoint.clone());
    let network = IrohNetwork::new(endpoint.clone(), gossip.clone());
    let distribution = myrhiza_distribution::BundleDistribution::new(endpoint.clone());

    let mut builder = iroh::protocol::Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, gossip.clone())
        // NEW per B-10 §3.6: BlobsProtocol ALPN registered on every
        // iroh peer so iroh-blobs publish/fetch work alongside gossip.
        .accept(iroh_blobs::ALPN, distribution.protocol_handler().clone());
    if register_heads_alpn {
        builder = builder.accept(HEADS_REQUEST_ALPN, network.protocol_handler());
    }
    let router = builder.spawn();
    IrohPeerStack {
        endpoint,
        gossip,
        router,
        network,
        distribution,
    }
}
```

**API note**: `iroh_blobs::ALPN` (top-level const) per spec §6.3. If the actual const is `iroh_blobs::BlobsProtocol::ALPN` (associated const), adapt. The `protocol_handler().clone()` invocation assumes `BlobsProtocol: Clone` (likely — iroh's protocol handlers Clone via Arc); if not, take a reference or stash the handle differently. **The structural goal is: `BlobsProtocol::ALPN` registered against the same router that holds gossip + heads.**

**Pre-integration: confirm the ALPN const name in iroh-blobs 0.101.0.**

The spec sketches `iroh_blobs::ALPN` (top-level) but the actual const may be `iroh_blobs::BlobsProtocol::ALPN` (associated) or a different identifier in the rc-pinned crate. Run ONE of the following before writing the `.accept(...)` line:

```bash
# Option A: grep the iroh-blobs source under cargo's registry cache
cargo metadata --format-version 1 \
    | jq -r '.packages[] | select(.name=="iroh-blobs") | .manifest_path' \
    | xargs dirname \
    | xargs -I{} rg 'pub const ALPN|impl ProtocolHandler' {}/src/ \
    | head -10

# Option B: build the docs and grep the generated HTML / module index
cargo doc -p iroh-blobs --no-deps 2>/dev/null \
    && rg 'pub const ALPN|impl ProtocolHandler' target/doc/iroh_blobs/ \
    | head -10
```

Implementer MUST confirm the exact const path (`iroh_blobs::ALPN` vs `iroh_blobs::BlobsProtocol::ALPN` vs other) before writing the integration line in `iroh_harness.rs`. Document the actual path used in the commit body if it differs from the spec sketch.

**Verification commands:**

```bash
cargo build -p myrhiza-test-utils --features network-iroh
cargo test -p myrhiza-test-utils --features network-iroh --lib iroh_harness::tests
# Existing E2E iroh tests must still pass:
cargo test -p myrhiza-kernel --features network-iroh --test iroh_convergence
cargo test -p myrhiza-kernel --features network-iroh --test iroh_coexistence
cargo clippy -p myrhiza-test-utils --features network-iroh --all-targets -- -D warnings
```

**Risk:** Adding the `iroh_blobs::ALPN` registration changes the router's ALPN-accept set. Existing iroh_convergence / iroh_coexistence tests must continue to pass — verify before commit. The existing identity-alignment test (`iroh_secret_aligns_network_pubkey_with_peer_key`) is unaffected.

**Commit message:**
```
feat(test-utils): IrohHarness registers iroh_blobs::BlobsProtocol::ALPN (B-10 §3.6 + §6.3)

Adds one .accept(...) line in the Router builder chain and an
IrohPeerStack::distribution: BundleDistribution field. Mechanical
extension — every previously-passing iroh_convergence / iroh_coexistence
test continues to pass with no behavior delta beyond the new ALPN.

Sets up T11's kernel-tier acceptance test (publish + fetch over real
iroh-blobs through real QUIC over loopback).
```

---

## Task T11 — Kernel-tier acceptance: publish-on-A → fetch-on-B → instantiate → assert state (closes mvp.md §15.1 #1 against iroh-blobs)

**Spec ref:** §6.3 (the load-bearing test); §3.6 (mix-strategy — real iroh-blobs over loopback QUIC).

**Subject:** Add `crates/kernel/tests/iroh_bundle_distribution.rs`. Peer A publishes the counter bundle (manifest + state-apply + state-propose + interaction components) via `BundleDistribution::publish`. Peer B fetches by manifest hash via `BundleDistribution::fetch`. The materialized `BundleAddress::Disk` feeds `InstallFlow::load`. The loaded component bytes instantiate through `WasmtimeBackend::instantiate_state_apply` and apply genesis. The test asserts the resulting state-digest matches the canonical `0_i64.to_be_bytes()` byte string for the counter app.

**Files touched:**
- Create: `crates/kernel/tests/iroh_bundle_distribution.rs`
- (Possibly) Modify: `crates/test-utils/src/bundle.rs` (add an in-memory fixture builder if the existing `build_signed_counter_bundle_three_components` writes to disk before we have manifest bytes in hand)

**Implementation notes:**

```rust
//! B-10 kernel-tier acceptance: real iroh-blobs publish + fetch +
//! Runtime + WASM state-apply. Closes `mvp.md §15.1 #1` against the
//! iroh-blobs wire shape (vs the disk-only proxy currently passing).
//!
//! Per B-10 spec §6.3.

#![cfg(feature = "network-iroh")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::time::Duration;

use bincode::Options;
use myrhiza_backend::Backend;
use myrhiza_kernel::{InstallFlow, StateApplyHandle};
use myrhiza_types::canonical_bincode;
use myrhiza_wasmtime_backend::WasmtimeBackend;

mod helpers;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn b10_fetch_via_iroh_blobs_closes_mvp_15_1_criterion_1() {
    use iroh::address_lookup::MemoryLookup;
    use myrhiza_test_utils::iroh_harness::spawn_iroh_peer;

    let lookup = MemoryLookup::default();

    // Two iroh peers, distinct identities. Author = peer A; installer
    // = peer B. The harness layer (T10) wires iroh-blobs ALPN onto
    // both.
    let mut secret_a = [0u8; 32];
    secret_a[..8].copy_from_slice(&1_u64.to_be_bytes());
    let stack_a = spawn_iroh_peer(&lookup, Some(secret_a), true).await;

    let mut secret_b = [0u8; 32];
    secret_b[..8].copy_from_slice(&2_u64.to_be_bytes());
    let stack_b = spawn_iroh_peer(&lookup, Some(secret_b), true).await;

    // Build a counter bundle IN MEMORY — we need the canonical-bincode
    // manifest bytes to hand to publish() without first writing them
    // to disk. The existing build_signed_counter_bundle_three_components
    // writes to a tempdir; here we either (a) read the tempdir bytes
    // back, or (b) extract the in-memory build path. Pick (a) for
    // simplicity: build to tempdir, then read manifest + 3 wasm files
    // back as Vec<u8>.

    let (test_bundle, _disk_addr) =
        myrhiza_test_utils::bundle::build_signed_counter_bundle_three_components();
    let manifest_bytes = std::fs::read(test_bundle.bundle_dir.join(&test_bundle.manifest_path))
        .expect("read manifest");
    let manifest: myrhiza_manifest::schema::Manifest = canonical_bincode()
        .deserialize(&manifest_bytes)
        .expect("decode manifest");
    // The bundle dir layout: components/state-apply.wasm etc.
    let apply_bytes = std::fs::read(
        test_bundle
            .bundle_dir
            .join(manifest.components.state_apply.as_deref().unwrap()),
    )
    .expect("read state-apply");
    let propose_bytes = manifest.components.state_propose.as_deref().map(|p| {
        std::fs::read(test_bundle.bundle_dir.join(p)).expect("read state-propose")
    });
    let interaction_bytes = manifest.components.interaction.as_deref().map(|p| {
        std::fs::read(test_bundle.bundle_dir.join(p)).expect("read interaction")
    });

    // T3 extended `build_signed_counter_bundle_three_components` to
    // populate the *_hash fields with the BLAKE3 of each on-disk
    // component before signing. If this test fails on
    // FetchError::ComponentMissingHash, the fixture builder regressed
    // — root-cause in the fixture, not here.

    // PUBLISH on peer A.
    let manifest_hash = stack_a
        .distribution
        .publish(
            &manifest,
            &manifest_bytes,
            &apply_bytes,
            propose_bytes.as_deref(),
            interaction_bytes.as_deref(),
            None,
        )
        .await
        .expect("publish ok");

    // FETCH on peer B. Bootstrap = peer A.
    let peer_a_pk = stack_a.network.peer_pubkey();
    let materialized = tokio::time::timeout(
        Duration::from_secs(30),
        stack_b.distribution.fetch(manifest_hash, &[peer_a_pk]),
    )
    .await
    .expect("fetch timed out (per spec §5 risk row 2)")
    .expect("fetch ok");

    // LOAD via the existing install flow.
    let flow = InstallFlow::new();
    let loaded = flow.load(&materialized.address).expect("install + verify");

    // INSTANTIATE the state-apply component and apply genesis.
    let backend = WasmtimeBackend::new().expect("backend");
    let handle = backend
        .instantiate_state_apply(
            &loaded.component_bytes,
            &myrhiza_kernel::state_apply::DEFAULT_KERNEL_FUEL_TABLE,
            1, // kernel_fuel_table_version
        )
        .expect("instantiate");
    // ... apply genesis, assert state == 0_i64.to_be_bytes() ...
    // (Mirror the existing crates/kernel/tests/acceptance.rs patterns
    // for genesis + apply.)
}
```

**The key assertion:** after applying the canonical counter-genesis event, the state digest equals `0_i64.to_be_bytes()` (the counter's initial value). This is what closes `mvp.md §15.1 #1` — the WASM was fetched via iroh-blobs (not loaded from disk) and instantiated successfully.

**Verification commands:**

```bash
just build-fixtures  # ensure the counter-state-apply + -propose + -interaction WASM exist
cargo test -p myrhiza-kernel --features network-iroh --test iroh_bundle_distribution \
    b10_fetch_via_iroh_blobs_closes_mvp_15_1_criterion_1 -- --nocapture
cargo clippy -p myrhiza-kernel --features network-iroh --all-targets -- -D warnings
cargo test -p myrhiza-kernel --features network-iroh --test iroh_bundle_distribution \
    --release  # validate release-mode timing
```

**Acceptance:** the test passes within 30 s wall-clock (per spec §5 risk row 2 fetch timeout). If it consistently exceeds, the iroh-blobs API call is using a non-loopback discovery path or the verified-streaming bandwidth is misconfigured — root-cause before bumping the timeout.

**Risk:** This test exercises the largest surface change in B-10 and is therefore the highest-risk to flake. Mitigation: wrap the fetch in `tokio::time::timeout(Duration::from_secs(30), ...)` (spec §5 risk row 2). The settle-time discipline from E2E-1 (200-500 ms after spawn before first publish) applies — add a `tokio::time::sleep(Duration::from_millis(300)).await;` between spawning peer B and calling fetch if iroh-blobs discovery needs a swarm-warm-up window.

**Commit message:**
```
test(kernel/iroh_bundle_distribution): publish-on-A → fetch-on-B → instantiate (B-10 §6.3)

Closes mvp.md §15.1 #1 against the iroh-blobs wire shape. Real
iroh-blobs over loopback QUIC via IrohHarness (T10 extension).
Asserts the fetched component bytes instantiate cleanly and the
state-apply produces the canonical counter-genesis state digest.
```

---

## Task T12 — Documentation: gap-analysis update + docs README catalog entry + final lint pass

**Spec ref:** §8 (cross-references to update); CLAUDE.md "Documentation conventions" + the `organizing-docs` skill.

**Subject:** Update `docs/reports/2026-05-21-mvp-gap-analysis.md` item 14 from 🟡 → ✅ with cite-link to B-10. Add a catalog entry for B-10 in `docs/README.md` under both "Runtime core" and "App distribution" areas (the spec is cross-area). Run a workspace-wide `cargo fmt --all` + `cargo clippy --workspace --all-targets --all-features -- -D warnings` + `cargo test --workspace --all-features` clean pass.

**Files touched:**
- Modify: `docs/reports/2026-05-21-mvp-gap-analysis.md` (item 14 status + linkback)
- Modify: `docs/README.md` (B-10 entry under "Runtime core" + "App distribution")
- (Spec status flip: `**Status:** draft` → `**Status:** active` in `docs/specs/2026-05-26-b-10-bundle-distribution-design.md` already done at commit time of plan; OR flip in T12 if not yet)

**Implementation notes:**

`docs/README.md` "Runtime core" area gains:

```markdown
- [Plan B-10 — Bundle distribution + iroh-blobs fetch](specs/2026-05-26-b-10-bundle-distribution-design.md) — wires iroh-blobs 0.101.0 as the production bundle fetch transport; introduces `BundleAddress::IrohBlob` variant; ships per-author revocation + publication topics; new `crates/distribution/` workspace member. Closes mvp.md §15.1 #1 against the iroh-blobs wire shape. Realized by [plan B-10](plans/2026-05-26-b-10-bundle-distribution.md). `[active]`
```

`docs/README.md` "App distribution" area gains (cross-referenced):

```markdown
- [Plan B-10 — Bundle distribution + iroh-blobs fetch](specs/2026-05-26-b-10-bundle-distribution-design.md) — per-author revocation topic, per-author publication topic, iroh-blobs publish + fetch. `[active]`
```

`docs/reports/2026-05-21-mvp-gap-analysis.md` item 14:

```markdown
| 14. Bundle distribution + signing | ✅ | B-10 (2026-05-26) — iroh-blobs publish + fetch via `crates/distribution/`, per-author revocation + publication topics, `BundleAddress::IrohBlob` variant. See [`docs/plans/2026-05-26-b-10-bundle-distribution.md`](../plans/2026-05-26-b-10-bundle-distribution.md). |
```

**Final verification:**

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --tests
cargo test --workspace --all-features --tests
cargo build --workspace --all-features
just ci  # if the justfile recipe exists; else equivalent
```

**Risk:** A late lint finding from a feature-cross-product compile that wasn't covered by earlier per-task checks. Mitigation: the per-task verification commands cover `--features network-iroh` paths; this task's full `--all-features` clip catches any residual feature-interaction lint.

**Commit message:**
```
docs: B-10 catalog + gap-analysis update (B-10 §8)

- docs/README.md: B-10 entry under Runtime core + App distribution.
- docs/reports/2026-05-21-mvp-gap-analysis.md: item 14 🟡 → ✅.
- Final clippy/fmt/test clean across the workspace under network-iroh.
```

---

## Spec-coverage table

| Spec section | Task |
|---|---|
| §1 Goal | T0–T12 (the whole plan) |
| §2 Scope (in v1) | T0–T11 |
| §2 Scope (deferred) | — (intentionally not addressed) |
| §3.1 Wire format (two blobs) | T3, T7 |
| §3.2 Hash semantics (BLAKE3 of canonical-bincode manifest) | T7, T8 |
| §3.3 Revocation topic (per-author, gossip) | T2 (topic deriv), T5 (state machine), T9 (dispatch) |
| §3.4 Publication topic | T2 (topic deriv), T6 (state machine), T9 (dispatch) |
| §3.5 `BundleAddress` enum | T4 + T8 (move to types) |
| §3.6 Test strategy (mix) | T5, T6 (state-tier), T10, T11 (kernel-tier) |
| §3.7 `crates/distribution/` | T2 |
| §4.1 Wire format detail (manifest schema delta) | T3 |
| §4.2 Hash semantics formula | T1 (`BlobHash`), T2 (orphan-rule conversions) |
| §4.3 `BundleDistribution` API | T7 (publish), T8 (fetch) |
| §4.4 Revocation schema + state machine | T5 |
| §4.5 Publishing sequence | T7 + T8 + T9 wire together |
| §4.6 Crate dependency direction | T2 + T8 (Cargo.toml shape) |
| §4.7 Cargo.toml changes | T0 (workspace), T2 (crate) |
| §5 Risks (BundleAddress migration) | T4 |
| §6.1 State-tier revocation tests | T5 |
| §6.2 State-tier publication tests | T6 |
| §6.3 Kernel-tier iroh-blobs fetch acceptance | T11 (depends on T10) |
| §6.4 Kernel-tier revocation propagation (optional second test) | T9 (deferred to follow-up if time-budget exceeded) |
| §7 Surface change summary | T1 (BlobHash), T3 (manifest fields), T4 (BundleAddress), T7+T8 (BundleDistribution) |
| §8 Cross-references | T12 |
| §9 Prior-art consulted | — (no impl impact; consulted during spec authoring) |
| §10 Out-of-scope | — (do not implement) |
| §11 Estimate | (informational — see Estimate breakdown below) |
| §12 Open questions for plan writer | Q1 MemStore (T7), Q2 EventSender=None (T7), Q3 timeout=30s (T11), Q4 bench numbers (PR body), Q5 BundleShareUri deferred |

---

## Determinism callouts

The load-bearing determinism property in B-10 is **`RevocationLog::apply` purity** (and the parallel property on `PublicationLog::apply`):

- `RevocationLog::apply(self, &RevocationEvent, &AuthorPubkey) -> Result<RevocationLog, RevocationError>` is a pure function. No system-clock read. No network I/O. No filesystem I/O. No global mutable state. The output is structurally determined by the inputs — apply on two different peers with identical (prior, event, author) MUST produce byte-identical output state.

Why this matters: per CLAUDE.md, "state-apply components must be pure functions of (prior state, event) plus the deterministic helper set." Cross-peer convergence depends on this. The revocation log is a kernel-resident state machine analog — it doesn't run inside a WASM sandbox, but its purity contract is structurally identical because peers must agree on which bundles an author has revoked.

The plan's mechanical enforcement:

- T5's state-tier tests run `RevocationLog::apply` exclusively as a pure function (no tokio runtime, no spawning, no `std::time::Instant::now`).
- The `event.revoked_at` field is **explicitly informational** (per spec §4.4); the kernel does NOT trust it for ordering. `revocation_seq` is the authoritative ordering.
- The dispatch helper (T9) is also pure: `verify_revocation(&RevocationEvent, &AuthorPubkey) -> Result<(), DispatchReject>` does only Ed25519 verification + length check.

**Anti-pattern guard:** if a future change tempts adding "drop revocations older than N hours" via system clock — STOP. That's non-determinism. The correct shape is to extend the state machine to include peer-local age tracking driven by event arrival order, never by wall-clock time. Same discipline as the state-apply rule.

---

## Risks per task

| Task | Risk | Mitigation |
|---|---|---|
| T0 | `iroh-blobs 0.101.0` yanked or transitive version conflict | Verify pre-execution; spec §4.7 contingency path is "bump iroh-blobs to nearest compatible". Do NOT skip the verification step. |
| T1 | None (additive newtype) | — |
| T2 | Workspace-members list change must propagate to all cargo invocations | Per-task `cargo build --workspace` validates. |
| T3 | Manifest schema break invalidates pre-baked fixtures | Every fixture rebuilds + re-signs at test time — confirmed via grep that no `manifest.bincode` is persisted outside tempdirs. |
| T4 | 9-site mechanical migration; missing a site breaks the tree | `grep -rn "BundleAddress {" crates/ tests/` after migration verifies completeness. |
| T5 | Pure-function purity in apply — accidentally introducing a `SystemTime::now` would be a correctness bug | Test that compares two runs of apply with identical inputs across two threads must produce identical outputs (byte-equal). Add this as the first state-tier test. |
| T6 | Structural drift from `RevocationLog` shape | Keep `apply` skeleton parallel; deviate only on `latest_announcement` semantics. |
| T7 | iroh-blobs API name uncertainty | Adapt at impl time + document in commit body; precedent from B-4.0 / B-4.1. |
| T8 | `BundleAddress` move (known-up-front per spec §4.6) is a public-surface migration | Move `BundleAddress` to `crates/types/` (foundational crate); `crates/kernel/` re-exports for backwards compat. The move is the dep-direction prerequisite for `BundleDistribution::fetch` returning `BundleAddress` from `crates/distribution/`. |
| T9 | Reusing `PeerWarning::SignatureInvalid` from B-4.8 — may need a new variant if shape doesn't fit | Inspect `runtime.rs:240`; if needed, add `PeerWarning::RevocationSignatureInvalid` rather than overload. Within spec scope. |
| T10 | Existing iroh tests must continue to pass after ALPN registration | Per-task verification re-runs `iroh_convergence` + `iroh_coexistence`. |
| T11 | Test flake under iroh-blobs API churn or network jitter | `tokio::time::timeout(30s)`; release-mode validation. Settle-time before fetch per E2E-1 precedent. |
| T12 | Final lint pass surfaces feature-interaction warning | Per-task `--features network-iroh` checks should have caught it; T12 fullsweep is defense-in-depth. |

---

## Estimate breakdown

Matches spec §11's 5-7 day envelope:

- **Day 1 (T0–T2)**: workspace dep + `BlobHash` + `crates/distribution/` scaffold with conversions + topic derivation. Pure mechanical Cargo wiring + 1 newtype + 2 free fns. Risk-free.
- **Day 1.5 (T3–T4)**: manifest schema delta + `BundleAddress` enum migration (the 9-site mechanical migration). Touches many files but the changes are obvious. Half-day for T3, half-day for T4.
- **Day 2 (T5–T6)**: revocation + publication state machines + state-tier tests. Parallel implementations; T5 wraps a half-day, T6 mirrors in another half-day.
- **Day 3 (T7)**: `BundleDistribution::publish` + in-memory store wiring. The first iroh-blobs API contact — budget includes impl-time API adaptation.
- **Day 4 (T8)**: `BundleDistribution::fetch` + `MaterializedBundle` + crate-boundary move of `BundleAddress` to `crates/types/`. The longest single task because of the crate-boundary work.
- **Day 4.5 (T9)**: dispatch helpers + `PeerWarning` wiring. Half-day if `PeerWarning::SignatureInvalid` already fits; full day if a new variant is needed.
- **Day 5 (T10–T11)**: harness extension (mechanical) + the load-bearing kernel-tier acceptance test. Most of the day is debugging iroh-blobs API integration under real QUIC.
- **Day 5.5–6 (debugging buffer)**: empirical iroh-blobs API adjustments, flake shake-out, settle-time tuning.
- **Day 7 (T12)**: docs polish + final lint + PR open.

**Risk-adjusted upper bound: 7 days. Likely-case middle: 5-6 days.** Matches spec §11.

> **Risk note (T11 slip):** T11 may slip by 1 day per spec §5 risk row 1
> (iroh-blobs API rotation under exact-version pin — the iroh-blobs
> 0.101.0 surface differs from the spec's sketch in §4.3 in ways that
> only manifest under real fetch). Buffer is Day 5.5–6; if T11 still
> doesn't pass by end of Day 6, escalate per spec §5 contingency
> rather than expanding T11 into T12's docs window.

---

## Self-review (per writing-plans skill)

1. **Spec coverage**: Every spec section maps to at least one task. The deferred §6.4 (revocation propagation kernel-tier test) is explicitly called out as "optional within budget" in T9; if time-pressed, it becomes a follow-up PR.
2. **Placeholder scan**: One acknowledged placeholder — the T7/T8 `iroh-blobs` API names are tagged "adapt at impl time" with spec §4.3 / §9 / `prior-art/iroh/lessons.md` §Avoid row 1 backing the uncertainty. This matches the precedent set by B-4.0 (where iroh's preset arg + endpoint_id naming was similarly adapted at impl time). NOT a plan failure — it's documented uncertainty.
3. **Type consistency**: `BlobHash` defined in T1 used identically across T2–T11. `BundleAddress` enum lands in `crates/kernel` at T4 and relocates to `crates/types` at T8 per spec §4.6's declared dep direction — the move is reserved for T8 because that's when the cross-crate dependency first materializes; `crates/kernel/src/lib.rs` re-exports for backwards compat.
4. **Task order**: each task's deps precede it. T0 (workspace dep) before any feature-gated code. T1 (`BlobHash`) before T3 (manifest fields use it) + T2 (conversions use it). T3 (manifest fields) before T7 (publish populates them) + T8 (fetch reads them). T4 (`BundleAddress` enum) before T8 (fetch returns it). T5 + T6 (state machines) before T9 (dispatch wires them). T10 (harness) before T11 (test uses harness). T11 closes the v1 criterion; T12 polishes.
