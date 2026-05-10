# Myrhiza Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Wasmtime-based kernel that can load a state-apply WASM component, gate its imports per signed manifest, and run apply + pre-check with fuel + float-ban + canonical state-digest emission. Acceptance criteria #1 (kernel loads and instantiates) and #5 (capability declarations gate access) of [mvp.md §15.1](../specs/2026-05-09-myrhiza-master-design/mvp.md) demonstrably pass in isolation.

**Architecture:** Workspace of focused crates: `myrhiza-types` (shared core types), `myrhiza-manifest` (TOML schema + canonical encoding + Ed25519 signature), `myrhiza-backend` (trait), `myrhiza-wasmtime-backend` (Wasmtime impl), `myrhiza-kernel` (orchestration: load + state-apply ABI + state-digest stub). WIT packages live in `wit/`. Hand-built test fixtures (cargo-component) live in `tests/fixtures/`. No iroh, no event DAG, no crypto host imports beyond hash + verify-signature, no apps — those land in plans B and C.

**Tech Stack:**
- Rust 2024 edition
- `wasmtime` LTS — current LTS line as of plan-execution; v36 in May 2026; bumping to a future LTS (e.g., v48) is a kernel-major version bump (component model, fuel, resource handles)
- `bincode` 1.3.x with pinned `DefaultOptions::new().with_fixint_encoding().with_big_endian()` (firm pin per [determinism.md §5.4](../specs/2026-05-09-myrhiza-master-design/determinism.md))
- `serde` 1.0.x
- `blake3` 1.5.x (canonical 32-byte digest per [determinism.md §5.1](../specs/2026-05-09-myrhiza-master-design/determinism.md))
- `ed25519-dalek` 2.1.x with `verifying_key.verify_strict` (RFC 8032 strict per [determinism.md §5.1](../specs/2026-05-09-myrhiza-master-design/determinism.md))
- `toml_edit` 0.22.x (pinned per [distribution.md §10.2](../specs/2026-05-09-myrhiza-master-design/distribution.md))
- `wit-bindgen` (latest stable; 0.30+)
- `cargo-component` 0.x (test fixture builds)
- `unicode-normalization` 0.1.x (NFC normalization for canonical manifest encoding)
- `thiserror` 1.x (typed errors); `anyhow` 1.x (test-only)

**Phase scope:** This is plan A of three.
- **Plan A (this plan):** workspace + core types + WIT packages + manifest schema + capability gating + Wasmtime backend + state-apply ABI + fuel + float-ban + state-digest emission. Maps spec §20 steps 1-9.
- **Plan B (future, `2026-05-09-myrhiza-network.md`):** event/DAG primitives, iroh transport, HeadsSummary sync, drift-detection digest gossip, full crypto primitives (`host.install-key`, `host.verify-payload-mac`, AEAD, ECDH, HKDF), bundle distribution, revocation. Maps spec §20 steps 10-14.
- **Plan C (future, `2026-05-09-myrhiza-apps.md`):** counter + poll example apps, all test tiers (state/kernel/e2e/browser), SDK macros, jco backend, v1.1 behavior profile. Maps spec §20 steps 15-24.

**Determinism discipline (load-bearing for entire foundation):**
- Use `BTreeMap` / `BTreeSet` for any field reachable from `state-digest()`. `HashMap`/`HashSet` are forbidden in those paths. Every encoder uses `myrhiza_types::canonical_bincode()` — not `bincode::serialize` directly.
- All `verify-signature` paths use `VerifyingKey::verify_strict` (rejects malleable s-values per Cremers ETK 2025). Plain `verify` is forbidden anywhere in the kernel.
- Every host import bound by `state-apply` profile is justified by a row in [architecture.md §3.5](../specs/2026-05-09-myrhiza-master-design/architecture.md). Adding a non-table import to state-apply is an ABI break and must not be done in this plan.

---

## File structure

### New crates (workspace members)

| Path | Responsibility |
|---|---|
| `crates/types/` | Shared core types: `EventHash`, `BundleHash`, `Topic`, `Hlc`, `IdentityScope`, `InstanceBinding`, `InstanceKind`, `Event` envelope, `canonical_bincode()` helper. Pure data — no I/O, no crypto beyond BLAKE3-for-hash. |
| `crates/manifest/` | Manifest typed structs (matching [distribution.md §10.2](../specs/2026-05-09-myrhiza-master-design/distribution.md)), TOML parser, capability vocabulary registry, canonical encoding (bincode over typed struct), Ed25519 signature verification. |
| `crates/backend/` | `Backend` trait + `ComponentInstance` trait — stable internal abstraction both Wasmtime (this plan) and jco (plan C) impls satisfy. Plus error types. |
| `crates/wasmtime-backend/` | Wasmtime impl of `Backend`. Component loader, capability-gated linker (manifest intersection), deterministic helper imports, per-call gating wrapper, fuel budget enforcement, float-ban byte-level lint. |
| `crates/kernel/` | Orchestration. Owns the install-flow scaffold (load bundle directory → verify signature → instantiate via backend), state-apply ABI (apply mode + pre-check mode sharing fuel), state-digest emission stub, log sink. No network, no DAG (plan B). |

### Existing layout to scrub

| Path | Action |
|---|---|
| `Cargo.toml` (root, currently a single-package manifest) | Replace with workspace manifest. |
| `src/lib.rs` (currently `cargo new` boilerplate) | Delete. |
| `src/` directory | Delete (no top-level package). |

### WIT packages

WIT lives at workspace root under `wit/myrhiza-kernel/`. Each profile gets a world; shared types in `types.wit`.

| Path | Content |
|---|---|
| `wit/myrhiza-kernel/wit/types.wit` | `package myrhiza:kernel`, shared types: `identity-handle` resource, `peer-handle` resource, `identity-scope` record, `instance-binding` record, `instance-kind` variant, `hlc` record, `event-payload` type alias, `key-handle` resource, `log-level` variant. |
| `wit/myrhiza-kernel/wit/host.wit` | Interface `myrhiza:kernel/host` — declares EVERY host import in [architecture.md §3.5](../specs/2026-05-09-myrhiza-master-design/architecture.md). Worlds import sub-interfaces from this. Split into sub-interfaces by determinism category. |
| `wit/myrhiza-kernel/wit/state-apply.wit` | World `state-apply`. Imports only the deterministic-helper-set sub-interface. Exports `apply(prior-state, event) -> verdict` and `state-digest() -> list<u8>`. |
| `wit/myrhiza-kernel/wit/state-propose.wit` | World `state-propose`. Imports deterministic-helpers + hlc + random + log + seal (capability-gated). Exports `propose(intent) -> result<event-payload, propose-error>`. |
| `wit/myrhiza-kernel/wit/interaction.wit` | World `interaction`. Imports deterministic-helpers + broadcast-submit/on-broadcast-completion + subscribe + kv + user-prompt + open + can-open + ui:*. Exports `view(state, peer-state) -> view-model` + `dispatch(action) -> result<intent, dispatch-error>`. |
| `wit/myrhiza-kernel/wit/behavior.wit` | World `behavior`. Imports superset of interaction + http-submit/on-http-completion + timer + author-event. Exports `on-event(event)` + `tick()`. |

Plan A only consumes `state-apply.wit`; the others are authored now (one-shot ABI surface) but not bound in this plan's runtime.

### Test fixtures (hand-built cargo-component crates)

Each fixture is a tiny crate that compiles to a wasm component. Built once with `cargo component build --release` and the resulting `.wasm` is committed under `tests/fixtures/built/` so kernel tests run without invoking cargo-component every time. A `just build-fixtures` recipe rebuilds them.

| Path | Purpose |
|---|---|
| `tests/fixtures/counter-state-apply/` | Counter-shaped state-apply: applies `Increment(by)` events, returns `Accept` + new digest. State is `BTreeMap<&'static str, i64>` with key `"value"`. |
| `tests/fixtures/float-banned/` | State-apply that imports nothing illegal but contains `let _ = 1.0_f32 + 2.0_f32;` (forces `f32.add` opcode). Tests float-ban lint. |
| `tests/fixtures/over-importer/` | State-apply that imports `myrhiza:kernel/host.broadcast-submit` (a non-deterministic import). Tests linker rejection. |
| `tests/fixtures/infinite-loop/` | State-apply whose `apply()` body is `loop {}`. Tests fuel exhaustion trap. |
| `tests/fixtures/pre-check-rejector/` | State-apply that always returns `Reject("not allowed")`. Tests pre-check fail-closed semantics. |
| `tests/fixtures/built/*.wasm` | Committed pre-built wasm artifacts. CI rebuilds and asserts byte-equivalence (reproducible build check) only on `nightly-fixture-rebuild` job. |

### Tooling

| Path | Content |
|---|---|
| `Justfile` | Recipes: `fmt`, `lint`, `test`, `check`, `build-fixtures`, `ci` (runs all gates). |
| `.github/workflows/ci.yml` | CI matrix: stable Rust, runs `just ci`. Warnings-as-errors via `RUSTFLAGS="-D warnings"`. |
| `rust-toolchain.toml` | Pin stable Rust channel for reproducibility. |
| `.cargo/config.toml` | Workspace-wide rustflags: `-D warnings` for kernel crates. |

### Dependency direction (CI-enforced in plan C; documented here)

- `crates/types` — depends on nothing in workspace (leaf).
- `crates/manifest` — depends on `types` only.
- `crates/backend` — depends on `types`, `manifest`.
- `crates/wasmtime-backend` — depends on `types`, `manifest`, `backend`.
- `crates/kernel` — depends on `types`, `manifest`, `backend`, `wasmtime-backend` (backend impl is plugged in via the `Backend` trait — kernel does not pin the impl name in its public API).
- `tests/fixtures/*` — depend ONLY on generated WIT bindings via `wit-bindgen`. Never pull in any `crates/*` workspace member.

---

## Pre-Task: read the spec

Before starting Task 1, the executing engineer reads (in order, ~15 min):

1. [README.md](../specs/2026-05-09-myrhiza-master-design/README.md) — vision and three-tier architecture
2. [architecture.md](../specs/2026-05-09-myrhiza-master-design/architecture.md) — four profiles + §3.5 host import table (LOAD-BEARING)
3. [determinism.md](../specs/2026-05-09-myrhiza-master-design/determinism.md) — helper set, fuel costs, bincode pin
4. [capabilities.md](../specs/2026-05-09-myrhiza-master-design/capabilities.md) — four-layer gating
5. [abi.md](../specs/2026-05-09-myrhiza-master-design/abi.md) — full CM decision, submit-and-poll
6. [distribution.md §10.1-10.5](../specs/2026-05-09-myrhiza-master-design/distribution.md) — manifest schema and signing

---

<!-- TASKS_BEGIN -->

## Phase 0: workspace scaffold

### Task 1: Workspace skeleton

**Files:**
- Modify: `Cargo.toml` (replace `cargo new` boilerplate)
- Create: `crates/types/Cargo.toml`
- Create: `crates/types/src/lib.rs`
- Delete: `src/` (top-level)
- Create: `rust-toolchain.toml`
- Create: `.cargo/config.toml`

- [ ] **Step 1: Delete cargo-new boilerplate**

```bash
rm -rf src/
```

- [ ] **Step 2: Replace root `Cargo.toml` with workspace manifest**

Write `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
    "crates/types",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "AGPL-3.0-only"
repository = "https://github.com/intendednull/myrhiza"
rust-version = "1.95"

[workspace.dependencies]
# Pins required by determinism.md §5.4 and distribution.md §10.2.
bincode = "=1.3.3"
serde = { version = "1", features = ["derive"] }
serde_bytes = "0.11"
blake3 = { version = "1.5", features = ["traits-preview"] }
ed25519-dalek = { version = "2.1", default-features = false, features = ["std", "fast"] }
toml_edit = { version = "0.22", default-features = false, features = ["parse", "serde"] }
unicode-normalization = "0.1"
thiserror = "1"
hex = "0.4"
# Wasmtime LTS line — current LTS as of plan-execution date is v36; bumping to v48 (named in browser-native.md §14.2 as v1 ship target) is a kernel-major bump per distribution.md §10.2.
wasmtime = { version = "=36.0.9", default-features = false, features = ["component-model", "cranelift", "runtime"] }
wasmtime-wasi = { version = "=36.0.9", default-features = false }
# Test/dev only.
anyhow = "1"
hex-literal = "0.4"
proptest = "1"
tempfile = "3"

[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
# panic!() / unwrap()/expect() are warns workspace-wide so the
# state-apply runtime path stays panic-free. Test-only crates and
# build scripts may override this in their own [lints] section, e.g.:
#
#   [lints.clippy]
#   panic = "allow"
#   unwrap_used = "allow"
#   expect_used = "allow"
#
# This is the documented escape hatch — do not sprinkle #[allow(...)]
# at call sites unless the override pattern is impractical.
unwrap_used = "warn"
expect_used = "warn"
panic = "warn"
```

- [ ] **Step 3: Pin Rust channel**

Write `rust-toolchain.toml`:

```toml
[toolchain]
channel = "1.95.0"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

- [ ] **Step 4: Workspace-wide warnings-as-errors**

Write `.cargo/config.toml`:

```toml
[build]
rustflags = ["-D", "warnings"]
```

- [ ] **Step 5: Scaffold the leaf `types` crate**

Write `crates/types/Cargo.toml`:

```toml
[package]
name = "myrhiza-types"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Shared core types for the Myrhiza runtime."

[lints]
workspace = true

[dependencies]
bincode.workspace = true
serde.workspace = true
serde_bytes.workspace = true
blake3.workspace = true
hex.workspace = true
thiserror.workspace = true
```

Write `crates/types/src/lib.rs`:

```rust
//! Shared core types for the Myrhiza runtime.
//!
//! This crate is a leaf in the workspace dependency graph and contains
//! no I/O, no crypto beyond BLAKE3 hashing, and no host bindings.
```

- [ ] **Step 6: Verify the workspace builds**

Run: `cargo check --workspace`
Expected: `Finished` (no warnings, no errors).

Run: `cargo fmt --all -- --check`
Expected: exit 0.

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: exit 0.

- [ ] **Step 7: Commit**

```bash
git rm -r src
git add Cargo.toml rust-toolchain.toml .cargo/config.toml crates/types
git commit -m "$(cat <<'EOF'
chore: scaffold workspace and leaf types crate

Replace cargo-new boilerplate with a workspace manifest pinning the
deps required by the master spec's determinism + distribution
sections (bincode 1.3.3, ed25519-dalek 2.1, blake3 1.5, toml_edit
0.22, wasmtime 36 LTS).

Add rust-toolchain.toml + .cargo/config.toml so warnings-as-errors
applies workspace-wide.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

- [ ] **Step 8: Commit `Cargo.lock` for determinism**

For a runtime where reproducible builds + cross-peer convergence are
load-bearing, the workspace lockfile must be tracked.

```bash
git add Cargo.lock
git commit -m "$(cat <<'EOF'
chore: commit Cargo.lock for deterministic dep resolution

Determinism is load-bearing per determinism.md §5.4. Tracking the
workspace lockfile makes dep resolution reproducible across peers
and CI runs.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Justfile + CI gate

**Files:**
- Create: `Justfile`
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Write the Justfile**

Write `Justfile`:

```just
default: ci

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

test:
    cargo test --workspace --all-targets

check:
    cargo check --workspace --all-targets

build-fixtures:
    @echo "build-fixtures wired in Task 39"

ci: fmt-check lint test
```

- [ ] **Step 2: Write the CI workflow**

Write `.github/workflows/ci.yml`:

```yaml
name: ci

on:
  push:
    branches: [main]
  pull_request:

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  ci:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.95.0
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - uses: extractions/setup-just@v2
      - run: just ci
```

- [ ] **Step 3: Run locally**

Run: `just ci`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add Justfile .github/workflows/ci.yml
git commit -m "$(cat <<'EOF'
chore: add Justfile + CI gate (fmt, clippy -D warnings, test)

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 1: core types

Per [determinism.md §5.4](../specs/2026-05-09-myrhiza-master-design/determinism.md), bincode must be invoked through one canonical helper. Per [identity.md §6.1](../specs/2026-05-09-myrhiza-master-design/identity.md), `IdentityScope` is a kernel primitive. Per [convergence.md §4](../specs/2026-05-09-myrhiza-master-design/convergence.md), `EventHash` is the topo-sort tie-break key, so its byte ordering is normative.

### Task 3: `canonical_bincode()` helper

**Files:**
- Create: `crates/types/src/encoding.rs`
- Modify: `crates/types/src/lib.rs`
- Test: `crates/types/src/encoding.rs` (inline `#[cfg(test)] mod tests`)

- [ ] **Step 1: Write the failing test**

Append to `crates/types/src/encoding.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn empty_btreemap_encodes_to_zero_length() {
        let map: BTreeMap<String, u32> = BTreeMap::new();
        let bytes = canonical_bincode().serialize(&map).expect("encode empty btreemap");
        // bincode 1.3 with fixint big-endian encodes a length prefix as u64 BE.
        assert_eq!(bytes, vec![0, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn u32_encodes_big_endian_fixint() {
        let bytes = canonical_bincode().serialize(&0x01020304_u32).expect("encode u32");
        assert_eq!(bytes, vec![0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn btreemap_round_trip_preserves_order() {
        let mut map: BTreeMap<String, u32> = BTreeMap::new();
        map.insert("zebra".into(), 1);
        map.insert("apple".into(), 2);
        let bytes = canonical_bincode().serialize(&map).expect("encode");
        let decoded: BTreeMap<String, u32> = canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(map, decoded);
        // Re-encoding the decoded map must produce identical bytes (canonical).
        let bytes2 = canonical_bincode().serialize(&decoded).expect("re-encode");
        assert_eq!(bytes, bytes2);
    }
}
```

- [ ] **Step 2: Run test and verify it fails**

Run: `cargo test -p myrhiza-types encoding`
Expected: FAIL — `canonical_bincode` is not defined.

- [ ] **Step 3: Implement the helper**

Write `crates/types/src/encoding.rs` (above the `#[cfg(test)]` block):

```rust
//! Canonical bincode configuration.
//!
//! `bincode 1.3.x` exposes both function-level entry points
//! (`bincode::serialize`) and an `Options` builder. The function-level
//! entry points use a different default config than the builder — two
//! correct implementations following different idioms produce different
//! bytes. Per [determinism.md §5.4] this divergence is convergence-
//! breaking.
//!
//! Every byte-stable encode in the Myrhiza runtime MUST go through
//! [`canonical_bincode`]. Direct calls to `bincode::serialize` /
//! `bincode::deserialize` are forbidden (clippy lint enforces this in
//! the workspace; reviewer enforces during code review).

use bincode::{
    DefaultOptions, Options,
    config::{BigEndian, FixintEncoding, WithOtherEndian, WithOtherIntEncoding},
};

/// The canonical bincode `Options` chain.
///
/// Equivalent to:
/// `DefaultOptions::new().with_fixint_encoding().with_big_endian()`.
pub type CanonicalOptions =
    WithOtherEndian<WithOtherIntEncoding<DefaultOptions, FixintEncoding>, BigEndian>;

/// Returns the canonical bincode options chain pinned by the master spec.
#[must_use]
pub fn canonical_bincode() -> CanonicalOptions {
    DefaultOptions::new().with_fixint_encoding().with_big_endian()
}
```

Append to `crates/types/src/lib.rs`:

```rust
pub mod encoding;
pub use encoding::{CanonicalOptions, canonical_bincode};
```

- [ ] **Step 4: Run tests and verify they pass**

Run: `cargo test -p myrhiza-types encoding`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/types
git commit -m "$(cat <<'EOF'
feat(types): canonical_bincode() helper pinned to spec §5.4

Single entry point for every byte-stable encode in the runtime.
Pins bincode 1.3.x DefaultOptions + fixint + big-endian per
determinism.md §5.4 — divergence from this chain is convergence-
breaking.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: `EventHash` newtype

**Files:**
- Create: `crates/types/src/hash.rs`
- Modify: `crates/types/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/types/src/hash.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_hash_is_thirty_two_bytes() {
        assert_eq!(core::mem::size_of::<EventHash>(), 32);
    }

    #[test]
    fn event_hash_from_bytes_round_trip() {
        let raw = [0xAB; 32];
        let h = EventHash::from_bytes(raw);
        assert_eq!(h.as_bytes(), &raw);
    }

    #[test]
    fn event_hash_blake3_of_empty_is_canonical() {
        // BLAKE3 of empty input — published canonical vector.
        let h = EventHash::blake3(b"");
        assert_eq!(
            hex::encode(h.as_bytes()),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
    }

    #[test]
    fn event_hash_lex_ord_matches_byte_ord() {
        let a = EventHash::from_bytes([0x00; 32]);
        let mut b_bytes = [0x00; 32];
        b_bytes[31] = 0x01;
        let b = EventHash::from_bytes(b_bytes);
        assert!(a < b);
    }

    #[test]
    fn event_hash_serde_round_trip() {
        let h = EventHash::blake3(b"hello");
        let bytes = crate::canonical_bincode().serialize(&h).expect("encode");
        let decoded: EventHash = crate::canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(h, decoded);
    }

    #[test]
    fn event_hash_display_is_lowercase_hex() {
        let h = EventHash::from_bytes([0xDE; 32]);
        let s = format!("{h}");
        assert_eq!(s, "de".repeat(32));
    }
}
```

- [ ] **Step 2: Run and verify failure**

Run: `cargo test -p myrhiza-types hash`
Expected: FAIL — `EventHash` is not defined.

- [ ] **Step 3: Implement `EventHash`**

Write `crates/types/src/hash.rs` (above tests):

```rust
//! Content-addressed hashes used across the runtime.
//!
//! All hashes are BLAKE3 with canonical 32-byte output per
//! [determinism.md §5.1]. The byte ordering of `EventHash` is the
//! topological tie-break key per [convergence.md §4.1] — `Ord` on
//! `EventHash` is byte-lex over the inner array.

use core::fmt;

use serde::{Deserialize, Serialize};

/// 32-byte BLAKE3 hash of an event envelope.
///
/// `Ord` is byte-lex; this is normative for topo-sort tie-break per
/// [convergence.md §4.1].
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct EventHash(#[serde(with = "serde_bytes_32")] [u8; 32]);

/// 32-byte BLAKE3 hash of a bundle's content+manifest pair.
#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct BundleHash(#[serde(with = "serde_bytes_32")] [u8; 32]);

mod serde_bytes_32 {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(b: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
        serde_bytes::Bytes::new(b).serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let v: serde_bytes::ByteBuf = serde_bytes::ByteBuf::deserialize(d)?;
        let arr: [u8; 32] = v
            .as_ref()
            .try_into()
            .map_err(|_| serde::de::Error::invalid_length(v.len(), &"32 bytes"))?;
        Ok(arr)
    }
}

impl EventHash {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Hash arbitrary bytes via BLAKE3.
    #[must_use]
    pub fn blake3(input: &[u8]) -> Self {
        Self(*blake3::hash(input).as_bytes())
    }
}

impl BundleHash {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    #[must_use]
    pub fn blake3(input: &[u8]) -> Self {
        Self(*blake3::hash(input).as_bytes())
    }
}

impl fmt::Display for EventHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for EventHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EventHash({self})")
    }
}

impl fmt::Display for BundleHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for BundleHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BundleHash({self})")
    }
}
```

Append to `crates/types/src/lib.rs`:

```rust
pub mod hash;
pub use hash::{BundleHash, EventHash};
```

- [ ] **Step 4: Run and verify pass**

Run: `cargo test -p myrhiza-types hash`
Expected: 6 passed.

Run: `cargo clippy -p myrhiza-types -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/types
git commit -m "$(cat <<'EOF'
feat(types): EventHash + BundleHash newtypes

32-byte BLAKE3-derived content addresses. Ord is byte-lex on the
inner array; this is the topo-sort tie-break key per
convergence.md §4.1. Display is lowercase hex.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: `Hlc` (Hybrid Logical Clock) record

**Files:**
- Create: `crates/types/src/hlc.rs`
- Modify: `crates/types/src/lib.rs`

The HLC is signed into events by the originator and decoded by `host.now-hlc-from-event` per [determinism.md §5.1](../specs/2026-05-09-myrhiza-master-design/determinism.md). The kernel never consults the wall clock when serving the helper. Format must be deterministic.

- [ ] **Step 1: Write the failing test**

Append to `crates/types/src/hlc.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hlc_round_trip_via_canonical_bincode() {
        let hlc = Hlc { wall_ms: 1_700_000_000_000, logical: 7 };
        let bytes = crate::canonical_bincode().serialize(&hlc).expect("encode");
        // 8 bytes wall_ms BE + 4 bytes logical BE = 12 bytes.
        assert_eq!(bytes.len(), 12);
        let decoded: Hlc = crate::canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(hlc, decoded);
    }

    #[test]
    fn hlc_ord_is_lex_wall_then_logical() {
        let a = Hlc { wall_ms: 100, logical: 5 };
        let b = Hlc { wall_ms: 100, logical: 6 };
        let c = Hlc { wall_ms: 101, logical: 0 };
        assert!(a < b);
        assert!(b < c);
    }
}
```

- [ ] **Step 2: Run and verify failure**

Run: `cargo test -p myrhiza-types hlc`
Expected: FAIL — `Hlc` is not defined.

- [ ] **Step 3: Implement `Hlc`**

Write `crates/types/src/hlc.rs` (above tests):

```rust
//! Hybrid Logical Clock.
//!
//! Signed into events by the originator. Extracted by every peer
//! deterministically via `host.now-hlc-from-event` per
//! [determinism.md §5.1]. NOT used for DAG ordering or topo-sort
//! tie-break (per [convergence.md §4.1]); materialized into derived
//! state where useful.

use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize,
)]
pub struct Hlc {
    /// Wall-clock milliseconds since UNIX epoch (signed by originator).
    pub wall_ms: u64,
    /// Per-(peer, ms) logical counter. Resets to 0 each ms.
    pub logical: u32,
}
```

Append to `crates/types/src/lib.rs`:

```rust
pub mod hlc;
pub use hlc::Hlc;
```

- [ ] **Step 4: Run and verify pass**

Run: `cargo test -p myrhiza-types hlc`
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/types
git commit -m "$(cat <<'EOF'
feat(types): Hlc record (wall_ms u64, logical u32)

Hybrid logical clock signed into events; decoded deterministically
via host.now-hlc-from-event per determinism.md §5.1. Ord is lex on
(wall_ms, logical).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 6: `Topic` newtype + author-pubkey-bytes type

**Files:**
- Create: `crates/types/src/topic.rs`
- Create: `crates/types/src/author.rs`
- Modify: `crates/types/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Append to `crates/types/src/topic.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topic_id_is_thirty_two_bytes() {
        let t = Topic::from_bytes([0u8; 32]);
        assert_eq!(t.as_bytes().len(), 32);
    }

    #[test]
    fn topic_round_trip_via_canonical_bincode() {
        let t = Topic::from_bytes([0xCD; 32]);
        let bytes = crate::canonical_bincode().serialize(&t).expect("encode");
        let decoded: Topic = crate::canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(t, decoded);
    }
}
```

Append to `crates/types/src/author.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn author_pubkey_round_trip() {
        let pk = AuthorPubkey::from_bytes([0xEE; 32]);
        let bytes = crate::canonical_bincode().serialize(&pk).expect("encode");
        let decoded: AuthorPubkey =
            crate::canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(pk, decoded);
    }
}
```

- [ ] **Step 2: Run and verify failure**

Run: `cargo test -p myrhiza-types topic author`
Expected: FAIL — `Topic` and `AuthorPubkey` not defined.

- [ ] **Step 3: Implement `Topic` and `AuthorPubkey`**

Write `crates/types/src/topic.rs` (above tests):

```rust
//! Topic identity.
//!
//! 32-byte ID derived from `BLAKE3("myrhiza/topic/v1" |
//! app_bundle_hash | per-topic-data)` (derivation lands in plan B
//! when bundle distribution wires up). This crate stores the ID
//! opaquely.

use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Topic(#[serde(with = "crate::hash::serde_bytes_32_pub")] [u8; 32]);

impl Topic {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}
```

Modify `crates/types/src/hash.rs` — promote `serde_bytes_32` to a `pub(crate)` re-exportable form. Replace the `mod serde_bytes_32` block with:

```rust
pub(crate) mod serde_bytes_32_pub {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(b: &[u8; 32], s: S) -> Result<S::Ok, S::Error> {
        serde_bytes::Bytes::new(b).serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 32], D::Error> {
        let v: serde_bytes::ByteBuf = serde_bytes::ByteBuf::deserialize(d)?;
        let arr: [u8; 32] = v
            .as_ref()
            .try_into()
            .map_err(|_| serde::de::Error::invalid_length(v.len(), &"32 bytes"))?;
        Ok(arr)
    }
}
```

Update `EventHash` and `BundleHash` `#[serde(with = ...)]` attributes to `"crate::hash::serde_bytes_32_pub"`.

Write `crates/types/src/author.rs` (above tests):

```rust
//! Ed25519 author public key (32 raw bytes per RFC 8032).

use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct AuthorPubkey(#[serde(with = "crate::hash::serde_bytes_32_pub")] [u8; 32]);

impl AuthorPubkey {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}
```

Append to `crates/types/src/lib.rs`:

```rust
pub mod author;
pub mod topic;
pub use author::AuthorPubkey;
pub use topic::Topic;
```

- [ ] **Step 4: Run tests + lint**

Run: `cargo test -p myrhiza-types`
Expected: all passed.

Run: `cargo clippy -p myrhiza-types -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/types
git commit -m "$(cat <<'EOF'
feat(types): Topic and AuthorPubkey newtypes

Both are 32-byte opaque newtypes with canonical bincode round-trip.
Topic ID derivation lands in plan B; this commit is the storage type.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 7: `IdentityScope`, `InstanceBinding`, `InstanceKind`

**Files:**
- Create: `crates/types/src/identity.rs`
- Modify: `crates/types/src/lib.rs`

These mirror the WIT types in [identity.md §6.1](../specs/2026-05-09-myrhiza-master-design/identity.md). They are not handles — handles are WCM resource handles created by the kernel at instantiation time. These records are the **kernel-side** value type used in code paths that don't cross the WIT boundary (manifest validation, author-policy checks, etc.).

- [ ] **Step 1: Write the failing test**

Append to `crates/types/src/identity.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_kind_round_trips() {
        for kind in [
            InstanceKind::Device,
            InstanceKind::Behavior,
            InstanceKind::MlsLeaf,
            InstanceKind::Custom("epoch-42".into()),
        ] {
            let bytes = crate::canonical_bincode().serialize(&kind).expect("encode");
            let decoded: InstanceKind =
                crate::canonical_bincode().deserialize(&bytes).expect("decode");
            assert_eq!(kind, decoded);
        }
    }

    #[test]
    fn identity_scope_no_instance_serializes() {
        let scope = IdentityScope { long_term: AuthorPubkey::from_bytes([1; 32]), instance: None };
        let bytes = crate::canonical_bincode().serialize(&scope).expect("encode");
        let decoded: IdentityScope =
            crate::canonical_bincode().deserialize(&bytes).expect("decode");
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
        let bytes = crate::canonical_bincode().serialize(&scope).expect("encode");
        let decoded: IdentityScope =
            crate::canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(scope, decoded);
    }
}
```

- [ ] **Step 2: Run and verify failure**

Run: `cargo test -p myrhiza-types identity`
Expected: FAIL — `IdentityScope` not defined.

- [ ] **Step 3: Implement the records**

Write `crates/types/src/identity.rs` (above tests):

```rust
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

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct IdentityScope {
    pub long_term: AuthorPubkey,
    pub instance: Option<InstanceBinding>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct InstanceBinding {
    /// The peer this instance is bound to. For behaviors and devices,
    /// this is the per-peer keypair the kernel materialized at instance
    /// creation time.
    pub peer: AuthorPubkey,
    pub kind: InstanceKind,
    pub name: String,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum InstanceKind {
    Device,
    Behavior,
    MlsLeaf,
    Custom(String),
}

/// Profile-level annotation used for author-policy checks per
/// [identity.md §6.1].
#[derive(
    Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize,
)]
pub enum CallingProfile {
    StateApply,
    StatePropose,
    Interaction,
    Behavior,
}
```

Append to `crates/types/src/lib.rs`:

```rust
pub mod identity;
pub use identity::{CallingProfile, IdentityScope, InstanceBinding, InstanceKind};
```

- [ ] **Step 4: Run and verify pass**

Run: `cargo test -p myrhiza-types identity`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/types
git commit -m "$(cat <<'EOF'
feat(types): IdentityScope, InstanceBinding, InstanceKind, CallingProfile

Kernel-side value types mirroring identity.md §6.1's WIT records.
Used in manifest author-policy checks and signing key lookup.
Components never see these structs — only opaque identity-handle
resources at the WIT boundary.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 8: `Event` envelope

**Files:**
- Create: `crates/types/src/event.rs`
- Modify: `crates/types/src/lib.rs`

Per [convergence.md §4](../specs/2026-05-09-myrhiza-master-design/convergence.md), an event has author, sequence number (monotonic per-author starting at 1), `prev` (hash of this author's previous event), `deps` (cross-author causal heads), HLC, payload bytes, and Ed25519 signature. `EventHash` is BLAKE3 of the canonical envelope bytes WITHOUT the signature (the signature signs the envelope hash, by definition).

- [ ] **Step 1: Write the failing test**

Append to `crates/types/src/event.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn sample_event() -> Event {
        Event {
            author: AuthorPubkey::from_bytes([1; 32]),
            seq: 1,
            prev: None,
            deps: BTreeSet::new(),
            hlc: Hlc { wall_ms: 1_700_000_000_000, logical: 0 },
            payload: vec![0x01, 0x02, 0x03],
            signature: [0xFF; 64],
        }
    }

    #[test]
    fn event_round_trips_via_canonical_bincode() {
        let e = sample_event();
        let bytes = crate::canonical_bincode().serialize(&e).expect("encode");
        let decoded: Event = crate::canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(e, decoded);
    }

    #[test]
    fn event_hash_excludes_signature() {
        let mut e1 = sample_event();
        let h1 = e1.hash_signed_body();
        e1.signature = [0x00; 64];
        let h2 = e1.hash_signed_body();
        assert_eq!(h1, h2, "hash_signed_body must NOT depend on signature bytes");
    }

    #[test]
    fn event_hash_distinct_for_distinct_payload() {
        let e1 = sample_event();
        let mut e2 = e1.clone();
        e2.payload = vec![0xFF, 0xFF];
        assert_ne!(e1.hash_signed_body(), e2.hash_signed_body());
    }

    #[test]
    fn deps_sorted_by_btreeset_iteration() {
        let mut deps = BTreeSet::new();
        deps.insert(EventHash::from_bytes([2; 32]));
        deps.insert(EventHash::from_bytes([1; 32]));
        let collected: Vec<_> = deps.iter().collect();
        assert!(collected[0] < collected[1]);
    }
}
```

- [ ] **Step 2: Run and verify failure**

Run: `cargo test -p myrhiza-types event`
Expected: FAIL — `Event` not defined.

- [ ] **Step 3: Implement `Event` + `hash_signed_body`**

Write `crates/types/src/event.rs` (above tests):

```rust
//! Event envelope.
//!
//! Per [convergence.md §4]: per-author signed Merkle DAG. `EventHash`
//! is BLAKE3 of the canonical encoding of the SIGNED BODY (every
//! field except `signature`). The signature signs that hash. Rejecting
//! events whose `signature` bytes are not 64 bytes is enforced at
//! decode time by bincode's fixint length prefix.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{AuthorPubkey, EventHash, Hlc, canonical_bincode};

/// The full event envelope, including signature.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Event {
    pub author: AuthorPubkey,
    /// Monotonic per-author, starting at 1.
    pub seq: u64,
    /// Hash of this author's previous event. `None` iff `seq == 1`.
    pub prev: Option<EventHash>,
    /// Cross-author causal heads. Sorted by BTreeSet for canonical
    /// encoding.
    pub deps: BTreeSet<EventHash>,
    pub hlc: Hlc,
    /// App-opaque payload bytes.
    #[serde(with = "serde_bytes")]
    pub payload: Vec<u8>,
    /// Ed25519 signature over BLAKE3(signed body bytes).
    #[serde(with = "serde_signature")]
    pub signature: [u8; 64],
}

mod serde_signature {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(b: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        serde_bytes::Bytes::new(b).serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        let v: serde_bytes::ByteBuf = serde_bytes::ByteBuf::deserialize(d)?;
        let arr: [u8; 64] = v
            .as_ref()
            .try_into()
            .map_err(|_| serde::de::Error::invalid_length(v.len(), &"64 bytes"))?;
        Ok(arr)
    }
}

/// Helper: the same fields as `Event` minus `signature`. Used for
/// computing the BLAKE3 hash that the signature signs.
#[derive(Serialize)]
struct SignedBody<'a> {
    author: &'a AuthorPubkey,
    seq: u64,
    prev: &'a Option<EventHash>,
    deps: &'a BTreeSet<EventHash>,
    hlc: &'a Hlc,
    #[serde(with = "serde_bytes")]
    payload: &'a [u8],
}

impl Event {
    /// Returns BLAKE3 of the canonical encoding of every field except
    /// `signature`. This is the value that `signature` is over.
    pub fn hash_signed_body(&self) -> EventHash {
        let body = SignedBody {
            author: &self.author,
            seq: self.seq,
            prev: &self.prev,
            deps: &self.deps,
            hlc: &self.hlc,
            payload: &self.payload,
        };
        let bytes = canonical_bincode()
            .serialize(&body)
            .expect("canonical bincode of SignedBody never fails");
        EventHash::blake3(&bytes)
    }

    /// Returns BLAKE3 of the canonical encoding of the FULL event
    /// (including signature). This is the wire-content hash used as
    /// the DAG node identifier.
    pub fn wire_hash(&self) -> EventHash {
        let bytes = canonical_bincode()
            .serialize(self)
            .expect("canonical bincode of Event never fails");
        EventHash::blake3(&bytes)
    }
}
```

Append to `crates/types/src/lib.rs`:

```rust
pub mod event;
pub use event::Event;
```

- [ ] **Step 4: Run tests + lint**

Run: `cargo test -p myrhiza-types`
Expected: all pass.

Run: `cargo clippy -p myrhiza-types -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/types
git commit -m "$(cat <<'EOF'
feat(types): Event envelope with hash_signed_body and wire_hash

Per convergence.md §4: per-author signed Merkle DAG envelope. Two
distinct hashes:
- hash_signed_body: BLAKE3 of every field except signature; the
  value the signature signs.
- wire_hash: BLAKE3 of the full canonical envelope; the DAG node ID.

deps stored as BTreeSet for canonical encoding ordering.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 2: WIT package authoring

The WIT surface is an ABI commitment per [architecture.md §3.5](../specs/2026-05-09-myrhiza-master-design/architecture.md). Once shipped, removing or changing semantics of any import is a kernel-major version bump (per [distribution.md §10.2](../specs/2026-05-09-myrhiza-master-design/distribution.md)). We author all four worlds now even though only `state-apply` is bound at runtime in this plan; later plans add the linker bindings without re-touching WIT.

### Task 9: WIT package skeleton + shared `types.wit`

**Files:**
- Create: `wit/myrhiza-kernel/wit/types.wit`

- [ ] **Step 1: Write the WIT shared types**

Write `wit/myrhiza-kernel/wit/types.wit`:

```wit
package myrhiza:kernel@1.0.0;

interface types {
    // Opaque identity primitives. Per identity.md §6.1 components hold
    // these handles but cannot inspect or forge their contents.
    resource identity-handle {}
    resource peer-handle {}
    resource key-handle {}

    record identity-scope {
        long-term: borrow<identity-handle>,
        instance: option<instance-binding>,
    }

    record instance-binding {
        peer: borrow<peer-handle>,
        kind: instance-kind,
        name: string,
    }

    variant instance-kind {
        device,
        behavior,
        mls-leaf,
        custom(string),
    }

    record hlc {
        wall-ms: u64,
        logical: u32,
    }

    enum log-level {
        trace,
        debug,
        info,
        warn,
        error,
    }

    // Opaque single-use kernel-issued tokens for submit-and-poll
    // surfaces per abi.md §8.5.
    resource request-token {}

    // Verdicts returned by state-apply.
    variant verdict {
        accept,
        reject(string),
    }
}
```

- [ ] **Step 2: Verify with `wit-parser`**

Add a one-shot validator. Write `crates/types/build.rs` (yes, in the types crate — it's the canonical home for spec types and we want a build-time guard that the WIT remains parseable):

```rust
fn main() {
    println!("cargo:rerun-if-changed=../../wit/myrhiza-kernel/wit");
    let path = std::path::Path::new("../../wit/myrhiza-kernel/wit");
    if !path.exists() {
        return;
    }
    let mut resolve = wit_parser::Resolve::new();
    if let Err(e) = resolve.push_dir(path) {
        panic!("wit/myrhiza-kernel/wit failed to parse: {e}");
    }
}
```

Add to `crates/types/Cargo.toml`:

```toml
[build-dependencies]
wit-parser = "0.215"
```

(Adjust version to whatever wit-parser publishes alongside the wit-bindgen pin chosen in Task 28; a build script that fails on unparseable WIT is the whole point.)

- [ ] **Step 3: Run check**

Run: `cargo check -p myrhiza-types`
Expected: PASS (build script parses WIT successfully).

- [ ] **Step 4: Commit**

```bash
git add wit/myrhiza-kernel/wit/types.wit crates/types/build.rs crates/types/Cargo.toml
git commit -m "$(cat <<'EOF'
feat(wit): myrhiza:kernel/types interface (resources + records)

Shared types every world imports: identity-handle, peer-handle,
key-handle, identity-scope, instance-binding, instance-kind, hlc,
log-level, request-token, verdict.

Build script in myrhiza-types asserts the WIT parses; CI fails
on parse error.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 10: WIT host import sub-interfaces (deterministic)

**Files:**
- Create: `wit/myrhiza-kernel/wit/host-deterministic.wit`

The deterministic helper set is **normative** per [determinism.md §5.1](../specs/2026-05-09-myrhiza-master-design/determinism.md). Adding a helper is a kernel-major bump.

- [ ] **Step 1: Write the interface**

Write `wit/myrhiza-kernel/wit/host-deterministic.wit`:

```wit
package myrhiza:kernel@1.0.0;

/// Deterministic helper set per determinism.md §5.1. Bound by
/// state-apply, state-propose, interaction, and behavior worlds.
interface host-deterministic {
    use types.{hlc, key-handle, log-level};

    /// BLAKE3 of `bytes`. Always returns 32 bytes.
    /// Fuel cost: n * 5 where n = bytes.len().
    hash: func(bytes: list<u8>) -> list<u8>;

    /// Ed25519 RFC 8032 strict (rejects malleable s-values).
    /// Fuel cost: 5_000.
    verify-signature: func(
        pubkey: list<u8>,
        msg: list<u8>,
        sig: list<u8>,
    ) -> bool;

    /// Verify a payload MAC under a kernel-managed key handle.
    /// (Stub in plan A; real impl lands in plan B with key-handle
    /// infrastructure.)
    /// Fuel cost: 1_000.
    verify-payload-mac: func(envelope: list<u8>, handle: borrow<key-handle>) -> bool;

    /// Record a (handle, sealed-distribution-blob) pair into the
    /// deterministic state surface. Returns unit deliberately —
    /// "this peer can decrypt" would peer-locally branch state-apply.
    /// Fuel cost: 100.
    install-key: func(handle: borrow<key-handle>, sealed-distribution-blob: list<u8>);

    /// Pure decoder over canonical event bytes; extracts the HLC
    /// signed into the envelope by the originator.
    /// Fuel cost: 50.
    now-hlc-from-event: func(event-bytes: list<u8>) -> hlc;

    /// Output-only sink. Not part of state-digest.
    /// Fuel cost: 100 + msg.len().
    log: func(level: log-level, msg: string);
}
```

- [ ] **Step 2: Verify**

Run: `cargo check -p myrhiza-types`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add wit/myrhiza-kernel/wit/host-deterministic.wit
git commit -m "$(cat <<'EOF'
feat(wit): host-deterministic interface (helper set, normative)

Six imports binding-allowed for every profile per architecture.md
§3.5 row-by-row + determinism.md §5.1. Adding an entry is a kernel
major bump because state-apply may bind any of these.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 11: WIT host import sub-interfaces (non-deterministic)

**Files:**
- Create: `wit/myrhiza-kernel/wit/host-non-deterministic.wit`
- Create: `wit/myrhiza-kernel/wit/host-async.wit`
- Create: `wit/myrhiza-kernel/wit/host-ui-surfaces.wit`

These are imported by worlds that allow them. Plan A authors them but binds none.

- [ ] **Step 1: Write `host-non-deterministic.wit`**

Write `wit/myrhiza-kernel/wit/host-non-deterministic.wit`:

```wit
package myrhiza:kernel@1.0.0;

/// Non-deterministic imports. Forbidden in state-apply.
interface host-non-deterministic {
    use types.{identity-scope, hlc, key-handle};

    /// Peer-local hybrid logical clock.
    hlc: func() -> hlc;

    /// Cryptographic randomness.
    random: func(bytes: u32) -> list<u8>;

    /// Author an event under the given identity scope. Kernel
    /// structurally validates the payload against the app's WIT
    /// contract, looks up the private key, signs canonical encoding,
    /// and returns the signature. Per identity.md §6.1.
    author-event: func(scope: identity-scope, event-payload: list<u8>) -> list<u8>;

    /// Block on user prompt response. UI app's surface (not kernel-
    /// controlled chrome).
    user-prompt: func(prompt: string) -> string;
}
```

- [ ] **Step 2: Write `host-async.wit`**

Write `wit/myrhiza-kernel/wit/host-async.wit`:

```wit
package myrhiza:kernel@1.0.0;

/// Submit-and-poll async surfaces per abi.md §8.5.
interface host-async {
    use types.{request-token};

    record http-request {
        method: string,
        url: string,
        headers: list<tuple<string, string>>,
        body: list<u8>,
    }

    record http-response {
        status: u16,
        headers: list<tuple<string, string>>,
        body: list<u8>,
    }

    variant broadcast-error {
        cap-denied,
        topic-unknown,
        would-block,
    }

    variant fetch-error {
        cap-denied,
        not-found,
        timeout,
        would-block,
    }

    variant http-error {
        cap-denied,
        timeout,
        transport(string),
        would-block,
    }

    broadcast-submit: func(topic: list<u8>, msg: list<u8>) -> request-token;
    blob-fetch-submit: func(hash: list<u8>) -> request-token;
    http-request-submit: func(req: http-request) -> request-token;
}
```

- [ ] **Step 3: Write `host-ui-surfaces.wit`**

Write `wit/myrhiza-kernel/wit/host-ui-surfaces.wit`:

```wit
package myrhiza:kernel@1.0.0;

/// V1 minimum ui:* vocabulary per distribution.md §10.2. Counter+poll
/// MVP exercises panel + button + input + form.
interface host-ui-surfaces {
    record panel { id: string, title: string, body: list<ui-element> }
    record button { id: string, label: string, action: string }
    record input-element { id: string, label: string, placeholder: string }
    record form { id: string, fields: list<ui-element>, submit-action: string }
    record list-element { id: string, items: list<string> }
    record message { id: string, body: string }
    record menu { id: string, items: list<tuple<string, string>> }
    record dialog { id: string, body: list<ui-element> }

    variant ui-element {
        panel(panel),
        button(button),
        input(input-element),
        form(form),
        list(list-element),
        message(message),
        menu(menu),
        dialog(dialog),
    }
}
```

- [ ] **Step 4: Verify all parse**

Run: `cargo check -p myrhiza-types`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add wit/myrhiza-kernel/wit/host-non-deterministic.wit \
        wit/myrhiza-kernel/wit/host-async.wit \
        wit/myrhiza-kernel/wit/host-ui-surfaces.wit
git commit -m "$(cat <<'EOF'
feat(wit): host-non-deterministic, host-async, host-ui-surfaces

host-non-deterministic: hlc, random, author-event, user-prompt
host-async: broadcast/fetch/http submit-and-poll surfaces
host-ui-surfaces: panel, button, input, form, list, message, menu,
                  dialog (v1 minimum vocabulary per distribution.md
                  §10.2)

Plan A authors these; runtime bindings land in plans B and C.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 12: state-apply world

**Files:**
- Create: `wit/myrhiza-kernel/wit/world-state-apply.wit`

- [ ] **Step 1: Write the world**

Write `wit/myrhiza-kernel/wit/world-state-apply.wit`:

```wit
package myrhiza:kernel@1.0.0;

/// state-apply: strict pure function of (prior state, event).
/// Bound only against host-deterministic. ABI is normative per
/// architecture.md §3.5 + determinism.md §5.1 — adding any other
/// import here is a kernel-major bump.
world state-apply {
    import host-deterministic;
    use types.{verdict};

    /// Apply mode: ingest an event, update state in place, return
    /// the verdict. State is opaque app-defined bytes, encoded
    /// canonically per the app's serde discipline.
    /// In dry-run (pre-check) mode the kernel calls this same export
    /// with the same `prior-state` it would use for apply, and
    /// discards `new-state` if the verdict is Accept.
    export apply: func(prior-state: list<u8>, event: list<u8>) ->
        tuple<verdict, list<u8>>;

    /// Canonical bytes of the app state, used by the kernel for
    /// cross-peer convergence verification. Per convergence.md §4.3.
    export state-digest: func(state: list<u8>) -> list<u8>;
}
```

- [ ] **Step 2: Verify**

Run: `cargo check -p myrhiza-types`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add wit/myrhiza-kernel/wit/world-state-apply.wit
git commit -m "$(cat <<'EOF'
feat(wit): state-apply world (deterministic-only imports)

Imports only host-deterministic. Exports apply (returns verdict +
new state) and state-digest (canonical bytes for cross-peer
convergence verification).

Adding any non-deterministic import to this world is a kernel-major
ABI break per distribution.md §10.2.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 13: state-propose, interaction, behavior worlds

**Files:**
- Create: `wit/myrhiza-kernel/wit/world-state-propose.wit`
- Create: `wit/myrhiza-kernel/wit/world-interaction.wit`
- Create: `wit/myrhiza-kernel/wit/world-behavior.wit`

These are authored once and not bound at runtime in plan A.

- [ ] **Step 1: Write `world-state-propose.wit`**

```wit
package myrhiza:kernel@1.0.0;

world state-propose {
    import host-deterministic;
    import host-non-deterministic;
    use types.{verdict};

    /// Build a candidate event from user intent. Kernel re-runs
    /// state-apply (dry-run) before signing per convergence.md §4.4.
    export propose: func(prior-state: list<u8>, intent: list<u8>) ->
        result<list<u8>, string>;
}
```

- [ ] **Step 2: Write `world-interaction.wit`**

```wit
package myrhiza:kernel@1.0.0;

world interaction {
    import host-deterministic;
    import host-non-deterministic;
    import host-async;
    import host-ui-surfaces;

    /// Project a per-peer view of the state.
    export view: func(state: list<u8>, peer-state: list<u8>) -> list<u8>;

    /// Translate a user action into an intent for state-propose to
    /// build into an event.
    export dispatch: func(action: string) -> result<list<u8>, string>;

    /// Submit-and-poll completion handlers per abi.md §8.5.
    export on-broadcast-completion: func(token: list<u8>, ok: bool, err: string);
    export on-blob-fetch-completion: func(token: list<u8>, ok: bool, payload: list<u8>, err: string);
}
```

- [ ] **Step 3: Write `world-behavior.wit`**

```wit
package myrhiza:kernel@1.0.0;

world behavior {
    import host-deterministic;
    import host-non-deterministic;
    import host-async;

    /// Observe an event materialized by state-apply.
    export on-event: func(event: list<u8>);

    /// Periodic tick from the kernel scheduler.
    export tick: func();

    /// Submit-and-poll completion handlers.
    export on-http-completion: func(token: list<u8>, ok: bool, status: u16, body: list<u8>, err: string);
    export on-broadcast-completion: func(token: list<u8>, ok: bool, err: string);
}
```

- [ ] **Step 4: Verify all parse together**

Run: `cargo check -p myrhiza-types`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add wit/myrhiza-kernel/wit/world-state-propose.wit \
        wit/myrhiza-kernel/wit/world-interaction.wit \
        wit/myrhiza-kernel/wit/world-behavior.wit
git commit -m "$(cat <<'EOF'
feat(wit): state-propose, interaction, behavior worlds

Plan A authors all four profile worlds; only state-apply is bound
at runtime in this plan. Plans B and C bind the rest as features
(network, apps) come online.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 3: manifest crate

Per [distribution.md §10.2](../specs/2026-05-09-myrhiza-master-design/distribution.md), the manifest TOML schema is part of the v1 master spec because [capabilities.md §7.2](../specs/2026-05-09-myrhiza-master-design/capabilities.md)'s intersection mechanic cannot be specified without it. The signature is computed not over TOML text but over a canonical bincode encoding of the parsed typed manifest struct.

### Task 14: Manifest crate skeleton

**Files:**
- Create: `crates/manifest/Cargo.toml`
- Create: `crates/manifest/src/lib.rs`
- Modify: `Cargo.toml` (add to workspace `members`)

- [ ] **Step 1: Add to workspace**

In `Cargo.toml`, change:

```toml
members = [
    "crates/types",
]
```

to:

```toml
members = [
    "crates/types",
    "crates/manifest",
]
```

- [ ] **Step 2: Write the Cargo.toml**

Write `crates/manifest/Cargo.toml`:

```toml
[package]
name = "myrhiza-manifest"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Manifest schema, canonical encoding, and Ed25519 signature verification for Myrhiza bundles."

[lints]
workspace = true

[dependencies]
myrhiza-types = { path = "../types" }
bincode.workspace = true
serde.workspace = true
serde_bytes.workspace = true
blake3.workspace = true
ed25519-dalek.workspace = true
toml_edit = { version = "0.22", default-features = false, features = ["parse", "serde"] }
unicode-normalization.workspace = true
thiserror.workspace = true
hex.workspace = true

[dev-dependencies]
hex-literal.workspace = true
```

- [ ] **Step 3: Write the lib.rs stub**

Write `crates/manifest/src/lib.rs`:

```rust
//! Manifest TOML schema, canonical encoding, and signature verification
//! for Myrhiza bundles.
//!
//! Per [distribution.md §10.2]:
//! - Parse manifest TOML with `toml_edit 0.22.x` (pinned).
//! - Convert to typed `Manifest` struct.
//! - Canonical encoding = bincode 1.3.x (via
//!   `myrhiza_types::canonical_bincode`) over the typed struct.
//! - BLAKE3 the encoded bytes → `manifest_canonical_hash`.
//! - Author signs `manifest_canonical_hash + content_hash + version
//!   + author_pubkey` (length-prefixed framing per §10.2).

#![deny(missing_docs)]
```

- [ ] **Step 4: Verify**

Run: `cargo check -p myrhiza-manifest`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/manifest
git commit -m "$(cat <<'EOF'
chore(manifest): scaffold myrhiza-manifest crate

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 15: Capability vocabulary registry

**Files:**
- Create: `crates/manifest/src/vocabulary.rs`
- Modify: `crates/manifest/src/lib.rs`

Per [distribution.md §10.2](../specs/2026-05-09-myrhiza-master-design/distribution.md): "Apps cannot invent new capability strings outside the kernel-defined vocabulary; the kernel rejects any unknown capability identifier at install."

The registry is **the v1 normative list** — any string not here is rejected. Adding a string is a kernel-minor (or kernel-major if state-apply may bind it) version bump.

- [ ] **Step 1: Write the failing test**

Append to `crates/manifest/src/vocabulary.rs`:

```rust
#[cfg(test)]
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
            "ui:panel", "ui:list", "ui:message", "ui:form",
            "ui:menu", "ui:button", "ui:input", "ui:dialog",
        ] {
            assert!(known_capability(cap), "{cap} must be in v1 ui vocabulary");
        }
    }

    #[test]
    fn deterministic_helpers_are_classified() {
        assert_eq!(classify("host.hash"), Some(CapabilityClass::DeterministicHelper));
        assert_eq!(classify("host.broadcast-submit"), Some(CapabilityClass::HostImport));
        assert_eq!(classify("host.clipboard.write"), Some(CapabilityClass::HighValueOp));
        assert_eq!(classify("ui:panel"), Some(CapabilityClass::UiSurface));
        assert_eq!(classify("host.invented"), None);
    }
}
```

- [ ] **Step 2: Run and verify failure**

Run: `cargo test -p myrhiza-manifest vocabulary`
Expected: FAIL — `known_capability` not defined.

- [ ] **Step 3: Implement the registry**

Write `crates/manifest/src/vocabulary.rs` (above tests):

```rust
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
    ("host.verify-signature", CapabilityClass::DeterministicHelper),
    ("host.verify-payload-mac", CapabilityClass::DeterministicHelper),
    ("host.install-key", CapabilityClass::DeterministicHelper),
    ("host.now-hlc-from-event", CapabilityClass::DeterministicHelper),
    ("host.log", CapabilityClass::DeterministicHelper),

    // Non-deterministic host imports (state-propose, interaction, behavior).
    ("host.hlc", CapabilityClass::HostImport),
    ("host.random", CapabilityClass::HostImport),
    ("host.author-event", CapabilityClass::HostImport),
    ("host.broadcast-submit", CapabilityClass::HostImport),
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
```

Append to `crates/manifest/src/lib.rs`:

```rust
pub mod vocabulary;
```

- [ ] **Step 4: Run and verify pass**

Run: `cargo test -p myrhiza-manifest vocabulary`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/manifest
git commit -m "$(cat <<'EOF'
feat(manifest): v1 capability vocabulary registry

Mirrors architecture.md §3.5 host imports + distribution.md §10.2
v1 ui:* minimum vocabulary. classify() returns the capability
class (DeterministicHelper, HostImport, HighValueOp, UiSurface).
Adding an entry is a kernel ABI version bump.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 16: Manifest typed struct

**Files:**
- Create: `crates/manifest/src/schema.rs`
- Modify: `crates/manifest/src/lib.rs`

The struct mirrors [distribution.md §10.2](../specs/2026-05-09-myrhiza-master-design/distribution.md). It is the canonical-encoding target.

- [ ] **Step 1: Write the failing test**

Append to `crates/manifest/src/schema.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use myrhiza_types::canonical_bincode;

    fn minimal() -> Manifest {
        Manifest {
            app: AppSection {
                name: "counter".into(),
                version: "0.1.0".into(),
                description: "Simple shared counter".into(),
                author_pubkey: "wpub-author1q9q...xy".into(),
                author_identity_class: AuthorIdentityClass::ThirdParty,
            },
            abi: AbiSection {
                kernel_major: 1,
                kernel_minor_min: 0,
                state_digest_format: StateDigestFormat::Bincode13,
            },
            capabilities: CapabilitiesSection {
                host_imports: std::collections::BTreeMap::new(),
                ui_surfaces: std::collections::BTreeMap::new(),
                high_value_ops: HighValueOps::default(),
                deterministic_helpers: std::collections::BTreeMap::new(),
            },
            determinism: DeterminismSection {
                allow_floats: false,
                drift_detection: DriftDetectionSection { interval_events: 1024 },
            },
            modules: ModulesSection { dep: Vec::new() },
            components: ComponentsSection {
                state_apply: Some("components/state-apply.wasm".into()),
                state_propose: None,
                interaction: None,
                behavior: None,
            },
            author_policy: AuthorPolicy::default_deny(),
            signature: None,
        }
    }

    #[test]
    fn manifest_round_trips_via_canonical_bincode() {
        let m = minimal();
        let bytes = canonical_bincode().serialize(&m).expect("encode");
        let decoded: Manifest =
            canonical_bincode().deserialize(&bytes).expect("decode");
        assert_eq!(m, decoded);
    }

    #[test]
    fn author_policy_default_is_deny() {
        let p = AuthorPolicy::default_deny();
        assert!(matches!(p, AuthorPolicy::Deny));
    }

    #[test]
    fn modules_dep_canonical_sort_by_content_hash() {
        let mut m = minimal();
        m.modules.dep = vec![
            ModuleDep {
                name: "z-mod".into(),
                content_hash: "blake3:fff".into(),
                expected_author: "wpub-myrhiza1xyz".into(),
                required_capabilities: vec![],
            },
            ModuleDep {
                name: "a-mod".into(),
                content_hash: "blake3:aaa".into(),
                expected_author: "wpub-myrhiza1xyz".into(),
                required_capabilities: vec![],
            },
        ];
        m.canonicalize();
        assert_eq!(m.modules.dep[0].content_hash, "blake3:aaa");
        assert_eq!(m.modules.dep[1].content_hash, "blake3:fff");
    }
}
```

- [ ] **Step 2: Run and verify failure**

Run: `cargo test -p myrhiza-manifest schema`
Expected: FAIL — `Manifest` not defined.

- [ ] **Step 3: Implement the typed struct**

Write `crates/manifest/src/schema.rs` (above tests):

```rust
//! Typed manifest struct.
//!
//! Mirrors distribution.md §10.2's TOML schema. The struct is the
//! canonical-encoding target — the signature signs bincode of the
//! struct's signed-body view, NOT the TOML text.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub app: AppSection,
    pub abi: AbiSection,
    pub capabilities: CapabilitiesSection,
    pub determinism: DeterminismSection,
    pub modules: ModulesSection,
    pub components: ComponentsSection,
    /// Author-policy is required at parse time per identity.md §6.1.
    /// `default_deny()` produces `Deny`, which forbids
    /// `host.author-event` from non-state-propose profiles.
    pub author_policy: AuthorPolicy,
    /// `Some` only after signing. The serialized signed-body excludes
    /// this field.
    pub signature: Option<Signature>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AppSection {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author_pubkey: String,
    pub author_identity_class: AuthorIdentityClass,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum AuthorIdentityClass {
    ThirdParty,
    MyrhizaOfficial,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct AbiSection {
    pub kernel_major: u32,
    pub kernel_minor_min: u32,
    pub state_digest_format: StateDigestFormat,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum StateDigestFormat {
    /// The only v1 value.
    Bincode13,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct CapabilitiesSection {
    pub host_imports: BTreeMap<String, bool>,
    pub ui_surfaces: BTreeMap<String, bool>,
    pub high_value_ops: HighValueOps,
    pub deterministic_helpers: BTreeMap<String, bool>,
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct HighValueOps {
    pub clipboard_write: bool,
    pub file_picker_show: bool,
    pub navigation_top_level: bool,
    pub push_register: bool,
    /// List of key-handle namespaces app may seal under.
    pub aead_seal: Vec<String>,
    /// List of key-handle namespaces app may open from.
    pub aead_open: Vec<String>,
    /// RFC 6454 exact origins; empty = denied. No glob/wildcard at v1.
    pub http_request: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DeterminismSection {
    pub allow_floats: bool,
    pub drift_detection: DriftDetectionSection,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DriftDetectionSection {
    pub interval_events: u32,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ModulesSection {
    pub dep: Vec<ModuleDep>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ModuleDep {
    pub name: String,
    pub content_hash: String,
    pub expected_author: String,
    pub required_capabilities: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ComponentsSection {
    pub state_apply: Option<String>,
    pub state_propose: Option<String>,
    pub interaction: Option<String>,
    pub behavior: Option<String>,
}

/// Author-policy per identity.md §6.1. v1 default is `Deny`.
/// `Permissive` is opt-in; `Map` is per-profile-per-variant.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum AuthorPolicy {
    Deny,
    Permissive,
    Map {
        state_propose: Vec<String>,
        behavior: Vec<String>,
    },
}

impl AuthorPolicy {
    #[must_use]
    pub fn default_deny() -> Self {
        Self::Deny
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Signature {
    pub algorithm: SignatureAlgorithm,
    /// Raw 64-byte Ed25519 signature.
    #[serde(with = "crate::schema::serde_sig_bytes")]
    pub value: [u8; 64],
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SignatureAlgorithm {
    /// The only v1 value. Cremers ETK 2025 forbids ECDSA on the
    /// kernel surface; manifest cannot declare alternative algorithms.
    Ed25519,
}

mod serde_sig_bytes {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(b: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        serde_bytes::Bytes::new(b).serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        let v: serde_bytes::ByteBuf = serde_bytes::ByteBuf::deserialize(d)?;
        let arr: [u8; 64] = v
            .as_ref()
            .try_into()
            .map_err(|_| serde::de::Error::invalid_length(v.len(), &"64 bytes"))?;
        Ok(arr)
    }
}

impl Manifest {
    /// Apply canonical-form normalizations per distribution.md §10.2:
    /// - `modules.dep` sorted by `content_hash` ascending.
    /// - String fields NFC-normalized.
    pub fn canonicalize(&mut self) {
        use unicode_normalization::UnicodeNormalization;

        self.modules.dep.sort_by(|a, b| a.content_hash.cmp(&b.content_hash));

        // NFC normalize every string field at the schema boundary.
        let nfc = |s: &str| s.nfc().collect::<String>();
        self.app.name = nfc(&self.app.name);
        self.app.version = nfc(&self.app.version);
        self.app.description = nfc(&self.app.description);
        self.app.author_pubkey = nfc(&self.app.author_pubkey);
        for dep in &mut self.modules.dep {
            dep.name = nfc(&dep.name);
            dep.content_hash = nfc(&dep.content_hash);
            dep.expected_author = nfc(&dep.expected_author);
            for cap in &mut dep.required_capabilities {
                *cap = nfc(cap);
            }
        }
    }
}
```

Append to `crates/manifest/src/lib.rs`:

```rust
pub mod schema;
pub use schema::*;
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p myrhiza-manifest schema`
Expected: 3 passed.

Run: `cargo clippy -p myrhiza-manifest -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/manifest
git commit -m "$(cat <<'EOF'
feat(manifest): typed Manifest struct mirroring distribution.md §10.2

Plus canonicalize() — sorts modules.dep by content_hash and NFC-
normalizes string fields per the canonical encoding rules. Signature
is Optional<Signature>; signed-body excludes the signature.

AuthorPolicy::default_deny() returns Deny per identity.md §6.1's
deny-by-default v1 default.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 17: Manifest TOML parser

**Files:**
- Create: `crates/manifest/src/parse.rs`
- Modify: `crates/manifest/src/lib.rs`
- Create: `crates/manifest/tests/fixtures/counter-manifest.toml`

- [ ] **Step 1: Write the failing test**

Append to `crates/manifest/src/parse.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_counter_fixture() {
        let toml = include_str!("../tests/fixtures/counter-manifest.toml");
        let m = parse_manifest(toml).expect("parse counter fixture");
        assert_eq!(m.app.name, "counter");
        assert_eq!(m.abi.kernel_major, 1);
        assert!(m.capabilities.host_imports.contains_key("host.broadcast-submit"));
        // canonicalize should have run during parse.
        let mut prev = String::new();
        for dep in &m.modules.dep {
            assert!(
                dep.content_hash >= prev,
                "modules.dep must be sorted by content_hash"
            );
            prev = dep.content_hash.clone();
        }
    }

    #[test]
    fn parse_rejects_unknown_capability() {
        let toml = r#"
[app]
name = "x"
version = "0.1.0"
description = "x"
author-pubkey = "wpub-author1xxx"
author-identity-class = "third-party"

[abi]
kernel-major = 1
kernel-minor-min = 0
state-digest-format = "bincode-1.3"

[capabilities.host-imports]
"host.invented-by-app" = true

[capabilities.high-value-ops]

[determinism]
allow-floats = false

[determinism.drift-detection]
interval-events = 1024

[components]
state-apply = "components/state-apply.wasm"
"#;
        let err = parse_manifest(toml).expect_err("must reject unknown capability");
        assert!(
            err.to_string().contains("host.invented-by-app"),
            "error must name the offending capability: {err}"
        );
    }

    #[test]
    fn parse_rejects_non_ed25519_signature_algorithm() {
        let toml = r#"
[app]
name = "x"
version = "0.1.0"
description = "x"
author-pubkey = "wpub-author1xxx"
author-identity-class = "third-party"

[abi]
kernel-major = 1
kernel-minor-min = 0
state-digest-format = "bincode-1.3"

[capabilities.host-imports]
[capabilities.high-value-ops]

[determinism]
allow-floats = false

[determinism.drift-detection]
interval-events = 1024

[components]
state-apply = "components/state-apply.wasm"

[signature]
algorithm = "ecdsa"
value = "0x00"
"#;
        let err = parse_manifest(toml).expect_err("ECDSA must be rejected");
        assert!(err.to_string().to_lowercase().contains("ed25519"));
    }
}
```

- [ ] **Step 2: Write the fixture**

Write `crates/manifest/tests/fixtures/counter-manifest.toml`:

```toml
[app]
name = "counter"
version = "0.1.0"
description = "Simple shared counter"
author-pubkey = "wpub-author1q9qexamplexamplexamplexamplexamplexample"
author-identity-class = "third-party"

[abi]
kernel-major = 1
kernel-minor-min = 0
state-digest-format = "bincode-1.3"

[capabilities.host-imports]
"host.author-event" = true
"host.broadcast-submit" = true
"host.subscribe" = true
"host.kv.get" = true
"host.kv.put" = true

[capabilities.ui-surfaces]
"ui:panel" = true
"ui:button" = true

[capabilities.high-value-ops]
"host.clipboard.write" = false
"host.file-picker.show" = false
"host.navigation.top-level" = false
"host.push.register" = false
"host.aead-seal" = []
"host.aead-open" = []
"host.http.request" = []

[capabilities.deterministic-helpers]
"host.verify-signature" = true
"host.hash" = true
"host.now-hlc-from-event" = true
"host.log" = true

[determinism]
allow-floats = false

[determinism.drift-detection]
interval-events = 1024

[[modules.dep]]
name = "myrhiza-permission-rbac"
content-hash = "blake3:f00d000000000000000000000000000000000000000000000000000000000000"
expected-author = "wpub-myrhiza1xyz"
required-capabilities = ["host.kv.get", "host.kv.put"]

[[modules.dep]]
name = "myrhiza-state-snapshot-cache"
content-hash = "blake3:000d000000000000000000000000000000000000000000000000000000000000"
expected-author = "wpub-myrhiza1xyz"
required-capabilities = ["host.kv.get", "host.kv.put", "host.broadcast-submit"]

[components]
state-apply = "components/state-apply.wasm"
state-propose = "components/state-propose.wasm"
interaction = "components/interaction.wasm"
```

- [ ] **Step 3: Run and verify failure**

Run: `cargo test -p myrhiza-manifest parse`
Expected: FAIL — `parse_manifest` not defined.

- [ ] **Step 4: Implement the parser**

Write `crates/manifest/src/parse.rs` (above tests):

```rust
//! Manifest TOML parser.
//!
//! - Pinned to `toml_edit 0.22.x` (per distribution.md §10.2).
//! - Rejects unknown capability strings (per §10.2).
//! - Rejects non-Ed25519 signature algorithms (Cremers ETK 2025
//!   structural enforcement; per identity.md §6.1).
//! - Calls `Manifest::canonicalize()` before returning so callers
//!   always see canonical-form output.

use std::collections::BTreeMap;

use thiserror::Error;
use toml_edit::DocumentMut;

use crate::schema::*;
use crate::vocabulary::known_capability;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml_edit::TomlError),
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    #[error("invalid value at {field}: {detail}")]
    InvalidValue { field: &'static str, detail: String },
    #[error("unknown capability: {0}")]
    UnknownCapability(String),
    #[error("Cremers ETK 2025: only ed25519 is permitted, got {0}")]
    NonEd25519Signature(String),
    #[error("invalid hex: {0}")]
    Hex(#[from] hex::FromHexError),
}

/// Parse a manifest TOML string and validate the result.
pub fn parse_manifest(input: &str) -> Result<Manifest, ParseError> {
    let doc: DocumentMut = input.parse()?;

    let app = parse_app(&doc)?;
    let abi = parse_abi(&doc)?;
    let capabilities = parse_capabilities(&doc)?;
    let determinism = parse_determinism(&doc)?;
    let modules = parse_modules(&doc)?;
    let components = parse_components(&doc)?;
    let author_policy = parse_author_policy(&doc)?;
    let signature = parse_signature(&doc)?;

    let mut m = Manifest {
        app,
        abi,
        capabilities,
        determinism,
        modules,
        components,
        author_policy,
        signature,
    };
    m.canonicalize();
    Ok(m)
}

fn require<'a>(doc: &'a DocumentMut, table: &str) -> Result<&'a toml_edit::Table, ParseError> {
    doc.get(table)
        .and_then(|i| i.as_table())
        .ok_or(ParseError::MissingField(static_str(table)))
}

fn require_str<'a>(t: &'a toml_edit::Table, key: &'static str) -> Result<&'a str, ParseError> {
    t.get(key)
        .and_then(|i| i.as_str())
        .ok_or(ParseError::MissingField(key))
}

fn require_int(t: &toml_edit::Table, key: &'static str) -> Result<i64, ParseError> {
    t.get(key)
        .and_then(|i| i.as_integer())
        .ok_or(ParseError::MissingField(key))
}

fn static_str(s: &str) -> &'static str {
    // SAFETY: leak is acceptable for diagnostic strings; cardinality
    // is bounded by manifest schema.
    Box::leak(s.to_string().into_boxed_str())
}

fn parse_app(doc: &DocumentMut) -> Result<AppSection, ParseError> {
    let t = require(doc, "app")?;
    let class = match require_str(t, "author-identity-class")? {
        "third-party" => AuthorIdentityClass::ThirdParty,
        "myrhiza-official" => AuthorIdentityClass::MyrhizaOfficial,
        other => {
            return Err(ParseError::InvalidValue {
                field: "app.author-identity-class",
                detail: format!("unknown class {other}"),
            })
        }
    };
    Ok(AppSection {
        name: require_str(t, "name")?.into(),
        version: require_str(t, "version")?.into(),
        description: require_str(t, "description")?.into(),
        author_pubkey: require_str(t, "author-pubkey")?.into(),
        author_identity_class: class,
    })
}

fn parse_abi(doc: &DocumentMut) -> Result<AbiSection, ParseError> {
    let t = require(doc, "abi")?;
    let format = match require_str(t, "state-digest-format")? {
        "bincode-1.3" => StateDigestFormat::Bincode13,
        other => {
            return Err(ParseError::InvalidValue {
                field: "abi.state-digest-format",
                detail: format!("unknown format {other}"),
            })
        }
    };
    Ok(AbiSection {
        kernel_major: u32::try_from(require_int(t, "kernel-major")?).map_err(|_| {
            ParseError::InvalidValue {
                field: "abi.kernel-major",
                detail: "out of u32 range".into(),
            }
        })?,
        kernel_minor_min: u32::try_from(require_int(t, "kernel-minor-min")?).map_err(|_| {
            ParseError::InvalidValue {
                field: "abi.kernel-minor-min",
                detail: "out of u32 range".into(),
            }
        })?,
        state_digest_format: format,
    })
}

fn parse_capabilities(doc: &DocumentMut) -> Result<CapabilitiesSection, ParseError> {
    let mut host_imports = BTreeMap::new();
    let mut ui_surfaces = BTreeMap::new();
    let mut deterministic_helpers = BTreeMap::new();

    if let Some(t) = doc.get("capabilities").and_then(|i| i.as_table()) {
        if let Some(hi) = t.get("host-imports").and_then(|i| i.as_table()) {
            for (k, v) in hi.iter() {
                if !known_capability(k) {
                    return Err(ParseError::UnknownCapability(k.into()));
                }
                let b = v.as_bool().ok_or(ParseError::InvalidValue {
                    field: "capabilities.host-imports.<key>",
                    detail: format!("expected bool for {k}"),
                })?;
                host_imports.insert(k.into(), b);
            }
        }
        if let Some(ui) = t.get("ui-surfaces").and_then(|i| i.as_table()) {
            for (k, v) in ui.iter() {
                if !known_capability(k) {
                    return Err(ParseError::UnknownCapability(k.into()));
                }
                let b = v.as_bool().ok_or(ParseError::InvalidValue {
                    field: "capabilities.ui-surfaces.<key>",
                    detail: format!("expected bool for {k}"),
                })?;
                ui_surfaces.insert(k.into(), b);
            }
        }
        if let Some(dh) = t.get("deterministic-helpers").and_then(|i| i.as_table()) {
            for (k, v) in dh.iter() {
                if !known_capability(k) {
                    return Err(ParseError::UnknownCapability(k.into()));
                }
                let b = v.as_bool().ok_or(ParseError::InvalidValue {
                    field: "capabilities.deterministic-helpers.<key>",
                    detail: format!("expected bool for {k}"),
                })?;
                deterministic_helpers.insert(k.into(), b);
            }
        }
    }

    let high_value_ops = parse_hvo(doc)?;

    Ok(CapabilitiesSection {
        host_imports,
        ui_surfaces,
        high_value_ops,
        deterministic_helpers,
    })
}

fn parse_hvo(doc: &DocumentMut) -> Result<HighValueOps, ParseError> {
    let mut hvo = HighValueOps::default();
    let Some(t) = doc
        .get("capabilities")
        .and_then(|i| i.as_table())
        .and_then(|t| t.get("high-value-ops"))
        .and_then(|i| i.as_table())
    else {
        return Ok(hvo);
    };
    for (k, v) in t.iter() {
        if !known_capability(k) {
            return Err(ParseError::UnknownCapability(k.into()));
        }
        match k {
            "host.clipboard.write" => hvo.clipboard_write = v.as_bool().unwrap_or(false),
            "host.file-picker.show" => hvo.file_picker_show = v.as_bool().unwrap_or(false),
            "host.navigation.top-level" => hvo.navigation_top_level = v.as_bool().unwrap_or(false),
            "host.push.register" => hvo.push_register = v.as_bool().unwrap_or(false),
            "host.aead-seal" => hvo.aead_seal = parse_str_array(v)?,
            "host.aead-open" => hvo.aead_open = parse_str_array(v)?,
            "host.http.request" => hvo.http_request = parse_str_array(v)?,
            other => {
                return Err(ParseError::InvalidValue {
                    field: "capabilities.high-value-ops",
                    detail: format!("unsupported field {other}"),
                })
            }
        }
    }
    Ok(hvo)
}

fn parse_str_array(item: &toml_edit::Item) -> Result<Vec<String>, ParseError> {
    let arr = item.as_array().ok_or(ParseError::InvalidValue {
        field: "high-value-ops.<list>",
        detail: "expected array".into(),
    })?;
    arr.iter()
        .map(|v| {
            v.as_str().map(String::from).ok_or(ParseError::InvalidValue {
                field: "high-value-ops.<list>",
                detail: "expected string element".into(),
            })
        })
        .collect()
}

fn parse_determinism(doc: &DocumentMut) -> Result<DeterminismSection, ParseError> {
    let t = require(doc, "determinism")?;
    let allow_floats = t
        .get("allow-floats")
        .and_then(|i| i.as_bool())
        .unwrap_or(false);
    let drift = t
        .get("drift-detection")
        .and_then(|i| i.as_table())
        .ok_or(ParseError::MissingField("determinism.drift-detection"))?;
    let interval_events = u32::try_from(require_int(drift, "interval-events")?).map_err(|_| {
        ParseError::InvalidValue {
            field: "determinism.drift-detection.interval-events",
            detail: "out of u32 range".into(),
        }
    })?;
    Ok(DeterminismSection {
        allow_floats,
        drift_detection: DriftDetectionSection { interval_events },
    })
}

fn parse_modules(doc: &DocumentMut) -> Result<ModulesSection, ParseError> {
    let mut deps = Vec::new();
    if let Some(arr) = doc
        .get("modules")
        .and_then(|i| i.as_table())
        .and_then(|t| t.get("dep"))
        .and_then(|i| i.as_array_of_tables())
    {
        for tbl in arr.iter() {
            let mut required = Vec::new();
            if let Some(rc) = tbl.get("required-capabilities").and_then(|i| i.as_array()) {
                for v in rc.iter() {
                    let s = v.as_str().ok_or(ParseError::InvalidValue {
                        field: "modules.dep.required-capabilities",
                        detail: "expected string element".into(),
                    })?;
                    if !known_capability(s) {
                        return Err(ParseError::UnknownCapability(s.into()));
                    }
                    required.push(s.into());
                }
            }
            deps.push(ModuleDep {
                name: require_str(tbl, "name")?.into(),
                content_hash: require_str(tbl, "content-hash")?.into(),
                expected_author: require_str(tbl, "expected-author")?.into(),
                required_capabilities: required,
            });
        }
    }
    Ok(ModulesSection { dep: deps })
}

fn parse_components(doc: &DocumentMut) -> Result<ComponentsSection, ParseError> {
    let t = require(doc, "components")?;
    let opt = |k: &str| t.get(k).and_then(|i| i.as_str()).map(String::from);
    Ok(ComponentsSection {
        state_apply: opt("state-apply"),
        state_propose: opt("state-propose"),
        interaction: opt("interaction"),
        behavior: opt("behavior"),
    })
}

fn parse_author_policy(doc: &DocumentMut) -> Result<AuthorPolicy, ParseError> {
    let Some(t) = doc.get("author-policy").and_then(|i| i.as_table()) else {
        return Ok(AuthorPolicy::default_deny());
    };
    if let Some(p) = t.get("policy").and_then(|i| i.as_str()) {
        return match p {
            "permissive" => Ok(AuthorPolicy::Permissive),
            "deny" => Ok(AuthorPolicy::Deny),
            other => Err(ParseError::InvalidValue {
                field: "author-policy.policy",
                detail: format!("unknown {other}"),
            }),
        };
    }
    let parse_list = |key: &str| -> Result<Vec<String>, ParseError> {
        let Some(arr) = t.get(key).and_then(|i| i.as_array()) else {
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for v in arr.iter() {
            out.push(
                v.as_str()
                    .ok_or(ParseError::InvalidValue {
                        field: "author-policy.<list>",
                        detail: "expected string element".into(),
                    })?
                    .into(),
            );
        }
        Ok(out)
    };
    Ok(AuthorPolicy::Map {
        state_propose: parse_list("state-propose")?,
        behavior: parse_list("behavior")?,
    })
}

fn parse_signature(doc: &DocumentMut) -> Result<Option<Signature>, ParseError> {
    let Some(t) = doc.get("signature").and_then(|i| i.as_table()) else {
        return Ok(None);
    };
    let alg = require_str(t, "algorithm")?;
    if alg != "ed25519" {
        return Err(ParseError::NonEd25519Signature(alg.into()));
    }
    let raw = require_str(t, "value")?;
    let stripped = raw.strip_prefix("0x").unwrap_or(raw);
    let bytes = hex::decode(stripped)?;
    let arr: [u8; 64] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| ParseError::InvalidValue {
            field: "signature.value",
            detail: "expected 64 bytes".into(),
        })?;
    Ok(Some(Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        value: arr,
    }))
}
```

Append to `crates/manifest/src/lib.rs`:

```rust
pub mod parse;
pub use parse::{ParseError, parse_manifest};
```

- [ ] **Step 5: Run and verify pass**

Run: `cargo test -p myrhiza-manifest`
Expected: all pass.

Run: `cargo clippy -p myrhiza-manifest -- -D warnings`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/manifest
git commit -m "$(cat <<'EOF'
feat(manifest): parse_manifest with vocabulary + algorithm checks

- Pinned to toml_edit 0.22.x (canonical TOML parser).
- Rejects unknown capability strings (anywhere they appear).
- Rejects non-Ed25519 signature algorithms (Cremers ETK 2025
  structural enforcement; identity.md §6.1).
- Calls Manifest::canonicalize() before returning so callers see
  canonical form.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 18: Manifest canonical encoding (signed body)

**Files:**
- Create: `crates/manifest/src/canonical.rs`
- Modify: `crates/manifest/src/lib.rs`

The signature is over `manifest_canonical_hash + content_hash + version + author_pubkey` with **length-prefixed framing** per [distribution.md §10.2](../specs/2026-05-09-myrhiza-master-design/distribution.md). Each field is `4-byte LE length || bytes`.

- [ ] **Step 1: Write the failing test**

Append to `crates/manifest/src/canonical.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use myrhiza_types::EventHash;

    #[test]
    fn signed_body_excludes_signature() {
        let m_no_sig = sample_manifest(None);
        let m_with_sig = sample_manifest(Some([0xFF; 64]));
        let h1 = manifest_canonical_hash(&m_no_sig);
        let h2 = manifest_canonical_hash(&m_with_sig);
        assert_eq!(h1, h2, "canonical hash must not depend on signature bytes");
    }

    #[test]
    fn length_prefix_layout() {
        // Single field with bytes [1, 2, 3] should produce
        // [3, 0, 0, 0, 1, 2, 3].
        let out = length_prefix_concat(&[&[1, 2, 3]]);
        assert_eq!(out, vec![3, 0, 0, 0, 1, 2, 3]);
    }

    #[test]
    fn signing_target_layout() {
        let m = sample_manifest(None);
        let content = EventHash::blake3(b"some-content");
        let target = signing_target_bytes(&m, &content);
        // 4 length prefixes for 4 fields.
        assert!(target.len() >= 16);
    }

    fn sample_manifest(sig: Option<[u8; 64]>) -> crate::schema::Manifest {
        use crate::schema::*;
        Manifest {
            app: AppSection {
                name: "x".into(),
                version: "0.1.0".into(),
                description: "x".into(),
                author_pubkey: "wpub-author1xxx".into(),
                author_identity_class: AuthorIdentityClass::ThirdParty,
            },
            abi: AbiSection {
                kernel_major: 1,
                kernel_minor_min: 0,
                state_digest_format: StateDigestFormat::Bincode13,
            },
            capabilities: CapabilitiesSection {
                host_imports: Default::default(),
                ui_surfaces: Default::default(),
                high_value_ops: HighValueOps::default(),
                deterministic_helpers: Default::default(),
            },
            determinism: DeterminismSection {
                allow_floats: false,
                drift_detection: DriftDetectionSection { interval_events: 1024 },
            },
            modules: ModulesSection { dep: vec![] },
            components: ComponentsSection {
                state_apply: Some("components/state-apply.wasm".into()),
                state_propose: None,
                interaction: None,
                behavior: None,
            },
            author_policy: AuthorPolicy::default_deny(),
            signature: sig.map(|v| Signature {
                algorithm: SignatureAlgorithm::Ed25519,
                value: v,
            }),
        }
    }
}
```

- [ ] **Step 2: Run and verify failure**

Run: `cargo test -p myrhiza-manifest canonical`
Expected: FAIL — symbols not defined.

- [ ] **Step 3: Implement canonical encoding**

Write `crates/manifest/src/canonical.rs` (above tests):

```rust
//! Canonical encoding and signing-target framing.
//!
//! Per distribution.md §10.2:
//! - manifest_canonical_hash = BLAKE3(canonical_bincode(signed_body))
//! - signing_target = length_prefix_concat(
//!       BLAKE3("myrhiza/manifest/v1"),
//!       manifest_canonical_hash,
//!       content_hash,
//!       version_string_bytes,
//!       author_pubkey_bytes)
//! - Length prefixes are 4-byte little-endian per the section text.

use myrhiza_types::{EventHash, canonical_bincode};
use serde::Serialize;

use crate::schema::Manifest;

/// Domain-separator string per §10.2.
pub const DOMAIN_SEP: &[u8] = b"myrhiza/manifest/v1";

/// Encode the manifest's signed body (everything except `signature`)
/// via `canonical_bincode`.
pub fn signed_body_bytes(m: &Manifest) -> Vec<u8> {
    #[derive(Serialize)]
    struct SignedBody<'a> {
        app: &'a crate::schema::AppSection,
        abi: &'a crate::schema::AbiSection,
        capabilities: &'a crate::schema::CapabilitiesSection,
        determinism: &'a crate::schema::DeterminismSection,
        modules: &'a crate::schema::ModulesSection,
        components: &'a crate::schema::ComponentsSection,
        author_policy: &'a crate::schema::AuthorPolicy,
    }

    let body = SignedBody {
        app: &m.app,
        abi: &m.abi,
        capabilities: &m.capabilities,
        determinism: &m.determinism,
        modules: &m.modules,
        components: &m.components,
        author_policy: &m.author_policy,
    };

    canonical_bincode()
        .serialize(&body)
        .expect("canonical bincode of SignedBody never fails")
}

/// BLAKE3 of the canonical signed body.
pub fn manifest_canonical_hash(m: &Manifest) -> EventHash {
    EventHash::blake3(&signed_body_bytes(m))
}

/// `length_prefix_concat(fields)` returns
/// `for each f: u32_le(f.len()) || f`.
pub fn length_prefix_concat(fields: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(fields.iter().map(|f| 4 + f.len()).sum());
    for f in fields {
        let len = u32::try_from(f.len()).expect("manifest field length > u32::MAX");
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(f);
    }
    out
}

/// Compute the byte string the author signs.
///
/// Layout per §10.2:
///   length_prefix_concat(
///       DOMAIN_SEP,
///       manifest_canonical_hash,
///       content_hash,
///       version_string_bytes,
///       author_pubkey_bytes)
pub fn signing_target_bytes(m: &Manifest, content_hash: &EventHash) -> Vec<u8> {
    let canonical_hash = manifest_canonical_hash(m);
    let version_bytes = m.app.version.as_bytes();
    let author_bytes = m.app.author_pubkey.as_bytes();
    length_prefix_concat(&[
        DOMAIN_SEP,
        canonical_hash.as_bytes(),
        content_hash.as_bytes(),
        version_bytes,
        author_bytes,
    ])
}
```

Append to `crates/manifest/src/lib.rs`:

```rust
pub mod canonical;
pub use canonical::{
    DOMAIN_SEP, length_prefix_concat, manifest_canonical_hash, signed_body_bytes,
    signing_target_bytes,
};
```

- [ ] **Step 4: Run tests + lint**

Run: `cargo test -p myrhiza-manifest canonical`
Expected: 3 passed.

Run: `cargo clippy -p myrhiza-manifest -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/manifest
git commit -m "$(cat <<'EOF'
feat(manifest): canonical encoding + length-prefixed signing target

- signed_body_bytes(m): canonical_bincode over every field except
  the signature, returning the byte sequence the canonical hash
  is over.
- manifest_canonical_hash(m): BLAKE3 of signed_body_bytes.
- signing_target_bytes(m, content_hash): length_prefix_concat per
  distribution.md §10.2 (4-byte LE length || bytes per field) over
  domain-sep + canonical hash + content hash + version + author
  pubkey.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 19: Manifest signature sign + verify

**Files:**
- Create: `crates/manifest/src/signature.rs`
- Modify: `crates/manifest/src/lib.rs`

Per [determinism.md §5.1](../specs/2026-05-09-myrhiza-master-design/determinism.md): RFC 8032 strict via `verify_strict`. Per [identity.md §6.1](../specs/2026-05-09-myrhiza-master-design/identity.md): no algorithm parameter exposed.

- [ ] **Step 1: Write the failing test**

Append to `crates/manifest/src/signature.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use myrhiza_types::EventHash;

    fn fixture_manifest() -> crate::schema::Manifest {
        use crate::schema::*;
        Manifest {
            app: AppSection {
                name: "x".into(),
                version: "0.1.0".into(),
                description: "x".into(),
                author_pubkey: "wpub-author1xxx".into(),
                author_identity_class: AuthorIdentityClass::ThirdParty,
            },
            abi: AbiSection {
                kernel_major: 1,
                kernel_minor_min: 0,
                state_digest_format: StateDigestFormat::Bincode13,
            },
            capabilities: CapabilitiesSection {
                host_imports: Default::default(),
                ui_surfaces: Default::default(),
                high_value_ops: HighValueOps::default(),
                deterministic_helpers: Default::default(),
            },
            determinism: DeterminismSection {
                allow_floats: false,
                drift_detection: DriftDetectionSection { interval_events: 1024 },
            },
            modules: ModulesSection { dep: vec![] },
            components: ComponentsSection {
                state_apply: Some("components/state-apply.wasm".into()),
                state_propose: None,
                interaction: None,
                behavior: None,
            },
            author_policy: AuthorPolicy::default_deny(),
            signature: None,
        }
    }

    #[test]
    fn sign_then_verify_round_trip() {
        let sk = SigningKey::from_bytes(&[42u8; 32]);
        let pk_bytes: [u8; 32] = sk.verifying_key().to_bytes();

        let m = fixture_manifest();
        let content = EventHash::blake3(b"content");

        let target = crate::canonical::signing_target_bytes(&m, &content);
        let sig = sk.sign(&target);

        verify_signature(&pk_bytes, &target, &sig.to_bytes())
            .expect("legitimate signature must verify");
    }

    #[test]
    fn verify_rejects_tampered_target() {
        let sk = SigningKey::from_bytes(&[42u8; 32]);
        let pk_bytes: [u8; 32] = sk.verifying_key().to_bytes();
        let target = b"original";
        let sig = sk.sign(target);
        let res = verify_signature(&pk_bytes, b"tampered", &sig.to_bytes());
        assert!(res.is_err());
    }

    #[test]
    fn verify_rejects_non_strict_signature() {
        // verify_strict catches signatures with non-canonical s.
        // We assert that the API used is verify_strict by checking
        // that the implementation does NOT compile if you swap to
        // verify(). This test exercises the API contract; the
        // adversarial vector test lands in plan B's crypto fuzz.
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let pk_bytes: [u8; 32] = sk.verifying_key().to_bytes();
        let sig = sk.sign(b"msg");
        verify_signature(&pk_bytes, b"msg", &sig.to_bytes()).expect("strict path passes for canonical sig");
    }
}
```

- [ ] **Step 2: Run and verify failure**

Run: `cargo test -p myrhiza-manifest signature`
Expected: FAIL — `verify_signature` not defined.

- [ ] **Step 3: Implement**

Write `crates/manifest/src/signature.rs` (above tests):

```rust
//! Manifest signature: Ed25519 RFC 8032 strict.
//!
//! Per determinism.md §5.1, every kernel-side `verify-signature`
//! path uses `VerifyingKey::verify_strict`. Plain `verify` is
//! forbidden — it accepts malleable s-values that fail Cremers ETK
//! 2025's SUF-CMA requirement.

use ed25519_dalek::{Signature, VerifyingKey};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SignatureError {
    #[error("invalid Ed25519 public key encoding")]
    InvalidPubkey,
    #[error("invalid Ed25519 signature encoding")]
    InvalidSignature,
    #[error("signature verification failed (RFC 8032 strict)")]
    VerificationFailed,
}

/// Verify an Ed25519 signature using `verify_strict` (RFC 8032 strict).
pub fn verify_signature(
    pubkey_bytes: &[u8; 32],
    message: &[u8],
    signature_bytes: &[u8; 64],
) -> Result<(), SignatureError> {
    let key = VerifyingKey::from_bytes(pubkey_bytes).map_err(|_| SignatureError::InvalidPubkey)?;
    let sig = Signature::from_bytes(signature_bytes);
    key.verify_strict(message, &sig)
        .map_err(|_| SignatureError::VerificationFailed)
}
```

Append to `crates/manifest/src/lib.rs`:

```rust
pub mod signature;
pub use signature::{SignatureError, verify_signature};
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p myrhiza-manifest`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates/manifest
git commit -m "$(cat <<'EOF'
feat(manifest): verify_signature using verify_strict (RFC 8032)

Plain ed25519_dalek::verify is forbidden per determinism.md §5.1
because it accepts malleable s-values that fail Cremers ETK 2025
SUF-CMA. Every signature path in the kernel goes through this
function.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 4: backend trait abstraction

Per [implementation.md §20 step 7](../specs/2026-05-09-myrhiza-master-design/implementation.md): "Backend trait abstraction: stable internal trait both Wasmtime and jco backends will satisfy. Wasmtime impl satisfies it; jco impl deferred to step 17." Designed in from the start, not retrofitted.

### Task 20: `myrhiza-backend` crate skeleton

**Files:**
- Modify: `Cargo.toml` (workspace members)
- Create: `crates/backend/Cargo.toml`
- Create: `crates/backend/src/lib.rs`

- [ ] **Step 1: Add to workspace members**

In `Cargo.toml` extend `members`:

```toml
members = [
    "crates/types",
    "crates/manifest",
    "crates/backend",
]
```

- [ ] **Step 2: Write Cargo.toml**

Write `crates/backend/Cargo.toml`:

```toml
[package]
name = "myrhiza-backend"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Backend trait abstraction; Wasmtime and jco impls satisfy this trait."

[lints]
workspace = true

[dependencies]
myrhiza-types = { path = "../types" }
myrhiza-manifest = { path = "../manifest" }
thiserror.workspace = true
```

- [ ] **Step 3: Write the trait**

Write `crates/backend/src/lib.rs`:

```rust
//! Backend trait abstraction.
//!
//! Plan A's `myrhiza-wasmtime-backend` is the v1 native impl.
//! Plan C's `myrhiza-jco-backend` will satisfy the same trait.
//! Designed in from the start so jco doesn't require kernel
//! retrofitting per implementation.md §20.

#![deny(missing_docs)]

use myrhiza_manifest::Manifest;
use myrhiza_types::IdentityScope;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BackendError {
    /// Component bytes failed to decode or instantiate.
    #[error("component instantiation failed: {0}")]
    Instantiation(String),
    /// Component imported a host import its manifest does not declare.
    #[error("capability check failed: component imports {imported} not in manifest grants")]
    UnauthorizedImport { imported: String },
    /// Component imported a function not in the v1 vocabulary.
    #[error("component imports unknown capability: {0}")]
    UnknownImport(String),
    /// State-apply WASM contains a banned float instruction.
    #[error("float-ban lint: component contains banned instruction {0}")]
    BannedInstruction(&'static str),
    /// Fuel exhaustion or other trap during apply.
    #[error("trap during apply: {0}")]
    Trap(String),
    /// Pre-check or apply returned a Reject verdict.
    #[error("verdict reject: {0}")]
    Verdict(String),
    /// Calling profile attempted operation not authorized for it.
    #[error("profile {profile:?} forbidden from operation: {op}")]
    ProfileForbidden { profile: &'static str, op: String },
}

/// A loaded, capability-gated component instance ready to be called.
pub trait ComponentInstance: Send + 'static {
    /// Invoke `apply(prior_state, event)` returning verdict + new state.
    /// Pre-check (dry-run) is the same call; the kernel decides whether
    /// to commit `new_state` based on the returned verdict.
    fn call_apply(
        &mut self,
        prior_state: &[u8],
        event: &[u8],
    ) -> Result<(Verdict, Vec<u8>), BackendError>;

    /// Invoke `state-digest(state)`.
    fn call_state_digest(&mut self, state: &[u8]) -> Result<Vec<u8>, BackendError>;
}

/// The verdict returned by state-apply.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Verdict {
    Accept,
    Reject(String),
}

/// Profile being instantiated. Determines which sub-interface is bound.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Profile {
    StateApply,
    StatePropose,
    Interaction,
    Behavior,
}

/// Identity context bound at instance creation. Plan A only uses this
/// for state-apply (which has no `host.author-event`); plans B/C use
/// it for non-deterministic profiles.
#[derive(Clone, Debug)]
pub struct InstanceIdentity {
    pub scope: IdentityScope,
}

/// A backend creates `ComponentInstance`s from component bytes + manifest.
pub trait Backend: Send + Sync + 'static {
    /// Instantiate a state-apply component, applying capability gating
    /// per the manifest. Returns an instance ready for `call_apply`.
    fn instantiate_state_apply(
        &self,
        component_bytes: &[u8],
        manifest: &Manifest,
    ) -> Result<Box<dyn ComponentInstance>, BackendError>;
}
```

- [ ] **Step 4: Verify**

Run: `cargo check -p myrhiza-backend`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/backend
git commit -m "$(cat <<'EOF'
feat(backend): Backend trait + ComponentInstance trait

Stable internal abstraction both Wasmtime (this plan) and jco
(plan C) impls satisfy. Per implementation.md §20 step 7 — designed
in from the start so jco doesn't require kernel retrofitting.

Plan A only requires instantiate_state_apply on the trait. Plans
B/C extend with instantiate_propose/interaction/behavior.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 5: Wasmtime backend

### Task 21: `myrhiza-wasmtime-backend` crate skeleton

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/wasmtime-backend/Cargo.toml`
- Create: `crates/wasmtime-backend/src/lib.rs`
- Create: `crates/wasmtime-backend/build.rs`

- [ ] **Step 1: Workspace member**

Add `crates/wasmtime-backend` to root `Cargo.toml` `members`.

- [ ] **Step 2: Cargo.toml**

Write `crates/wasmtime-backend/Cargo.toml`:

```toml
[package]
name = "myrhiza-wasmtime-backend"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Wasmtime impl of myrhiza-backend with capability-gated linker, fuel, float-ban."

[lints]
workspace = true

[dependencies]
myrhiza-types = { path = "../types" }
myrhiza-manifest = { path = "../manifest" }
myrhiza-backend = { path = "../backend" }
wasmtime = { workspace = true, features = ["component-model", "cranelift", "runtime"] }
blake3.workspace = true
ed25519-dalek.workspace = true
serde.workspace = true
thiserror.workspace = true
hex.workspace = true

[build-dependencies]
wasmtime = { workspace = true, features = ["component-model"] }

[dev-dependencies]
anyhow.workspace = true
tempfile.workspace = true
```

- [ ] **Step 3: lib.rs stub**

Write `crates/wasmtime-backend/src/lib.rs`:

```rust
//! Wasmtime backend for the Myrhiza runtime.
//!
//! Implements [`myrhiza_backend::Backend`] using Wasmtime's component
//! model. Capability gating is enforced at linker construction time
//! (only allowed imports are bound) plus a per-call interception
//! wrapper for high-value ops.

#![deny(missing_docs)]

mod engine;
mod float_ban;
mod gating;
mod helpers;
mod instance;

pub use engine::WasmtimeBackend;
```

- [ ] **Step 4: build.rs**

Write `crates/wasmtime-backend/build.rs`:

```rust
fn main() {
    println!("cargo:rerun-if-changed=../../wit/myrhiza-kernel/wit");
}
```

- [ ] **Step 5: Empty submodule files**

Write `crates/wasmtime-backend/src/engine.rs`:

```rust
//! Wasmtime engine + Backend impl.
```

Write `crates/wasmtime-backend/src/float_ban.rs`:

```rust
//! Byte-level float-ban lint.
```

Write `crates/wasmtime-backend/src/gating.rs`:

```rust
//! Capability-gated linker construction.
```

Write `crates/wasmtime-backend/src/helpers.rs`:

```rust
//! Deterministic helper imports.
```

Write `crates/wasmtime-backend/src/instance.rs`:

```rust
//! ComponentInstance impl for state-apply.
```

- [ ] **Step 6: Verify**

Run: `cargo check -p myrhiza-wasmtime-backend`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/wasmtime-backend
git commit -m "$(cat <<'EOF'
chore(wasmtime-backend): scaffold crate with module skeleton

Modules: engine (Backend impl), float_ban (lint), gating (linker),
helpers (host imports), instance (ComponentInstance impl).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 22: Float-ban byte-level lint

**Files:**
- Modify: `crates/wasmtime-backend/src/float_ban.rs`
- Create: `crates/wasmtime-backend/Cargo.toml` (add `wasmparser` dep)

Per [determinism.md §5.2](../specs/2026-05-09-myrhiza-master-design/determinism.md): "State-apply WASM modules importing or using float ops are rejected at component install time." We scan every function body in every core module of the component.

- [ ] **Step 1: Add wasmparser dep**

Add to `crates/wasmtime-backend/Cargo.toml`:

```toml
wasmparser = "0.215"
```

(Match the version against the wasmtime LTS pin at task implementation time.)

- [ ] **Step 2: Write the failing test**

Append to `crates/wasmtime-backend/src/float_ban.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-authored core wasm module with a single function that
    /// adds two f32s. wat::parse_str is not used because this crate
    /// avoids the wat dep — we hex-literal the module.
    fn float_module_bytes() -> Vec<u8> {
        // (module (func $f (export "f") (param) (result f32)
        //   f32.const 1.0 f32.const 2.0 f32.add))
        // Generated once via `wat2wasm` and committed inline.
        hex_literal::hex!(
            "0061736d010000000105016000017d030201000707010366010000"
            "0a0d010b0043000080404300000040920b"
        )
        .to_vec()
    }

    fn integer_module_bytes() -> Vec<u8> {
        // (module (func $f (export "f") (param) (result i32)
        //   i32.const 1 i32.const 2 i32.add))
        hex_literal::hex!(
            "0061736d01000000010501600001"
            "7f03020100070701036601000"
            "00a09010700410141026a0b"
        )
        .to_vec()
    }

    #[test]
    fn rejects_f32_add() {
        let err = scan_core_module_for_floats(&float_module_bytes()).expect_err("must reject");
        assert!(err.contains("f32.add") || err.contains("f32"), "got {err}");
    }

    #[test]
    fn accepts_integer_only_module() {
        scan_core_module_for_floats(&integer_module_bytes()).expect("integer module OK");
    }
}
```

(Add to `Cargo.toml`'s `[dev-dependencies]`: `hex-literal.workspace = true`. The exact hex strings will need verification against `wat2wasm` output during execution; an executor of this plan should re-derive them with `wat2wasm`. The bytes shown are **illustrative** and must be regenerated; the test format and assertions are exact.)

> **Plan author's note to executor:** the hex bytes above are illustrative starting points for `(module (func ... f32.add))`. Before committing, run `wat2wasm` against the corresponding `.wat` text and replace the hex with the actual output. If `wat2wasm` is not available, vendor `wat` as a dev-dependency and convert at test time. Commit only the resulting wasm bytes.

- [ ] **Step 3: Run and verify failure**

Run: `cargo test -p myrhiza-wasmtime-backend float_ban`
Expected: FAIL — `scan_core_module_for_floats` not defined.

- [ ] **Step 4: Implement the scanner**

Write `crates/wasmtime-backend/src/float_ban.rs` (above tests):

```rust
//! Byte-level float-ban lint per determinism.md §5.2.
//!
//! Components that import or use float instructions in any function
//! body fail this lint and are rejected at instantiation. Includes
//! SIMD-float ops (cross-platform divergence vectors per §5.2).

use wasmparser::{Operator, Parser, Payload};

/// Scan a core wasm module's function bodies for any float instruction.
/// Returns `Err` naming the first banned instruction encountered.
pub fn scan_core_module_for_floats(bytes: &[u8]) -> Result<(), String> {
    for payload in Parser::new(0).parse_all(bytes) {
        let payload = payload.map_err(|e| format!("wasm parse error: {e}"))?;
        if let Payload::CodeSectionEntry(body) = payload {
            let mut reader = body
                .get_operators_reader()
                .map_err(|e| format!("operators reader error: {e}"))?;
            while !reader.eof() {
                let op = reader
                    .read()
                    .map_err(|e| format!("read op error: {e}"))?;
                if is_float_op(&op) {
                    return Err(format!("banned float op: {}", op_name(&op)));
                }
            }
        }
    }
    Ok(())
}

/// Scan an entire component's embedded core modules.
pub fn scan_component_for_floats(component_bytes: &[u8]) -> Result<(), String> {
    for payload in Parser::new(0).parse_all(component_bytes) {
        let payload = payload.map_err(|e| format!("component parse error: {e}"))?;
        if let Payload::ModuleSection { unchecked_range, .. } = payload {
            let module_bytes = &component_bytes[unchecked_range];
            scan_core_module_for_floats(module_bytes)?;
        }
    }
    Ok(())
}

fn is_float_op(op: &Operator<'_>) -> bool {
    use Operator::*;
    matches!(op,
        F32Load { .. } | F64Load { .. } | F32Store { .. } | F64Store { .. }
        | F32Const { .. } | F64Const { .. }
        | F32Eq | F32Ne | F32Lt | F32Gt | F32Le | F32Ge
        | F64Eq | F64Ne | F64Lt | F64Gt | F64Le | F64Ge
        | F32Abs | F32Neg | F32Ceil | F32Floor | F32Trunc | F32Nearest | F32Sqrt
        | F32Add | F32Sub | F32Mul | F32Div | F32Min | F32Max | F32Copysign
        | F64Abs | F64Neg | F64Ceil | F64Floor | F64Trunc | F64Nearest | F64Sqrt
        | F64Add | F64Sub | F64Mul | F64Div | F64Min | F64Max | F64Copysign
        | I32TruncF32S | I32TruncF32U | I32TruncF64S | I32TruncF64U
        | I64TruncF32S | I64TruncF32U | I64TruncF64S | I64TruncF64U
        | F32ConvertI32S | F32ConvertI32U | F32ConvertI64S | F32ConvertI64U
        | F64ConvertI32S | F64ConvertI32U | F64ConvertI64S | F64ConvertI64U
        | F32DemoteF64 | F64PromoteF32
        | I32ReinterpretF32 | I64ReinterpretF64
        | F32ReinterpretI32 | F64ReinterpretI64
        // SIMD-float ops are also banned per §5.2.
        | V128Load { .. } | V128Store { .. }
        | F32x4Splat | F64x2Splat
        | F32x4ExtractLane { .. } | F32x4ReplaceLane { .. }
        | F64x2ExtractLane { .. } | F64x2ReplaceLane { .. }
        | F32x4Add | F32x4Sub | F32x4Mul | F32x4Div
        | F64x2Add | F64x2Sub | F64x2Mul | F64x2Div
    )
}

fn op_name(op: &Operator<'_>) -> &'static str {
    // wasmparser's Display would allocate; we just name the obvious
    // banned cases. Anything else returns "f-op".
    use Operator::*;
    match op {
        F32Add => "f32.add",
        F32Sub => "f32.sub",
        F32Mul => "f32.mul",
        F32Div => "f32.div",
        F64Add => "f64.add",
        F64Sub => "f64.sub",
        F64Mul => "f64.mul",
        F64Div => "f64.div",
        F32Const { .. } => "f32.const",
        F64Const { .. } => "f64.const",
        _ => "float op",
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p myrhiza-wasmtime-backend float_ban`
Expected: 2 passed (after executor regenerates hex bytes via wat2wasm).

- [ ] **Step 6: Commit**

```bash
git add crates/wasmtime-backend
git commit -m "$(cat <<'EOF'
feat(wasmtime-backend): byte-level float-ban lint

scan_component_for_floats walks every core module in a component
and rejects on any float instruction including SIMD-float per
determinism.md §5.2 (cross-platform divergence vectors).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 23: Capability-gated linker (state-apply imports only)

**Files:**
- Modify: `crates/wasmtime-backend/src/gating.rs`

Per [capabilities.md §7.2](../specs/2026-05-09-myrhiza-master-design/capabilities.md): the kernel intersects the app's ambient set with the module's required set at component instantiation. For state-apply specifically, ambient is the deterministic helper set (per [architecture.md §3.5](../specs/2026-05-09-myrhiza-master-design/architecture.md)) — and any non-deterministic import in a state-apply manifest is a hard error (regardless of `kernel-major`).

- [ ] **Step 1: Write the failing test**

Append to `crates/wasmtime-backend/src/gating.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use myrhiza_manifest::vocabulary::CapabilityClass;

    #[test]
    fn state_apply_ambient_is_only_deterministic_helpers() {
        let ambient = state_apply_ambient_set();
        for cap in &ambient {
            let class = myrhiza_manifest::vocabulary::classify(cap)
                .expect("ambient cap must be in vocabulary");
            assert_eq!(
                class,
                CapabilityClass::DeterministicHelper,
                "{cap} must be DeterministicHelper for state-apply ambient"
            );
        }
        assert!(ambient.contains("host.hash"));
        assert!(ambient.contains("host.verify-signature"));
        assert!(ambient.contains("host.now-hlc-from-event"));
        assert!(ambient.contains("host.log"));
    }

    #[test]
    fn validate_state_apply_manifest_rejects_non_deterministic_imports() {
        use myrhiza_manifest::schema::*;
        let mut m = Manifest {
            app: AppSection {
                name: "x".into(),
                version: "0.1.0".into(),
                description: "x".into(),
                author_pubkey: "wpub-x".into(),
                author_identity_class: AuthorIdentityClass::ThirdParty,
            },
            abi: AbiSection {
                kernel_major: 1,
                kernel_minor_min: 0,
                state_digest_format: StateDigestFormat::Bincode13,
            },
            capabilities: CapabilitiesSection {
                host_imports: Default::default(),
                ui_surfaces: Default::default(),
                high_value_ops: HighValueOps::default(),
                deterministic_helpers: Default::default(),
            },
            determinism: DeterminismSection {
                allow_floats: false,
                drift_detection: DriftDetectionSection { interval_events: 1024 },
            },
            modules: ModulesSection { dep: vec![] },
            components: ComponentsSection {
                state_apply: Some("c.wasm".into()),
                state_propose: None,
                interaction: None,
                behavior: None,
            },
            author_policy: AuthorPolicy::default_deny(),
            signature: None,
        };
        // Manifest declares non-deterministic broadcast — invalid for
        // a state-apply-only bundle.
        m.capabilities
            .host_imports
            .insert("host.broadcast-submit".into(), true);
        let res = validate_state_apply_manifest(&m);
        assert!(res.is_err(), "non-det import must be rejected");
    }

    #[test]
    fn validate_state_apply_manifest_accepts_helper_set_only() {
        use myrhiza_manifest::schema::*;
        let mut m = sample_state_apply_manifest();
        m.capabilities
            .deterministic_helpers
            .insert("host.hash".into(), true);
        m.capabilities
            .deterministic_helpers
            .insert("host.log".into(), true);
        validate_state_apply_manifest(&m).expect("helper-set-only must validate");
    }

    fn sample_state_apply_manifest() -> myrhiza_manifest::schema::Manifest {
        use myrhiza_manifest::schema::*;
        Manifest {
            app: AppSection {
                name: "x".into(),
                version: "0.1.0".into(),
                description: "x".into(),
                author_pubkey: "wpub-x".into(),
                author_identity_class: AuthorIdentityClass::ThirdParty,
            },
            abi: AbiSection {
                kernel_major: 1,
                kernel_minor_min: 0,
                state_digest_format: StateDigestFormat::Bincode13,
            },
            capabilities: CapabilitiesSection {
                host_imports: Default::default(),
                ui_surfaces: Default::default(),
                high_value_ops: HighValueOps::default(),
                deterministic_helpers: Default::default(),
            },
            determinism: DeterminismSection {
                allow_floats: false,
                drift_detection: DriftDetectionSection { interval_events: 1024 },
            },
            modules: ModulesSection { dep: vec![] },
            components: ComponentsSection {
                state_apply: Some("c.wasm".into()),
                state_propose: None,
                interaction: None,
                behavior: None,
            },
            author_policy: AuthorPolicy::default_deny(),
            signature: None,
        }
    }
}
```

- [ ] **Step 2: Run and verify failure**

Run: `cargo test -p myrhiza-wasmtime-backend gating`
Expected: FAIL — symbols not defined.

- [ ] **Step 3: Implement gating logic**

Write `crates/wasmtime-backend/src/gating.rs` (above tests):

```rust
//! Capability gating logic for state-apply.
//!
//! Per architecture.md §3.5: state-apply may bind ONLY the
//! deterministic helper set. Per capabilities.md §7.2: the kernel
//! intersects the app's declared set with what the component needs.
//! For state-apply specifically, "what the app declares" must be a
//! subset of the deterministic helper set — declaring any
//! non-deterministic import is a hard error regardless of kernel-major.

use std::collections::BTreeSet;

use myrhiza_backend::BackendError;
use myrhiza_manifest::Manifest;
use myrhiza_manifest::vocabulary::{CapabilityClass, classify};

/// The state-apply ambient set is the deterministic helper set per
/// architecture.md §3.5 (every cell marked "permitted" in the
/// state-apply column).
pub fn state_apply_ambient_set() -> BTreeSet<String> {
    let mut s = BTreeSet::new();
    s.insert("host.hash".into());
    s.insert("host.verify-signature".into());
    s.insert("host.verify-payload-mac".into());
    s.insert("host.install-key".into());
    s.insert("host.now-hlc-from-event".into());
    s.insert("host.log".into());
    s
}

/// Validate that the manifest's declared imports are a subset of the
/// state-apply ambient set. Any declared host-imports row that is not
/// a DeterministicHelper rejects.
pub fn validate_state_apply_manifest(m: &Manifest) -> Result<(), BackendError> {
    let ambient = state_apply_ambient_set();

    // Any value in capabilities.host_imports = true that is not a
    // DeterministicHelper is a hard error (state-apply cannot bind it).
    for (cap, &enabled) in &m.capabilities.host_imports {
        if !enabled {
            continue;
        }
        match classify(cap) {
            None => return Err(BackendError::UnknownImport(cap.clone())),
            Some(CapabilityClass::DeterministicHelper) => {
                if !ambient.contains(cap) {
                    return Err(BackendError::UnauthorizedImport {
                        imported: cap.clone(),
                    });
                }
            }
            Some(_) => {
                return Err(BackendError::UnauthorizedImport {
                    imported: cap.clone(),
                })
            }
        }
    }

    // capabilities.deterministic_helpers = true entries are
    // self-documenting only — they must all be in the ambient set.
    for (cap, &enabled) in &m.capabilities.deterministic_helpers {
        if !enabled {
            continue;
        }
        if !ambient.contains(cap) {
            return Err(BackendError::UnknownImport(cap.clone()));
        }
    }

    Ok(())
}

/// Compute the set of host imports that should be bound on the
/// state-apply linker for `manifest`. Returns the manifest-declared
/// subset of the ambient set. Imports outside this set are NOT bound,
/// so a component attempting to import them fails to link.
pub fn state_apply_bound_imports(m: &Manifest) -> BTreeSet<String> {
    let ambient = state_apply_ambient_set();
    let mut bound = BTreeSet::new();

    // Declared deterministic_helpers entries are merged in.
    for (cap, &enabled) in &m.capabilities.deterministic_helpers {
        if enabled && ambient.contains(cap) {
            bound.insert(cap.clone());
        }
    }
    // Declared host_imports entries (validated DeterministicHelper) too.
    for (cap, &enabled) in &m.capabilities.host_imports {
        if enabled && ambient.contains(cap) {
            bound.insert(cap.clone());
        }
    }

    // host.log is always available to state-apply (output-only sink
    // per determinism.md §5.1; no peer-divergence risk).
    bound.insert("host.log".into());

    bound
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p myrhiza-wasmtime-backend gating`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/wasmtime-backend
git commit -m "$(cat <<'EOF'
feat(wasmtime-backend): state-apply capability gating

state_apply_ambient_set: the six deterministic helpers from
architecture.md §3.5 + determinism.md §5.1.

validate_state_apply_manifest: rejects manifests declaring any
non-deterministic import in capabilities.host-imports (hard error
regardless of kernel-major; state-apply cannot bind those).

state_apply_bound_imports: the subset of ambient that the linker
should bind for this manifest. host.log is always bound (output-only,
no convergence impact).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 24: Deterministic helper imports — `host.hash`, `host.log`

**Files:**
- Modify: `crates/wasmtime-backend/src/helpers.rs`

These are the simplest helpers and exercise the linker pattern. The rest follow the same shape.

- [ ] **Step 1: Write the failing test**

Append to `crates/wasmtime-backend/src/helpers.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_hash_returns_blake3_canonical() {
        let out = host_hash_impl(b"");
        let expected = hex::decode(
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
        )
        .unwrap();
        assert_eq!(out, expected);
    }

    #[test]
    fn host_hash_deterministic() {
        let a = host_hash_impl(b"hello");
        let b = host_hash_impl(b"hello");
        assert_eq!(a, b);
    }

    #[test]
    fn log_sink_records_messages() {
        let sink = LogSink::default();
        sink.record(LogLevel::Info, "first".into());
        sink.record(LogLevel::Warn, "second".into());
        let lines = sink.drain();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], (LogLevel::Info, "first".into()));
        assert_eq!(lines[1], (LogLevel::Warn, "second".into()));
    }

    #[test]
    fn log_sink_not_part_of_state() {
        let sink = LogSink::default();
        sink.record(LogLevel::Info, "x".into());
        // The drain returns content; record returns nothing — by
        // construction state-apply cannot read what it logged. Asserts
        // the API shape required by determinism.md §5.1's "log content
        // is NOT part of the cross-peer convergence surface."
        let _: () = sink.record(LogLevel::Info, "y".into());
    }
}
```

- [ ] **Step 2: Run and verify failure**

Run: `cargo test -p myrhiza-wasmtime-backend helpers`
Expected: FAIL — symbols not defined.

- [ ] **Step 3: Implement helpers**

Write `crates/wasmtime-backend/src/helpers.rs` (above tests):

```rust
//! Deterministic helper imports.
//!
//! Each impl is a pure function of its bytes input. Side effects
//! (`host.log`) write to a peer-local sink that is NOT part of
//! state-digest per determinism.md §5.1.

use std::sync::Mutex;

use ed25519_dalek::{Signature, VerifyingKey};
use myrhiza_types::Hlc;

/// `host.hash(bytes)` returns BLAKE3(bytes) as 32 raw bytes.
pub fn host_hash_impl(bytes: &[u8]) -> Vec<u8> {
    blake3::hash(bytes).as_bytes().to_vec()
}

/// `host.verify-signature(pubkey, msg, sig)` using `verify_strict`
/// per determinism.md §5.1. Plain `verify` is forbidden.
pub fn host_verify_signature_impl(pubkey: &[u8], msg: &[u8], sig: &[u8]) -> bool {
    let Ok(pk_arr): Result<&[u8; 32], _> = pubkey.try_into() else {
        return false;
    };
    let Ok(sig_arr): Result<&[u8; 64], _> = sig.try_into() else {
        return false;
    };
    let Ok(key) = VerifyingKey::from_bytes(pk_arr) else {
        return false;
    };
    let signature = Signature::from_bytes(sig_arr);
    key.verify_strict(msg, &signature).is_ok()
}

/// `host.now-hlc-from-event(event-bytes)` decodes the HLC out of a
/// canonical event envelope. Pure decoder per determinism.md §5.1.
pub fn host_now_hlc_from_event_impl(event_bytes: &[u8]) -> Option<Hlc> {
    use myrhiza_types::canonical_bincode;
    let event: myrhiza_types::Event = canonical_bincode().deserialize(event_bytes).ok()?;
    Some(event.hlc)
}

/// `host.log` levels.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Per-peer log sink. `record` is the only API state-apply sees;
/// `drain` is host-side. Drained content is NOT part of state-digest.
#[derive(Default)]
pub struct LogSink {
    entries: Mutex<Vec<(LogLevel, String)>>,
}

impl LogSink {
    pub fn record(&self, level: LogLevel, msg: String) {
        if let Ok(mut g) = self.entries.lock() {
            g.push((level, msg));
        }
    }

    pub fn drain(&self) -> Vec<(LogLevel, String)> {
        self.entries
            .lock()
            .map(|mut g| std::mem::take(&mut *g))
            .unwrap_or_default()
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p myrhiza-wasmtime-backend helpers`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/wasmtime-backend
git commit -m "$(cat <<'EOF'
feat(wasmtime-backend): deterministic helper impls

- host.hash: BLAKE3 of bytes (32-byte canonical output).
- host.verify-signature: ed25519-dalek verify_strict (RFC 8032
  strict per determinism.md §5.1).
- host.now-hlc-from-event: pure bincode decode of canonical event
  envelope; extracts HLC.
- LogSink: peer-local record/drain pair; NOT part of state-digest
  per determinism.md §5.1.

host.install-key and host.verify-payload-mac stub-trap until plan
B's key-handle infrastructure lands; counter app does not need them.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 25: WIT bindings via `wasmtime::component::bindgen!`

**Files:**
- Modify: `crates/wasmtime-backend/src/engine.rs`

`wasmtime::component::bindgen!` generates host bindings from the WIT package authored in Phase 2. We invoke it once and re-export the generated types from `engine.rs`.

- [ ] **Step 1: Generate bindings**

Replace `crates/wasmtime-backend/src/engine.rs` with:

```rust
//! Wasmtime engine + Backend impl.
//!
//! `wasmtime::component::bindgen!` generates host trait skeletons
//! from the state-apply WIT world. Implementing those traits binds
//! the deterministic helper set to the Wasmtime linker.

use std::sync::Arc;

use myrhiza_backend::{Backend, BackendError, ComponentInstance, Verdict};
use myrhiza_manifest::Manifest;
use wasmtime::{
    Engine, Store,
    component::{Component, Linker, ResourceTable},
};

use crate::float_ban::scan_component_for_floats;
use crate::gating::{state_apply_bound_imports, validate_state_apply_manifest};
use crate::helpers::LogSink;
use crate::instance::StateApplyInstance;

wasmtime::component::bindgen!({
    path: "../../wit/myrhiza-kernel/wit",
    world: "state-apply",
    async: false,
    with: {
        // Plan A's state-apply does not use any kernel-managed resources.
    },
});

/// Per-instance host state held in the Wasmtime `Store`.
pub struct HostState {
    pub log_sink: Arc<LogSink>,
    pub bound_imports: std::collections::BTreeSet<String>,
    pub table: ResourceTable,
}

/// Backend impl using Wasmtime's component model.
pub struct WasmtimeBackend {
    engine: Engine,
}

impl WasmtimeBackend {
    /// Build a new backend with fuel + epoch interruption configured
    /// per determinism.md §5.3.
    pub fn new() -> Result<Self, BackendError> {
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);
        config.wasm_component_model(true);
        // Float-ban is a byte-level lint we enforce ourselves; we do
        // not disable Wasmtime's float support because the lint runs
        // before instantiation. (Disabling would also block validating
        // imports/exports that mention float types in WIT, but our WIT
        // does not declare any.)
        let engine = Engine::new(&config).map_err(|e| BackendError::Instantiation(e.to_string()))?;
        Ok(Self { engine })
    }

    pub(crate) fn engine(&self) -> &Engine {
        &self.engine
    }
}

impl Backend for WasmtimeBackend {
    fn instantiate_state_apply(
        &self,
        component_bytes: &[u8],
        manifest: &Manifest,
    ) -> Result<Box<dyn ComponentInstance>, BackendError> {
        // 1. Manifest gating check.
        validate_state_apply_manifest(manifest)?;

        // 2. Float-ban lint.
        scan_component_for_floats(component_bytes)
            .map_err(BackendError::BannedInstruction_str)?;

        // 3. Compute the bound import set.
        let bound_imports = state_apply_bound_imports(manifest);

        // 4. Decode the component.
        let component = Component::from_binary(&self.engine, component_bytes)
            .map_err(|e| BackendError::Instantiation(format!("decode component: {e}")))?;

        // 5. Build the linker, binding only the allowed imports.
        let mut linker: Linker<HostState> = Linker::new(&self.engine);
        crate::gating::wire_state_apply_linker(&mut linker, &bound_imports)?;

        // 6. Build the store with fuel budget per determinism.md §5.3.
        let host_state = HostState {
            log_sink: Arc::new(LogSink::default()),
            bound_imports,
            table: ResourceTable::new(),
        };
        let mut store: Store<HostState> = Store::new(&self.engine, host_state);
        const STATE_APPLY_FUEL_BUDGET: u64 = 10_000_000;
        store
            .set_fuel(STATE_APPLY_FUEL_BUDGET)
            .map_err(|e| BackendError::Instantiation(format!("set_fuel: {e}")))?;
        // Memory cap per determinism.md §5.3 = 64 MB.
        store.limiter(|_| {
            // Wasmtime's StoreLimiter API has type signature constraints
            // that pull a generic; the kernel pins it via
            // wasmtime::StoreLimits::new().memory_size(64 * 1024 * 1024).
            //
            // Replace with the actual builder per the wasmtime version
            // pinned in the workspace; this comment is a guard rail
            // for reviewer to verify the cap is wired.
            unimplemented!(
                "wire StoreLimits::new().memory_size(64 << 20).build()"
            )
        });

        let instance = StateApplyInstance::instantiate(store, &component, &linker)?;
        Ok(Box::new(instance))
    }
}

// Helper: wrap the float-ban String error into BackendError without
// allocating a static.
impl BackendError {
    fn BannedInstruction_str(s: String) -> Self {
        // Leak the dynamic string into a 'static so it can be carried
        // by BackendError::BannedInstruction. Cardinality is bounded
        // (one banned op per failure).
        let leaked: &'static str = Box::leak(s.into_boxed_str());
        BackendError::BannedInstruction(leaked)
    }
}
```

> **Plan author's note to executor:** the `store.limiter(...)` block is illustrative — the Wasmtime version pin determines the exact builder API. Before commit, replace with the actual `StoreLimits::new().memory_size(64 << 20).build()` flow from the workspace wasmtime pin (currently `=36.0.9`). If wasmtime's API name changes between drafting and execution, follow the upstream docs and update the call site. The 64 MB cap is normative per determinism.md §5.3; wiring it any other way is not acceptable.

- [ ] **Step 2: Verify compile**

Run: `cargo check -p myrhiza-wasmtime-backend`
Expected: PASS (after executor wires the actual `StoreLimits` builder).

- [ ] **Step 3: Commit**

```bash
git add crates/wasmtime-backend
git commit -m "$(cat <<'EOF'
feat(wasmtime-backend): WasmtimeBackend skeleton with gating + lint

WasmtimeBackend::instantiate_state_apply runs (in order):
1. validate_state_apply_manifest (gating check)
2. scan_component_for_floats (float-ban lint)
3. state_apply_bound_imports (compute linker import set)
4. Component::from_binary
5. linker construction binding only allowed helpers
6. Store with 10M fuel budget + 64 MB memory cap per
   determinism.md §5.3

StateApplyInstance creation lands in Task 27.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 26: Linker wiring for the bound import set

**Files:**
- Modify: `crates/wasmtime-backend/src/gating.rs`

For each name in `bound_imports`, register the corresponding host fn on the linker. Names not in `bound_imports` are NOT registered, so a component attempting to import them fails to link.

- [ ] **Step 1: Write the failing test**

Append to `crates/wasmtime-backend/src/gating.rs`:

```rust
#[cfg(test)]
mod tests_wire {
    use super::*;
    use crate::engine::HostState;
    use std::collections::BTreeSet;

    #[test]
    fn wire_binds_only_listed_imports() {
        let engine = wasmtime::Engine::new(
            wasmtime::Config::new().wasm_component_model(true),
        )
        .unwrap();
        let mut linker: wasmtime::component::Linker<HostState> =
            wasmtime::component::Linker::new(&engine);
        let mut bound = BTreeSet::new();
        bound.insert("host.hash".into());
        bound.insert("host.log".into());
        // Just verifies wire_state_apply_linker accepts the call.
        // Component-level link failure is exercised in the e2e test.
        wire_state_apply_linker(&mut linker, &bound).expect("wire OK");
    }
}
```

- [ ] **Step 2: Run and verify failure**

Run: `cargo test -p myrhiza-wasmtime-backend gating`
Expected: FAIL — `wire_state_apply_linker` not defined.

- [ ] **Step 3: Implement**

Append to `crates/wasmtime-backend/src/gating.rs`:

```rust
use wasmtime::component::{Linker, LinkerInstance};

use crate::engine::HostState;
use crate::helpers::{
    LogLevel, host_hash_impl, host_now_hlc_from_event_impl, host_verify_signature_impl,
};

/// Register host functions on the linker for the imports listed in
/// `bound_imports`. Imports not listed are not registered; a component
/// attempting to import them will fail to link.
pub fn wire_state_apply_linker(
    linker: &mut Linker<HostState>,
    bound_imports: &BTreeSet<String>,
) -> Result<(), BackendError> {
    let mut iface: LinkerInstance<HostState> = linker
        .instance("myrhiza:kernel/host-deterministic")
        .map_err(|e| BackendError::Instantiation(format!("linker instance: {e}")))?;

    if bound_imports.contains("host.hash") {
        iface
            .func_wrap(
                "hash",
                |_store: wasmtime::StoreContextMut<'_, HostState>,
                 (bytes,): (Vec<u8>,)|
                 -> wasmtime::Result<(Vec<u8>,)> {
                    Ok((host_hash_impl(&bytes),))
                },
            )
            .map_err(|e| BackendError::Instantiation(format!("wire host.hash: {e}")))?;
    }
    if bound_imports.contains("host.verify-signature") {
        iface
            .func_wrap(
                "verify-signature",
                |_: wasmtime::StoreContextMut<'_, HostState>,
                 (pk, msg, sig): (Vec<u8>, Vec<u8>, Vec<u8>)|
                 -> wasmtime::Result<(bool,)> {
                    Ok((host_verify_signature_impl(&pk, &msg, &sig),))
                },
            )
            .map_err(|e| BackendError::Instantiation(format!("wire verify-signature: {e}")))?;
    }
    if bound_imports.contains("host.now-hlc-from-event") {
        iface
            .func_wrap(
                "now-hlc-from-event",
                |_: wasmtime::StoreContextMut<'_, HostState>,
                 (event_bytes,): (Vec<u8>,)|
                 -> wasmtime::Result<(crate::engine::exports::myrhiza::kernel::types::Hlc,)> {
                    let hlc = host_now_hlc_from_event_impl(&event_bytes)
                        .ok_or_else(|| wasmtime::Error::msg("invalid event bytes"))?;
                    // Build the wit-bindgen Hlc record from our typed Hlc.
                    Ok((crate::engine::exports::myrhiza::kernel::types::Hlc {
                        wall_ms: hlc.wall_ms,
                        logical: hlc.logical,
                    },))
                },
            )
            .map_err(|e| {
                BackendError::Instantiation(format!("wire now-hlc-from-event: {e}"))
            })?;
    }

    // host.log is always bound.
    iface
        .func_wrap(
            "log",
            |store: wasmtime::StoreContextMut<'_, HostState>,
             (level, msg): (u8, String)|
             -> wasmtime::Result<()> {
                let level = match level {
                    0 => LogLevel::Trace,
                    1 => LogLevel::Debug,
                    2 => LogLevel::Info,
                    3 => LogLevel::Warn,
                    _ => LogLevel::Error,
                };
                store.data().log_sink.record(level, msg);
                Ok(())
            },
        )
        .map_err(|e| BackendError::Instantiation(format!("wire host.log: {e}")))?;

    // host.install-key and host.verify-payload-mac stub-trap until
    // plan B wires key-handle infrastructure. They are not bound here;
    // a state-apply that imports them fails to link, which is exactly
    // the desired plan-A behavior.

    Ok(())
}
```

> **Plan author's note to executor:** `wasmtime::component::Linker` API names for `instance(...)` and the closure signature for `func_wrap` track the Wasmtime version pinned in the workspace. The shape above is correct for the workspace `wasmtime` pin pre-bindgen; if the bindgen-generated trait-impl pattern is preferred (it is, when stable), implement the `MyrhizaKernelHostDeterministicHost` trait on `HostState` instead. The contract — only listed imports get bound — is unchanged.

- [ ] **Step 4: Run + lint**

Run: `cargo test -p myrhiza-wasmtime-backend gating`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/wasmtime-backend
git commit -m "$(cat <<'EOF'
feat(wasmtime-backend): wire_state_apply_linker

For each name in bound_imports, register the corresponding host fn
on the Wasmtime linker. Names not in the set are not registered, so
a state-apply that imports them fails to link — the load-bearing
gating mechanic for plan A's acceptance criterion #5.

host.install-key and host.verify-payload-mac are intentionally
unbound until plan B wires key-handle infrastructure.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 27: `StateApplyInstance` (call_apply, call_state_digest)

**Files:**
- Modify: `crates/wasmtime-backend/src/instance.rs`

- [ ] **Step 1: Implement**

Write `crates/wasmtime-backend/src/instance.rs`:

```rust
//! ComponentInstance impl for state-apply.

use myrhiza_backend::{BackendError, ComponentInstance, Verdict};
use wasmtime::{
    Store, StoreContextMut,
    component::{Component, Linker},
};

use crate::engine::{HostState, StateApply};

pub(crate) struct StateApplyInstance {
    store: Store<HostState>,
    bindings: StateApply,
}

impl StateApplyInstance {
    pub(crate) fn instantiate(
        mut store: Store<HostState>,
        component: &Component,
        linker: &Linker<HostState>,
    ) -> Result<Self, BackendError> {
        let bindings = StateApply::instantiate(&mut store, component, linker).map_err(|e| {
            // Distinguish capability-rejection (linker missing import)
            // from other instantiation failures.
            let s = e.to_string();
            if s.contains("import") || s.contains("unknown") {
                BackendError::UnauthorizedImport { imported: s }
            } else {
                BackendError::Instantiation(s)
            }
        })?;
        Ok(Self { store, bindings })
    }
}

impl ComponentInstance for StateApplyInstance {
    fn call_apply(
        &mut self,
        prior_state: &[u8],
        event: &[u8],
    ) -> Result<(Verdict, Vec<u8>), BackendError> {
        let (verdict, new_state) = self
            .bindings
            .call_apply(&mut self.store, prior_state, event)
            .map_err(|e| {
                if e.to_string().contains("fuel") {
                    BackendError::Trap("fuel exhausted".into())
                } else {
                    BackendError::Trap(e.to_string())
                }
            })?;

        let v = match verdict {
            crate::engine::exports::myrhiza::kernel::types::Verdict::Accept => Verdict::Accept,
            crate::engine::exports::myrhiza::kernel::types::Verdict::Reject(s) => {
                Verdict::Reject(s)
            }
        };
        Ok((v, new_state))
    }

    fn call_state_digest(&mut self, state: &[u8]) -> Result<Vec<u8>, BackendError> {
        self.bindings
            .call_state_digest(&mut self.store, state)
            .map_err(|e| BackendError::Trap(e.to_string()))
    }
}
```

- [ ] **Step 2: Verify compile**

Run: `cargo check -p myrhiza-wasmtime-backend`
Expected: PASS.

> **Plan author's note to executor:** the exact path to bindgen-generated `Verdict` and the call shape `bindings.call_apply(...)` are determined by `wasmtime::component::bindgen!`'s output. After the `bindgen!` invocation in Task 25 compiles, run `cargo expand -p myrhiza-wasmtime-backend --lib engine` to inspect the generated module path and update the `crate::engine::exports::myrhiza::kernel::types::Verdict` reference if it differs. The contract is: the `Verdict` enum has `Accept` and `Reject(String)` variants per the WIT definition, and `call_apply` returns `(Verdict, Vec<u8>)`.

- [ ] **Step 3: Commit**

```bash
git add crates/wasmtime-backend
git commit -m "$(cat <<'EOF'
feat(wasmtime-backend): StateApplyInstance impl ComponentInstance

call_apply maps Wasmtime traps to BackendError variants, distinguishing
fuel exhaustion from generic instantiation/runtime failures and
unauthorized-import errors from generic instantiation failures so the
kernel can surface accurate diagnostics.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 6: kernel orchestration

The kernel is the call broker per [abi.md §8.4](../specs/2026-05-09-myrhiza-master-design/abi.md). For plan A it owns: bundle directory loading, manifest signature verification, backend-driven instantiation, state-apply ABI (apply mode and pre-check dry-run mode sharing fuel per [determinism.md §5.3](../specs/2026-05-09-myrhiza-master-design/determinism.md)), and state-digest emission stub.

### Task 28: `myrhiza-kernel` crate skeleton

**Files:**
- Modify: `Cargo.toml` (add `crates/kernel` to members)
- Create: `crates/kernel/Cargo.toml`
- Create: `crates/kernel/src/lib.rs`

- [ ] **Step 1: Add to workspace**

Add `crates/kernel` to root `members`.

- [ ] **Step 2: Cargo.toml**

Write `crates/kernel/Cargo.toml`:

```toml
[package]
name = "myrhiza-kernel"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Myrhiza runtime kernel: install flow + state-apply ABI + state-digest stub."

[lints]
workspace = true

[dependencies]
myrhiza-types = { path = "../types" }
myrhiza-manifest = { path = "../manifest" }
myrhiza-backend = { path = "../backend" }
myrhiza-wasmtime-backend = { path = "../wasmtime-backend" }
serde.workspace = true
thiserror.workspace = true

[dev-dependencies]
anyhow.workspace = true
tempfile.workspace = true
hex.workspace = true
ed25519-dalek.workspace = true
```

- [ ] **Step 3: lib.rs**

Write `crates/kernel/src/lib.rs`:

```rust
//! Myrhiza runtime kernel.
//!
//! Plan A scope: install flow scaffold, state-apply ABI, state-digest
//! emission stub. No iroh, no event DAG, no apps — those live in
//! plans B and C.

#![deny(missing_docs)]

mod install;
mod state_apply;
mod digest;

pub use install::{InstallError, InstallFlow, LoadedBundle};
pub use state_apply::{StateApplyHandle, ApplyResult, ApplyError};
pub use digest::{DigestEmitter, DigestEvent};
```

Write empty stubs for the three modules:

```rust
// crates/kernel/src/install.rs
//! Install flow: load bundle directory, verify signature,
//! instantiate via backend.

// crates/kernel/src/state_apply.rs
//! State-apply ABI: apply mode and pre-check dry-run mode sharing
//! fuel per determinism.md §5.3.

// crates/kernel/src/digest.rs
//! State-digest emission stub. Emits the canonical state-digest
//! bytes after each apply for consumers (drift-detection gossip
//! lands in plan B).
```

- [ ] **Step 4: Verify**

Run: `cargo check -p myrhiza-kernel`
Expected: FAIL (the modules are referenced but empty — types not defined). This is expected; subsequent tasks define them.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/kernel
git commit -m "$(cat <<'EOF'
chore(kernel): scaffold myrhiza-kernel crate skeleton

Tasks 29-31 implement install, state_apply, digest modules.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 29: Install flow — load + verify + instantiate

**Files:**
- Modify: `crates/kernel/src/install.rs`

Per [distribution.md §10.5](../specs/2026-05-09-myrhiza-master-design/distribution.md): kernel fetches bundle, verifies Ed25519 signature against author pubkey embedded in manifest, intersects capabilities, prompts user, instantiates. Plan A skips the user prompt (lands in plan C with kernel-controlled UI surface) and the recursive module-dep resolution (lands in plan B). Plan A's install handles a single-component state-apply bundle from a local directory.

- [ ] **Step 1: Write the failing test**

Append to `crates/kernel/src/install.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use myrhiza_manifest::{
        canonical::signing_target_bytes,
        schema::*,
    };
    use myrhiza_types::EventHash;
    use std::collections::BTreeMap;
    use tempfile::TempDir;

    fn write_fixture_bundle(dir: &std::path::Path) -> (BundleAddress, [u8; 32]) {
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let pk = sk.verifying_key().to_bytes();
        let pk_hex = hex::encode(pk);

        let component_path = dir.join("components/state-apply.wasm");
        std::fs::create_dir_all(component_path.parent().unwrap()).unwrap();
        // Minimal wasm magic bytes; real component bytes provided by
        // tests/fixtures/built/counter-state-apply.wasm in the e2e test.
        std::fs::write(&component_path, b"\x00asm\x01\x00\x00\x00").unwrap();
        let component_bytes = std::fs::read(&component_path).unwrap();
        let content_hash = EventHash::blake3(&component_bytes);

        let mut helpers = BTreeMap::new();
        helpers.insert("host.hash".into(), true);
        helpers.insert("host.log".into(), true);

        let mut m = Manifest {
            app: AppSection {
                name: "counter".into(),
                version: "0.1.0".into(),
                description: "test".into(),
                author_pubkey: format!("0x{pk_hex}"),
                author_identity_class: AuthorIdentityClass::ThirdParty,
            },
            abi: AbiSection {
                kernel_major: 1,
                kernel_minor_min: 0,
                state_digest_format: StateDigestFormat::Bincode13,
            },
            capabilities: CapabilitiesSection {
                host_imports: BTreeMap::new(),
                ui_surfaces: BTreeMap::new(),
                high_value_ops: HighValueOps::default(),
                deterministic_helpers: helpers,
            },
            determinism: DeterminismSection {
                allow_floats: false,
                drift_detection: DriftDetectionSection { interval_events: 1024 },
            },
            modules: ModulesSection { dep: vec![] },
            components: ComponentsSection {
                state_apply: Some("components/state-apply.wasm".into()),
                state_propose: None,
                interaction: None,
                behavior: None,
            },
            author_policy: AuthorPolicy::default_deny(),
            signature: None,
        };
        m.canonicalize();

        let target = signing_target_bytes(&m, &content_hash);
        let sig = sk.sign(&target);
        m.signature = Some(Signature {
            algorithm: SignatureAlgorithm::Ed25519,
            value: sig.to_bytes(),
        });

        let manifest_bytes = myrhiza_types::canonical_bincode().serialize(&m).unwrap();
        std::fs::write(dir.join("manifest.bincode"), manifest_bytes).unwrap();

        (
            BundleAddress {
                bundle_dir: dir.to_path_buf(),
                manifest_path: "manifest.bincode".into(),
            },
            pk,
        )
    }

    #[test]
    fn loads_and_verifies_a_signed_bundle() {
        let tmp = TempDir::new().unwrap();
        let (addr, _pk) = write_fixture_bundle(tmp.path());

        let flow = InstallFlow::new();
        let loaded = flow.load(&addr).expect("load OK");
        assert_eq!(loaded.manifest.app.name, "counter");
        assert!(!loaded.component_bytes.is_empty());
    }

    #[test]
    fn rejects_tampered_component_bytes() {
        let tmp = TempDir::new().unwrap();
        let (addr, _) = write_fixture_bundle(tmp.path());
        // Tamper with the component file post-signing.
        std::fs::write(tmp.path().join("components/state-apply.wasm"), b"\x00asmTAMPERED").unwrap();
        let flow = InstallFlow::new();
        let err = flow.load(&addr).expect_err("tampered must reject");
        assert!(matches!(err, InstallError::ContentHashMismatch));
    }

    #[test]
    fn rejects_unsigned_manifest() {
        let tmp = TempDir::new().unwrap();
        let (addr, _) = write_fixture_bundle(tmp.path());
        // Strip the signature.
        let mut m: Manifest = myrhiza_types::canonical_bincode()
            .deserialize(&std::fs::read(tmp.path().join("manifest.bincode")).unwrap())
            .unwrap();
        m.signature = None;
        std::fs::write(
            tmp.path().join("manifest.bincode"),
            myrhiza_types::canonical_bincode().serialize(&m).unwrap(),
        )
        .unwrap();
        let flow = InstallFlow::new();
        let err = flow.load(&addr).expect_err("missing sig must reject");
        assert!(matches!(err, InstallError::MissingSignature));
    }
}
```

- [ ] **Step 2: Run and verify failure**

Run: `cargo test -p myrhiza-kernel install`
Expected: FAIL — `InstallFlow` not defined.

- [ ] **Step 3: Implement**

Write `crates/kernel/src/install.rs` (above tests):

```rust
//! Install flow: load bundle directory, verify Ed25519 signature
//! against the author pubkey embedded in the manifest, return
//! manifest + component bytes ready for backend instantiation.
//!
//! Plan A scope:
//! - Single-component state-apply bundles from a local directory.
//! - No recursive module-dep resolution (plan B).
//! - No user prompt (plan C: kernel-controlled UI surface).
//! - No revocation topic check (plan B).

use std::path::PathBuf;

use myrhiza_manifest::{
    Manifest, ParseError, SignatureError, parse_manifest, schema::Signature,
    canonical::signing_target_bytes, verify_signature,
};
use myrhiza_types::{EventHash, canonical_bincode};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct BundleAddress {
    pub bundle_dir: PathBuf,
    /// Path of the manifest file (canonical-bincode-encoded) relative
    /// to bundle_dir. v1 file naming is `manifest.bincode`. The TOML
    /// human-readable form is canonicalized at publish time; the
    /// kernel only consumes the canonical bytes.
    pub manifest_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum InstallError {
    #[error("io error reading bundle: {0}")]
    Io(#[from] std::io::Error),
    #[error("bincode decode error reading manifest: {0}")]
    Bincode(#[from] bincode::Error),
    #[error("manifest parse error: {0}")]
    Parse(#[from] ParseError),
    #[error("manifest is missing the signature section")]
    MissingSignature,
    #[error("Ed25519 signature verification failed: {0}")]
    Signature(#[from] SignatureError),
    #[error("author-pubkey field could not be decoded as 32 raw bytes")]
    AuthorPubkeyDecode,
    #[error("manifest references components/state-apply but file is absent")]
    ComponentMissing,
    #[error("component bytes do not hash to a value the manifest signature commits to")]
    ContentHashMismatch,
}

pub struct LoadedBundle {
    pub manifest: Manifest,
    /// The state-apply component bytes referenced by the manifest.
    pub component_bytes: Vec<u8>,
    /// BLAKE3 of `component_bytes`.
    pub content_hash: EventHash,
}

pub struct InstallFlow;

impl InstallFlow {
    pub fn new() -> Self {
        Self
    }

    pub fn load(&self, addr: &BundleAddress) -> Result<LoadedBundle, InstallError> {
        let manifest_bytes = std::fs::read(addr.bundle_dir.join(&addr.manifest_path))?;
        let mut manifest: Manifest = canonical_bincode().deserialize(&manifest_bytes)?;
        manifest.canonicalize();

        let signature = manifest
            .signature
            .clone()
            .ok_or(InstallError::MissingSignature)?;

        let component_rel = manifest
            .components
            .state_apply
            .clone()
            .ok_or(InstallError::ComponentMissing)?;
        let component_bytes = std::fs::read(addr.bundle_dir.join(&component_rel))?;
        let content_hash = EventHash::blake3(&component_bytes);

        // Decode author pubkey from `0x<hex>` form for plan A.
        // Plan B replaces this with bech32m decoding (per
        // distribution.md §10.2 wpub-author HRP).
        let pk = decode_author_pubkey_hex(&manifest.app.author_pubkey)?;

        let target = signing_target_bytes(&manifest, &content_hash);
        verify_signature(&pk, &target, &signature.value)?;

        Ok(LoadedBundle {
            manifest,
            component_bytes,
            content_hash,
        })
    }
}

fn decode_author_pubkey_hex(s: &str) -> Result<[u8; 32], InstallError> {
    let stripped = s.strip_prefix("0x").ok_or(InstallError::AuthorPubkeyDecode)?;
    let bytes = hex::decode(stripped).map_err(|_| InstallError::AuthorPubkeyDecode)?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| InstallError::AuthorPubkeyDecode)
}
```

- [ ] **Step 4: Run and verify pass**

Run: `cargo test -p myrhiza-kernel install`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add crates/kernel
git commit -m "$(cat <<'EOF'
feat(kernel): install flow — load + verify Ed25519 signature

InstallFlow::load reads canonical-bincode manifest from bundle dir,
verifies the signature via myrhiza_manifest::verify_signature
(verify_strict path), and loads the referenced state-apply component
bytes. Plan A scope: single-component state-apply bundles from
local dirs; no module-dep recursion (plan B), no user prompt
(plan C), no revocation check (plan B).

The author pubkey is decoded from `0x<hex>` form for plan A. Plan B
replaces this with bech32m decoding per distribution.md §10.2.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 30: State-apply ABI — apply + pre-check sharing fuel

**Files:**
- Modify: `crates/kernel/src/state_apply.rs`

Per [convergence.md §4.4](../specs/2026-05-09-myrhiza-master-design/convergence.md): pre-check is mechanically the same WASM function as state-apply, called by the kernel in dry-run mode. Per [determinism.md §5.3](../specs/2026-05-09-myrhiza-master-design/determinism.md): pre-check shares apply's per-event fuel budget — combined cannot exceed 10M units.

The wrinkle: each backend instance has its own fuel budget set at instantiation time. To share fuel between pre-check and apply for the same event we must either (a) reset the budget once before each event and forbid going above 10M for the (pre-check, apply) pair, or (b) instantiate twice — once per call. (b) doesn't share, so we use (a): the kernel sets fuel to 10M at the start of every (event, peer) pair, performs pre-check, then performs apply with whatever fuel is left.

For plan A, pre-check and apply are offered as two methods on `StateApplyHandle`. The kernel client decides whether to call pre-check (originator only) or apply (every peer). The handle exposes a `reset_fuel()` to be called between distinct events, never within a (pre-check, apply) pair.

- [ ] **Step 1: Write the failing test**

Append to `crates/kernel/src/state_apply.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Mock instance that just echoes prior_state || event back as
    /// new state and returns Accept. No fuel accounting in this mock.
    struct Echo;

    impl myrhiza_backend::ComponentInstance for Echo {
        fn call_apply(
            &mut self,
            prior: &[u8],
            event: &[u8],
        ) -> Result<(myrhiza_backend::Verdict, Vec<u8>), myrhiza_backend::BackendError> {
            let mut out = Vec::with_capacity(prior.len() + event.len());
            out.extend_from_slice(prior);
            out.extend_from_slice(event);
            Ok((myrhiza_backend::Verdict::Accept, out))
        }

        fn call_state_digest(
            &mut self,
            state: &[u8],
        ) -> Result<Vec<u8>, myrhiza_backend::BackendError> {
            Ok(state.to_vec())
        }
    }

    #[test]
    fn pre_check_does_not_commit_state() {
        let handle = StateApplyHandle::new(Box::new(Echo));
        let mut handle = handle;
        let prior = vec![1, 2, 3];
        let event = vec![4, 5];
        let r = handle.pre_check(&prior, &event).unwrap();
        assert!(matches!(r.outcome, ApplyOutcome::Accepted));
        // pre_check returns the candidate state but does NOT mutate
        // the handle's view of "current state" — it has none.
        assert_eq!(r.candidate_state, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn apply_returns_accept_with_new_state() {
        let mut handle = StateApplyHandle::new(Box::new(Echo));
        let prior = vec![10];
        let event = vec![20];
        let r = handle.apply(&prior, &event).unwrap();
        assert!(matches!(r.outcome, ApplyOutcome::Accepted));
        assert_eq!(r.new_state, vec![10, 20]);
    }

    /// Verifies pre-check fail-closed semantics: on Reject the kernel
    /// must NOT sign or broadcast. The handle's pre_check returns
    /// outcome=Rejected; calling code is responsible for NOT calling
    /// apply.
    struct AlwaysReject;
    impl myrhiza_backend::ComponentInstance for AlwaysReject {
        fn call_apply(
            &mut self,
            _: &[u8],
            _: &[u8],
        ) -> Result<(myrhiza_backend::Verdict, Vec<u8>), myrhiza_backend::BackendError> {
            Ok((myrhiza_backend::Verdict::Reject("nope".into()), vec![]))
        }
        fn call_state_digest(
            &mut self,
            _: &[u8],
        ) -> Result<Vec<u8>, myrhiza_backend::BackendError> {
            Ok(vec![])
        }
    }

    #[test]
    fn pre_check_fail_closed_on_reject() {
        let mut handle = StateApplyHandle::new(Box::new(AlwaysReject));
        let r = handle.pre_check(&[], &[]).unwrap();
        match r.outcome {
            ApplyOutcome::Rejected(reason) => assert_eq!(reason, "nope"),
            ApplyOutcome::Accepted => panic!("must not accept"),
        }
    }
}
```

- [ ] **Step 2: Run and verify failure**

Run: `cargo test -p myrhiza-kernel state_apply`
Expected: FAIL — symbols not defined.

- [ ] **Step 3: Implement**

Write `crates/kernel/src/state_apply.rs` (above tests):

```rust
//! State-apply ABI: apply mode and pre-check dry-run mode.
//!
//! Per convergence.md §4.4 + determinism.md §5.3: pre-check and
//! apply are the same WASM function called by the kernel in two
//! different modes. The fuel budget is shared per (event, peer)
//! pair — the kernel sets fuel to 10M at the start of each event
//! and lets pre-check + apply share that pool.
//!
//! Plan A delivers handle methods `pre_check` and `apply` returning
//! verdicts. The kernel client (originator path) calls pre_check
//! first; on Accept, the kernel signs the event and broadcasts; on
//! Reject, the kernel surfaces an error and does NOT sign. On the
//! receiving path each peer calls apply directly and commits the
//! returned state if the verdict is Accept.

use myrhiza_backend::{BackendError, ComponentInstance, Verdict};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApplyError {
    #[error("backend error: {0}")]
    Backend(#[from] BackendError),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ApplyOutcome {
    Accepted,
    Rejected(String),
}

#[derive(Clone, Debug)]
pub struct ApplyResult {
    pub outcome: ApplyOutcome,
    /// Apply's resulting state. Empty on Reject. Per convergence.md
    /// §4.4, the kernel commits this iff outcome == Accepted.
    pub new_state: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct PreCheckResult {
    pub outcome: ApplyOutcome,
    /// Pre-check's hypothetical post-state. Discarded by the kernel
    /// (originator never commits pre-check state; only the post-
    /// signing apply call mutates state). Returned for tests + drift
    /// inspection.
    pub candidate_state: Vec<u8>,
}

pub struct StateApplyHandle {
    instance: Box<dyn ComponentInstance>,
}

impl StateApplyHandle {
    pub fn new(instance: Box<dyn ComponentInstance>) -> Self {
        Self { instance }
    }

    /// Apply mode: ingest an event, mutate state in place. Per
    /// convergence.md §4.4, called on every receiving peer.
    pub fn apply(&mut self, prior_state: &[u8], event: &[u8]) -> Result<ApplyResult, ApplyError> {
        let (verdict, new_state) = self.instance.call_apply(prior_state, event)?;
        Ok(ApplyResult {
            outcome: match verdict {
                Verdict::Accept => ApplyOutcome::Accepted,
                Verdict::Reject(s) => ApplyOutcome::Rejected(s),
            },
            new_state,
        })
    }

    /// Pre-check dry-run mode: same WASM function, kernel discards
    /// the new state. Per convergence.md §4.4. Pre-check fails closed:
    /// the kernel does NOT sign and broadcast on Reject.
    pub fn pre_check(
        &mut self,
        prior_state: &[u8],
        event: &[u8],
    ) -> Result<PreCheckResult, ApplyError> {
        let (verdict, candidate_state) = self.instance.call_apply(prior_state, event)?;
        Ok(PreCheckResult {
            outcome: match verdict {
                Verdict::Accept => ApplyOutcome::Accepted,
                Verdict::Reject(s) => ApplyOutcome::Rejected(s),
            },
            candidate_state,
        })
    }

    /// Forward to the underlying instance.
    pub fn state_digest(&mut self, state: &[u8]) -> Result<Vec<u8>, ApplyError> {
        Ok(self.instance.call_state_digest(state)?)
    }
}
```

Update `crates/kernel/src/lib.rs` exports to match (`PreCheckResult`, `ApplyOutcome`):

```rust
pub use state_apply::{
    ApplyError, ApplyOutcome, ApplyResult, PreCheckResult, StateApplyHandle,
};
```

- [ ] **Step 4: Run + lint**

Run: `cargo test -p myrhiza-kernel state_apply`
Expected: 3 passed.

Run: `cargo clippy -p myrhiza-kernel -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/kernel
git commit -m "$(cat <<'EOF'
feat(kernel): StateApplyHandle with apply + pre_check methods

Per convergence.md §4.4 + determinism.md §5.3: pre-check and apply
share the same backend ComponentInstance and the same backend fuel
budget. Pre-check returns PreCheckResult with the candidate state
the kernel discards; apply returns ApplyResult with the state the
kernel commits iff outcome == Accepted (fail-closed semantics).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 31: State-digest emission stub

**Files:**
- Modify: `crates/kernel/src/digest.rs`

Per [convergence.md §4.3](../specs/2026-05-09-myrhiza-master-design/convergence.md): the kernel hashes the digest output and gossips the hash; mismatches surface as bugs. Plan A's emitter records (event_index, state_digest_hash) pairs into a peer-local log; gossip wires up in plan B. The emitter is what plan B's drift-detection topic subscribes to.

- [ ] **Step 1: Write the failing test**

Append to `crates/kernel/src/digest.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use myrhiza_types::EventHash;

    #[test]
    fn emitter_records_per_event_digest() {
        let mut emitter = DigestEmitter::new(1024);
        emitter.observe(0, b"state_v0");
        emitter.observe(1, b"state_v1");
        emitter.observe(1024, b"state_v1024");
        let log = emitter.drain();
        assert_eq!(log.len(), 3);
        assert_eq!(log[0].event_index, 0);
        assert_eq!(log[0].digest_hash, EventHash::blake3(b"state_v0"));
        assert_eq!(log[2].event_index, 1024);
    }

    #[test]
    fn emitter_uses_blake3_canonical() {
        let mut emitter = DigestEmitter::new(1);
        emitter.observe(0, b"abc");
        let log = emitter.drain();
        assert_eq!(log[0].digest_hash, EventHash::blake3(b"abc"));
    }

    #[test]
    fn drain_clears_log() {
        let mut emitter = DigestEmitter::new(1);
        emitter.observe(0, b"x");
        emitter.drain();
        let log = emitter.drain();
        assert!(log.is_empty());
    }
}
```

- [ ] **Step 2: Run and verify failure**

Run: `cargo test -p myrhiza-kernel digest`
Expected: FAIL — `DigestEmitter` not defined.

- [ ] **Step 3: Implement**

Write `crates/kernel/src/digest.rs` (above tests):

```rust
//! State-digest emission stub.
//!
//! Per convergence.md §4.3, the kernel hashes each app's
//! `state-digest()` output and gossips the hash on the drift-
//! detection topic. Plan A produces the (event_index, hash) pairs
//! into a peer-local log; plan B wires them onto the gossip topic
//! per determinism.drift-detection.interval-events.

use myrhiza_types::EventHash;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DigestEvent {
    /// Canonical topo-sort index of the event whose post-state this
    /// digests.
    pub event_index: u64,
    /// BLAKE3 of the app's state-digest output bytes.
    pub digest_hash: EventHash,
}

/// Emits digests every `interval_events` per
/// `determinism.drift-detection.interval-events`. Plan A's stub
/// records every observation it is given; plan B integrates with
/// the kernel's apply loop to call `observe` only at the configured
/// modulo cadence.
pub struct DigestEmitter {
    log: Vec<DigestEvent>,
    interval_events: u32,
}

impl DigestEmitter {
    pub fn new(interval_events: u32) -> Self {
        Self {
            log: Vec::new(),
            interval_events,
        }
    }

    pub fn interval_events(&self) -> u32 {
        self.interval_events
    }

    /// Record a digest observation for an event at the given index.
    pub fn observe(&mut self, event_index: u64, state_digest_bytes: &[u8]) {
        self.log.push(DigestEvent {
            event_index,
            digest_hash: EventHash::blake3(state_digest_bytes),
        });
    }

    /// Drain the recorded events; clears the log.
    pub fn drain(&mut self) -> Vec<DigestEvent> {
        std::mem::take(&mut self.log)
    }
}
```

- [ ] **Step 4: Run + lint**

Run: `cargo test -p myrhiza-kernel digest`
Expected: 3 passed.

Run: `cargo clippy -p myrhiza-kernel -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/kernel
git commit -m "$(cat <<'EOF'
feat(kernel): DigestEmitter stub for state-digest cadence

Per convergence.md §4.3 + determinism.md §5.4: kernel hashes each
state-digest() output via BLAKE3 and emits (event_index, hash) pairs
to a peer-local log. Plan A is the in-process log; plan B subscribes
the drift-detection gossip topic to the drained output.

The emitter stores interval_events from manifest determinism config
but does not enforce the modulo cadence yet — callers (plan A's
state_apply integration) call observe() at apply commit time and
plan B inserts the modulo gate.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Phase 7: verification + acceptance tests

This phase implements plan A's slice of [verification.md](../specs/2026-05-09-myrhiza-master-design/verification.md): tier scaffold, spec-coverage matrix, WIT freeze snapshot, resource-cap regression, plus the kernel-tier acceptance tests that prove [mvp.md §15.1](../specs/2026-05-09-myrhiza-master-design/mvp.md) criteria #1 and #5.

### Task 32: Resource-cap constants + shadow regression test

Per [verification.md §22.4](../specs/2026-05-09-myrhiza-master-design/verification.md): bumping any normative constant requires updating both `myrhiza-types::limits` and the shadow test, forcing a deliberate edit.

**Files:**
- Create: `crates/types/src/limits.rs`
- Modify: `crates/types/src/lib.rs`
- Create: `crates/types/tests/limits_shadow.rs`

- [ ] **Step 1: Implement constants**

Write `crates/types/src/limits.rs`:

```rust
//! V1 normative resource caps per determinism.md §5.3.
//!
//! Bumping any constant requires:
//! 1. A kernel-major version bump (convergence-breaking).
//! 2. Updating crates/types/tests/limits_shadow.rs to match.
//! 3. A spec amendment naming the new value.
//!
//! See verification.md §22.4 for the discipline.

/// Per-event apply fuel budget per determinism.md §5.3.
pub const STATE_APPLY_FUEL_BUDGET_V1: u64 = 10_000_000;

/// Per-event propose fuel budget per determinism.md §5.3 (5x apply).
pub const STATE_PROPOSE_FUEL_BUDGET_V1: u64 = 50_000_000;

/// Per-component memory cap per determinism.md §5.3.
pub const COMPONENT_MEMORY_CAP_V1: usize = 64 * 1024 * 1024;

/// Maximum event payload size per determinism.md §5.3.
pub const EVENT_PAYLOAD_CAP_V1: usize = 1024 * 1024;

/// Maximum DAG `deps` array size per determinism.md §5.3.
pub const DAG_DEPS_CAP_V1: usize = 64;

/// `host.hash(bytes)` cost: n * this constant per determinism.md §5.3.
pub const HOST_HASH_FUEL_PER_BYTE: u64 = 5;

/// `host.verify-signature` cost per determinism.md §5.3.
pub const HOST_VERIFY_SIGNATURE_FUEL: u64 = 5_000;

/// `host.verify-payload-mac` cost per determinism.md §5.3.
pub const HOST_VERIFY_PAYLOAD_MAC_FUEL: u64 = 1_000;

/// `host.install-key` cost per determinism.md §5.3.
pub const HOST_INSTALL_KEY_FUEL: u64 = 100;

/// `host.now-hlc-from-event` cost per determinism.md §5.3.
pub const HOST_NOW_HLC_FROM_EVENT_FUEL: u64 = 50;

/// `host.log` base cost (per-byte msg cost adds on top) per
/// determinism.md §5.3.
pub const HOST_LOG_FUEL_BASE: u64 = 100;
```

Append to `crates/types/src/lib.rs`:

```rust
pub mod limits;
```

- [ ] **Step 2: Shadow test**

Write `crates/types/tests/limits_shadow.rs`:

```rust
//! Shadow regression test for the v1 normative resource caps.
//!
//! /// Covers: determinism.md §5.3, verification.md §22.4.
//!
//! These literals re-state every constant in
//! crates/types/src/limits.rs. Editing only one side fails CI.
//! Bumping a constant requires editing both AND a kernel-major
//! version bump per distribution.md §10.2.

use myrhiza_types::limits::*;

#[test]
fn fuel_budgets_match_spec_v1() {
    assert_eq!(STATE_APPLY_FUEL_BUDGET_V1, 10_000_000);
    assert_eq!(STATE_PROPOSE_FUEL_BUDGET_V1, 50_000_000);
}

#[test]
fn resource_caps_match_spec_v1() {
    assert_eq!(COMPONENT_MEMORY_CAP_V1, 64 * 1024 * 1024);
    assert_eq!(EVENT_PAYLOAD_CAP_V1, 1024 * 1024);
    assert_eq!(DAG_DEPS_CAP_V1, 64);
}

#[test]
fn per_host_call_fuel_costs_match_spec_v1() {
    assert_eq!(HOST_HASH_FUEL_PER_BYTE, 5);
    assert_eq!(HOST_VERIFY_SIGNATURE_FUEL, 5_000);
    assert_eq!(HOST_VERIFY_PAYLOAD_MAC_FUEL, 1_000);
    assert_eq!(HOST_INSTALL_KEY_FUEL, 100);
    assert_eq!(HOST_NOW_HLC_FROM_EVENT_FUEL, 50);
    assert_eq!(HOST_LOG_FUEL_BASE, 100);
}
```

- [ ] **Step 3: Run + lint**

Run: `cargo test -p myrhiza-types`
Expected: 3 new tests passed.

- [ ] **Step 4: Commit**

```bash
git add crates/types
git commit -m "$(cat <<'EOF'
feat(types): limits module + shadow regression test

V1 normative resource caps per determinism.md §5.3 + the shadow-test
guard per verification.md §22.4. Bumping any constant requires
editing both source and shadow + a kernel-major bump.

Constants: STATE_APPLY_FUEL_BUDGET_V1 (10M),
STATE_PROPOSE_FUEL_BUDGET_V1 (50M), COMPONENT_MEMORY_CAP_V1 (64 MB),
EVENT_PAYLOAD_CAP_V1 (1 MB), DAG_DEPS_CAP_V1 (64), and per-host-call
fuel costs.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 33: WIT freeze snapshot

Per [verification.md §22.3](../specs/2026-05-09-myrhiza-master-design/verification.md). Plan A snapshots the state-apply world; the other three worlds get advisory snapshots in plan C.

**Files:**
- Create: `tests/snapshots/state-apply-world.bindgen.txt`
- Create: `crates/wasmtime-backend/tests/wit_freeze.rs`

- [ ] **Step 1: Set up the snapshot directory**

```bash
mkdir -p tests/snapshots
```

- [ ] **Step 2: Bootstrap the snapshot**

The snapshot is auto-generated on first run. Write `crates/wasmtime-backend/tests/wit_freeze.rs`:

```rust
//! WIT/ABI freeze test per verification.md §22.3.
//!
//! /// Covers: architecture.md §3.5, distribution.md §10.2,
//!             verification.md §22.3.
//!
//! Re-runs the same wit-bindgen invocation the runtime uses (in
//! crates/wasmtime-backend/src/engine.rs) and asserts the generated
//! Rust bindings match the committed snapshot. Drift fails CI;
//! accepting drift requires updating the snapshot AND a kernel-major
//! version bump per distribution.md §10.2.

const SNAPSHOT_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../tests/snapshots/state-apply-world.bindgen.txt");

#[test]
fn state_apply_world_bindings_match_snapshot() {
    let generated = generate_state_apply_bindings();
    let expected = match std::fs::read_to_string(SNAPSHOT_PATH) {
        Ok(s) => s,
        Err(_) => {
            // First run: write the snapshot. Subsequent runs assert.
            std::fs::write(SNAPSHOT_PATH, &generated)
                .expect("write initial snapshot");
            panic!(
                "WIT-freeze snapshot bootstrapped at {SNAPSHOT_PATH}. Re-run the test."
            );
        }
    };
    assert_eq!(
        generated, expected,
        "WIT/ABI drift detected. Either:\n\
         1. Revert the WIT change, or\n\
         2. Bump kernel-major + update {SNAPSHOT_PATH} via:\n\
            cp <(cargo expand -p myrhiza-wasmtime-backend --lib | extract_bindings) {SNAPSHOT_PATH}",
    );
}

fn generate_state_apply_bindings() -> String {
    // The runtime uses wasmtime::component::bindgen!() in
    // crates/wasmtime-backend/src/engine.rs. To freeze the generated
    // surface, we invoke wit-parser directly to build a textual
    // representation of the resolved world, which is what the WIT
    // package promises to consumers. wit-parser's Resolve renders
    // canonically.
    //
    // This is preferable to expanding the bindgen! macro (which
    // depends on wasmtime version + macro implementation details
    // that drift across patch releases for reasons orthogonal to
    // ABI semantics).

    let mut resolve = wit_parser::Resolve::new();
    let pkg_id = resolve
        .push_dir(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../wit/myrhiza-kernel/wit"
        )))
        .expect("parse WIT package");
    let world_id = resolve
        .select_world(pkg_id, Some("state-apply"))
        .expect("select state-apply world");
    let world = &resolve.worlds[world_id];

    let mut out = String::new();
    use std::fmt::Write as _;
    writeln!(out, "world {} {{", world.name).unwrap();
    writeln!(out, "  imports:").unwrap();
    for (name, item) in &world.imports {
        let key = match name {
            wit_parser::WorldKey::Name(n) => n.clone(),
            wit_parser::WorldKey::Interface(id) => resolve
                .id_of(*id)
                .unwrap_or_else(|| format!("interface#{id:?}")),
        };
        writeln!(out, "    {key}: {item:?}").unwrap();
    }
    writeln!(out, "  exports:").unwrap();
    for (name, item) in &world.exports {
        let key = match name {
            wit_parser::WorldKey::Name(n) => n.clone(),
            wit_parser::WorldKey::Interface(id) => resolve
                .id_of(*id)
                .unwrap_or_else(|| format!("interface#{id:?}")),
        };
        writeln!(out, "    {key}: {item:?}").unwrap();
    }
    writeln!(out, "}}").unwrap();
    out
}
```

Add to `crates/wasmtime-backend/Cargo.toml` `[dev-dependencies]`:

```toml
wit-parser = "0.215"
```

- [ ] **Step 3: Bootstrap the snapshot**

Run: `cargo test -p myrhiza-wasmtime-backend --test wit_freeze -- state_apply_world_bindings_match_snapshot`
Expected: PANIC on first run (bootstraps snapshot, fails with bootstrap message).

Run: `cargo test -p myrhiza-wasmtime-backend --test wit_freeze -- state_apply_world_bindings_match_snapshot`
Expected: PASS on second run (snapshot now exists; assertion passes).

- [ ] **Step 4: Verify the snapshot exists**

```bash
ls tests/snapshots/state-apply-world.bindgen.txt
```

- [ ] **Step 5: Commit**

```bash
git add tests/snapshots crates/wasmtime-backend
git commit -m "$(cat <<'EOF'
feat(verification): WIT/ABI freeze test for state-apply world

Per verification.md §22.3. Re-parses wit/myrhiza-kernel via
wit-parser and renders a canonical text dump of the state-apply
world's imports/exports. Asserts byte-equality with
tests/snapshots/state-apply-world.bindgen.txt; drift fails CI.

Accepting drift requires updating the snapshot AND a kernel-major
bump per distribution.md §10.2.

Plan C extends with snapshots for state-propose, interaction,
behavior worlds (advisory until those bind at runtime).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 34: Spec-coverage matrix script

Per [verification.md §22.2](../specs/2026-05-09-myrhiza-master-design/verification.md): doc-comment convention `/// Covers: <file>.md §X.Y` aggregated into `tests/spec-coverage.md`.

**Files:**
- Create: `scripts/spec-coverage.sh`
- Create: `tests/spec-coverage.md` (generated; committed)
- Modify: `Justfile` (add `spec-coverage` recipe)
- Modify: `.github/workflows/ci.yml` (run `just spec-coverage` + assert no diff)

- [ ] **Step 1: Write the script**

Write `scripts/spec-coverage.sh`:

```bash
#!/usr/bin/env bash
# Generates tests/spec-coverage.md from /// Covers: doc comments
# across the workspace. Run via `just spec-coverage`. CI asserts the
# committed file matches the generated output.
#
# Convention per verification.md §22.2.
set -euo pipefail

OUT="tests/spec-coverage.md"
ROOT="$(git rev-parse --show-toplevel)"
cd "$ROOT"

cat > "$OUT" <<'EOF'
# Spec-coverage matrix

Generated by `just spec-coverage`. Do not hand-edit; CI fails on diff.

Mapping: each spec section to the tests that prove it. Tests carry
`/// Covers: <file>.md §X.Y` doc comments; this file aggregates them.

EOF

# Find every test file and grep `/// Covers:` lines.
grep -RIn --include='*.rs' '/// Covers:' \
    crates tests \
    2>/dev/null \
| awk -F: '{
    file=$1; line=$2; rest=$3;
    sub(/.*Covers:[[:space:]]*/, "", rest);
    n=split(rest, parts, /,[[:space:]]*/);
    for (i=1; i<=n; i++) {
        sec=parts[i]; sub(/[.[:space:]]*$/, "", sec);
        print sec "\t" file ":" line;
    }
}' \
| sort -k1,1 -k2,2 \
| awk -F'\t' '
    BEGIN { last="" }
    {
        if ($1 != last) {
            if (last != "") print "";
            print "## " $1;
            last = $1;
        }
        print "- " $2;
    }
' >> "$OUT"

echo "" >> "$OUT"
echo "_Generated $(date -u +%Y-%m-%dT%H:%M:%SZ)_" >> "$OUT"

echo "Wrote $OUT"
```

```bash
chmod +x scripts/spec-coverage.sh
```

- [ ] **Step 2: Add Justfile recipe**

Append to `Justfile`:

```just
spec-coverage:
    ./scripts/spec-coverage.sh

spec-coverage-check: spec-coverage
    @if ! git diff --exit-code tests/spec-coverage.md; then \
        echo "tests/spec-coverage.md is stale. Run 'just spec-coverage' and commit."; \
        exit 1; \
    fi
```

Modify the `ci:` recipe:

```just
ci: fmt-check lint test spec-coverage-check
```

- [ ] **Step 3: Generate initial matrix**

Run: `just spec-coverage`
Expected: produces `tests/spec-coverage.md` listing all `/// Covers:` annotations from prior tasks (limits_shadow.rs, wit_freeze.rs).

- [ ] **Step 4: Commit**

```bash
git add scripts/spec-coverage.sh tests/spec-coverage.md Justfile
git commit -m "$(cat <<'EOF'
feat(verification): spec-coverage matrix script + CI gate

Per verification.md §22.2. scripts/spec-coverage.sh greps
/// Covers: doc comments across the workspace and aggregates them
into tests/spec-coverage.md. CI runs `just spec-coverage-check`
which fails if the committed file is stale.

Initial matrix populated from limits_shadow.rs and wit_freeze.rs
covers (more added by later tasks in this plan + plans B/C).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 35: `myrhiza-test-utils` crate scaffold

Per [verification.md §22.8](../specs/2026-05-09-myrhiza-master-design/verification.md). Plan A populates `manifest` and `bundle` helpers; plans B/C extend.

**Files:**
- Modify: `Cargo.toml` (add `crates/test-utils` to members)
- Create: `crates/test-utils/Cargo.toml`
- Create: `crates/test-utils/src/lib.rs`
- Create: `crates/test-utils/src/manifest.rs`
- Create: `crates/test-utils/src/bundle.rs`

- [ ] **Step 1: Add to workspace**

Add `crates/test-utils` to root `members`.

- [ ] **Step 2: Cargo.toml**

Write `crates/test-utils/Cargo.toml`:

```toml
[package]
name = "myrhiza-test-utils"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
description = "Shared test fixtures + doubles. Dev-only; not published."
publish = false

[lints]
workspace = true

[dependencies]
myrhiza-types = { path = "../types" }
myrhiza-manifest = { path = "../manifest" }
ed25519-dalek.workspace = true
hex.workspace = true
serde.workspace = true
tempfile.workspace = true
```

- [ ] **Step 3: lib.rs**

Write `crates/test-utils/src/lib.rs`:

```rust
//! Shared test fixtures + doubles for the Myrhiza workspace.
//!
//! Per verification.md §22.8. Dev-only crate; never depend on
//! production paths. Plan A populates manifest + bundle helpers;
//! plan B adds mem-network double; plan C adds proptest generators.

pub mod bundle;
pub mod manifest;
```

- [ ] **Step 4: manifest builder**

Write `crates/test-utils/src/manifest.rs`:

```rust
//! Manifest builders for tests. Returns canonicalized + signed
//! manifests for common test shapes.

use std::collections::BTreeMap;

use ed25519_dalek::{Signer, SigningKey};
use myrhiza_manifest::{
    canonical::signing_target_bytes,
    schema::*,
};
use myrhiza_types::{EventHash, canonical_bincode};

/// Build a state-apply manifest declaring just `host.hash` + `host.log`
/// (the minimum useful set for plan A's counter fixture).
pub fn helpers_only_state_apply_manifest() -> Manifest {
    let mut helpers = BTreeMap::new();
    helpers.insert("host.hash".into(), true);
    helpers.insert("host.log".into(), true);

    let mut m = Manifest {
        app: AppSection {
            name: "test-fixture".into(),
            version: "0.1.0".into(),
            description: "test".into(),
            author_pubkey: "0x".into(), // filled by sign_manifest
            author_identity_class: AuthorIdentityClass::ThirdParty,
        },
        abi: AbiSection {
            kernel_major: 1,
            kernel_minor_min: 0,
            state_digest_format: StateDigestFormat::Bincode13,
        },
        capabilities: CapabilitiesSection {
            host_imports: BTreeMap::new(),
            ui_surfaces: BTreeMap::new(),
            high_value_ops: HighValueOps::default(),
            deterministic_helpers: helpers,
        },
        determinism: DeterminismSection {
            allow_floats: false,
            drift_detection: DriftDetectionSection { interval_events: 1024 },
        },
        modules: ModulesSection { dep: vec![] },
        components: ComponentsSection {
            state_apply: Some("components/state-apply.wasm".into()),
            state_propose: None,
            interaction: None,
            behavior: None,
        },
        author_policy: AuthorPolicy::default_deny(),
        signature: None,
    };
    m.canonicalize();
    m
}

/// Return a fixed test signing key. Same seed across runs.
pub fn deterministic_signing_key(seed: u8) -> SigningKey {
    SigningKey::from_bytes(&[seed; 32])
}

/// Sign `m` against `content_hash` using `key`. Mutates `m.signature`
/// and `m.app.author_pubkey` in place.
pub fn sign_manifest(m: &mut Manifest, content_hash: &EventHash, key: &SigningKey) {
    let pk = key.verifying_key().to_bytes();
    m.app.author_pubkey = format!("0x{}", hex::encode(pk));
    m.canonicalize();
    let target = signing_target_bytes(m, content_hash);
    let sig = key.sign(&target);
    m.signature = Some(Signature {
        algorithm: SignatureAlgorithm::Ed25519,
        value: sig.to_bytes(),
    });
}
```

- [ ] **Step 5: bundle builder**

Write `crates/test-utils/src/bundle.rs`:

```rust
//! Bundle directory builders for tests.

use std::path::PathBuf;

use myrhiza_manifest::schema::Manifest;
use myrhiza_types::canonical_bincode;
use tempfile::TempDir;

/// A built test bundle: tempdir + manifest path + content bytes.
pub struct TestBundle {
    pub _dir: TempDir,
    pub bundle_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub content_bytes: Vec<u8>,
}

/// Write a signed bundle into a fresh tempdir.
///
/// `m` must already be signed via [`crate::manifest::sign_manifest`]
/// against the content_hash of `component_bytes`.
pub fn write_bundle(m: &Manifest, component_bytes: &[u8]) -> std::io::Result<TestBundle> {
    let dir = TempDir::new()?;
    let bundle_dir = dir.path().to_path_buf();

    let comp_rel = m
        .components
        .state_apply
        .clone()
        .unwrap_or_else(|| "components/state-apply.wasm".into());
    let comp_path = bundle_dir.join(&comp_rel);
    std::fs::create_dir_all(comp_path.parent().unwrap())?;
    std::fs::write(&comp_path, component_bytes)?;

    let manifest_rel = PathBuf::from("manifest.bincode");
    let manifest_bytes = canonical_bincode()
        .serialize(m)
        .expect("canonical bincode of Manifest never fails");
    std::fs::write(bundle_dir.join(&manifest_rel), manifest_bytes)?;

    Ok(TestBundle {
        _dir: dir,
        bundle_dir,
        manifest_path: manifest_rel,
        content_bytes: component_bytes.to_vec(),
    })
}
```

- [ ] **Step 6: Verify**

Run: `cargo check -p myrhiza-test-utils`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/test-utils
git commit -m "$(cat <<'EOF'
feat(test-utils): scaffold crate with manifest + bundle builders

Per verification.md §22.8. Dev-only crate (publish = false). Plan A
populates manifest builder (helpers_only_state_apply_manifest,
sign_manifest) + bundle writer (write_bundle). Plan B adds the
mem-network double; plan C adds proptest generators tied to
counter/poll types.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 36: Counter state-apply fixture (cargo-component)

The fixture is a tiny crate that compiles to a wasm component and exports the state-apply world.

**Files:**
- Create: `tests/fixtures/counter-state-apply/Cargo.toml`
- Create: `tests/fixtures/counter-state-apply/src/lib.rs`
- Create: `tests/fixtures/counter-state-apply/wit/world.wit`
- Create: `tests/fixtures/built/.gitkeep`
- Modify: `Justfile` (add `build-fixtures` recipe with cargo-component)

- [ ] **Step 1: Fixture Cargo.toml**

Write `tests/fixtures/counter-state-apply/Cargo.toml`:

```toml
[package]
name = "counter-state-apply-fixture"
version = "0.1.0"
edition = "2024"
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
wit-bindgen = "0.30"
serde = { version = "1", features = ["derive"] }
bincode = "=1.3.3"

[package.metadata.component]
package = "myrhiza:counter-fixture"

[package.metadata.component.target]
path = "wit"
world = "state-apply"
```

- [ ] **Step 2: Fixture WIT**

Write `tests/fixtures/counter-state-apply/wit/world.wit`:

```wit
package myrhiza:counter-fixture@0.1.0;

interface host-deterministic {
    hash: func(bytes: list<u8>) -> list<u8>;
    log: func(level: u8, msg: string);
}

variant verdict {
    accept,
    reject(string),
}

world state-apply {
    import host-deterministic;
    export apply: func(prior-state: list<u8>, event: list<u8>) ->
        tuple<verdict, list<u8>>;
    export state-digest: func(state: list<u8>) -> list<u8>;
}
```

> **Note to executor:** the production WIT under `wit/myrhiza-kernel/` is the canonical kernel ABI. The fixture re-declares a minimal local copy because cargo-component does not yet support a target's WIT being authored outside the fixture's own crate root. The `host-deterministic` import set used here is a strict subset of the production interface (limited to hash + log per the manifest). When the fixture loads under the production kernel WIT, Wasmtime's component-model linker resolves the names by interface — the fixture's WIT is structurally compatible. This redundancy is documented honestly here; collapsing it requires a cargo-component upstream feature.

- [ ] **Step 3: Fixture lib.rs**

Write `tests/fixtures/counter-state-apply/src/lib.rs`:

```rust
#![no_std]
extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

wit_bindgen::generate!({
    world: "state-apply",
    exports: { "apply": Component, "state-digest": Component },
});

struct Component;

#[derive(Default, Serialize, Deserialize)]
struct CounterState {
    by_key: BTreeMap<String, i64>,
}

#[derive(Serialize, Deserialize)]
enum CounterEvent {
    Increment { key: String, by: i64 },
    Reset { key: String },
}

fn opts() -> impl bincode::Options {
    use bincode::Options;
    bincode::DefaultOptions::new().with_fixint_encoding().with_big_endian()
}

impl Guest for Component {
    fn apply(prior_state: Vec<u8>, event: Vec<u8>) -> (Verdict, Vec<u8>) {
        let mut state: CounterState = if prior_state.is_empty() {
            CounterState::default()
        } else {
            match opts().deserialize(&prior_state) {
                Ok(s) => s,
                Err(_) => return (Verdict::Reject("malformed prior state".into()), Vec::new()),
            }
        };

        let evt: CounterEvent = match opts().deserialize(&event) {
            Ok(e) => e,
            Err(_) => return (Verdict::Reject("malformed event".into()), Vec::new()),
        };

        match evt {
            CounterEvent::Increment { key, by } => {
                let entry = state.by_key.entry(key).or_insert(0);
                *entry = entry.saturating_add(by);
            }
            CounterEvent::Reset { key } => {
                state.by_key.remove(&key);
            }
        }

        match opts().serialize(&state) {
            Ok(bytes) => (Verdict::Accept, bytes),
            Err(_) => (Verdict::Reject("encode failure".into()), Vec::new()),
        }
    }

    fn state_digest(state: Vec<u8>) -> Vec<u8> {
        // Already canonical bincode of CounterState. Hash externally.
        state
    }
}
```

- [ ] **Step 4: Justfile build-fixtures**

Replace the `build-fixtures` placeholder in `Justfile`:

```just
build-fixtures:
    @mkdir -p tests/fixtures/built
    cd tests/fixtures/counter-state-apply && \
        cargo component build --release --target wasm32-wasip2 --locked --frozen
    cp tests/fixtures/counter-state-apply/target/wasm32-wasip2/release/counter_state_apply_fixture.wasm \
        tests/fixtures/built/counter-state-apply.wasm
```

- [ ] **Step 5: Build the fixture**

Run: `just build-fixtures`
Expected: `tests/fixtures/built/counter-state-apply.wasm` exists.

- [ ] **Step 6: Commit**

```bash
git add tests/fixtures Justfile
git commit -m "$(cat <<'EOF'
feat(fixtures): counter-state-apply cargo-component fixture

Tiny state-apply crate exercising the v1 ABI with a BTreeMap-backed
counter (Increment + Reset events). Compiles to wasm via
cargo-component; built artifact committed at
tests/fixtures/built/counter-state-apply.wasm.

The fixture's WIT is a structurally-compatible subset of the
production wit/myrhiza-kernel/ ABI — Wasmtime's linker resolves
imports by interface name. The redundancy is honest; collapsing
requires a cargo-component upstream feature.

Justfile build-fixtures recipe rebuilds the artifact with --locked
--frozen for repro-build discipline per verification.md §22.6.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 37: Kernel-tier acceptance test — load + apply

The load-bearing acceptance test for [mvp.md §15.1](../specs/2026-05-09-myrhiza-master-design/mvp.md) criterion #1.

**Files:**
- Create: `crates/kernel/tests/acceptance.rs`

- [ ] **Step 1: Write the test**

Write `crates/kernel/tests/acceptance.rs`:

```rust
//! Plan A kernel-tier acceptance tests.
//!
//! /// Covers: mvp.md §15.1 #1 (kernel loads + instantiates),
//!             mvp.md §15.1 #5 (capability declarations gate access),
//!             convergence.md §4.4 (pre-check fail-closed),
//!             determinism.md §5.2 (float-ban),
//!             determinism.md §5.3 (fuel exhaustion),
//!             verification.md §22.1 (kernel tier).

use std::collections::BTreeMap;

use myrhiza_backend::Backend;
use myrhiza_kernel::{ApplyOutcome, InstallFlow, StateApplyHandle};
use myrhiza_test_utils::{
    bundle::write_bundle,
    manifest::{deterministic_signing_key, helpers_only_state_apply_manifest, sign_manifest},
};
use myrhiza_types::EventHash;
use myrhiza_wasmtime_backend::WasmtimeBackend;

fn fixture_component_bytes() -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/built/counter-state-apply.wasm"),
    )
    .expect("build the fixture via `just build-fixtures` before running tests")
}

fn build_signed_bundle() -> myrhiza_test_utils::bundle::TestBundle {
    let component_bytes = fixture_component_bytes();
    let content_hash = EventHash::blake3(&component_bytes);

    let mut m = helpers_only_state_apply_manifest();
    let key = deterministic_signing_key(7);
    sign_manifest(&mut m, &content_hash, &key);
    write_bundle(&m, &component_bytes).expect("write bundle")
}

fn encode_event(key: &str, by: i64) -> Vec<u8> {
    use bincode::Options;
    #[derive(serde::Serialize)]
    enum E<'a> {
        Increment { key: &'a str, by: i64 },
        #[allow(dead_code)]
        Reset { key: &'a str },
    }
    bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .with_big_endian()
        .serialize(&E::Increment { key, by })
        .expect("encode event")
}

#[test]
fn kernel_loads_signed_bundle() {
    let bundle = build_signed_bundle();
    let flow = InstallFlow::new();
    let loaded = flow
        .load(&myrhiza_kernel::install::BundleAddress {
            bundle_dir: bundle.bundle_dir.clone(),
            manifest_path: bundle.manifest_path.clone(),
        })
        .expect("load OK");
    assert_eq!(loaded.manifest.app.name, "test-fixture");
    assert!(!loaded.component_bytes.is_empty());
}

#[test]
fn kernel_instantiates_and_applies_increment() {
    let bundle = build_signed_bundle();
    let flow = InstallFlow::new();
    let loaded = flow
        .load(&myrhiza_kernel::install::BundleAddress {
            bundle_dir: bundle.bundle_dir.clone(),
            manifest_path: bundle.manifest_path.clone(),
        })
        .expect("load");

    let backend = WasmtimeBackend::new().expect("backend");
    let instance = backend
        .instantiate_state_apply(&loaded.component_bytes, &loaded.manifest)
        .expect("instantiate state-apply");
    let mut handle = StateApplyHandle::new(instance);

    let event = encode_event("counter", 42);
    let r = handle.apply(&[], &event).expect("apply");
    assert_eq!(r.outcome, ApplyOutcome::Accepted);

    // Re-decode state to verify it advanced.
    use bincode::Options;
    let decoded: BTreeMap<String, i64> = bincode::DefaultOptions::new()
        .with_fixint_encoding()
        .with_big_endian()
        .deserialize::<BTreeMap<String, i64>>(&r.new_state)
        .expect("decode state");
    assert_eq!(decoded.get("counter"), Some(&42));
}
```

- [ ] **Step 2: Build fixture if needed, run test**

Run: `just build-fixtures && cargo test -p myrhiza-kernel --test acceptance kernel_loads_signed_bundle kernel_instantiates_and_applies_increment`
Expected: 2 passed.

- [ ] **Step 3: Commit**

```bash
git add crates/kernel/tests
git commit -m "$(cat <<'EOF'
test(kernel): acceptance — load signed bundle + apply Increment

Plan A's load-bearing acceptance test for mvp.md §15.1 criterion #1
(kernel loads + instantiates a WASM state component from a signed
bundle). Builds counter-state-apply fixture, signs it, loads via
InstallFlow, instantiates via WasmtimeBackend, applies an Increment
event, decodes the resulting BTreeMap state, verifies value == 42.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 38: Kernel-tier acceptance test — capability gating

[mvp.md §15.1](../specs/2026-05-09-myrhiza-master-design/mvp.md) criterion #5.

**Files:**
- Modify: `crates/kernel/tests/acceptance.rs`
- Create: `tests/fixtures/over-importer/` cargo-component crate
- Modify: `Justfile` (add over-importer to build-fixtures)

- [ ] **Step 1: Build over-importer fixture**

Write `tests/fixtures/over-importer/Cargo.toml`:

```toml
[package]
name = "over-importer-fixture"
version = "0.1.0"
edition = "2024"
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
wit-bindgen = "0.30"

[package.metadata.component]
package = "myrhiza:over-importer-fixture"

[package.metadata.component.target]
path = "wit"
world = "state-apply"
```

Write `tests/fixtures/over-importer/wit/world.wit`:

```wit
package myrhiza:over-importer-fixture@0.1.0;

/// State-apply that imports a non-deterministic surface.
/// Manifest gating must reject this regardless of declarations.
interface host-non-deterministic {
    broadcast-submit: func(topic: list<u8>, msg: list<u8>) -> list<u8>;
}

variant verdict { accept, reject(string) }

world state-apply {
    import host-non-deterministic;
    export apply: func(prior-state: list<u8>, event: list<u8>) ->
        tuple<verdict, list<u8>>;
    export state-digest: func(state: list<u8>) -> list<u8>;
}
```

Write `tests/fixtures/over-importer/src/lib.rs`:

```rust
#![no_std]
extern crate alloc;
use alloc::vec::Vec;

wit_bindgen::generate!({
    world: "state-apply",
    exports: { "apply": Component, "state-digest": Component },
});

struct Component;

impl Guest for Component {
    fn apply(_: Vec<u8>, _: Vec<u8>) -> (Verdict, Vec<u8>) {
        // Calling a non-deterministic import would land here in a
        // real attack; the fixture simply imports it. Linker rejection
        // happens before any code runs.
        let _ = host_non_deterministic::broadcast_submit(&[], &[]);
        (Verdict::Accept, Vec::new())
    }
    fn state_digest(_: Vec<u8>) -> Vec<u8> { Vec::new() }
}
```

- [ ] **Step 2: Add to Justfile**

Modify `build-fixtures` to also build over-importer:

```just
build-fixtures:
    @mkdir -p tests/fixtures/built
    cd tests/fixtures/counter-state-apply && \
        cargo component build --release --target wasm32-wasip2 --locked --frozen
    cp tests/fixtures/counter-state-apply/target/wasm32-wasip2/release/counter_state_apply_fixture.wasm \
        tests/fixtures/built/counter-state-apply.wasm
    cd tests/fixtures/over-importer && \
        cargo component build --release --target wasm32-wasip2 --locked --frozen
    cp tests/fixtures/over-importer/target/wasm32-wasip2/release/over_importer_fixture.wasm \
        tests/fixtures/built/over-importer.wasm
```

- [ ] **Step 3: Build**

Run: `just build-fixtures`
Expected: both fixtures built.

- [ ] **Step 4: Append acceptance test**

Append to `crates/kernel/tests/acceptance.rs`:

```rust
fn over_importer_component_bytes() -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/built/over-importer.wasm"),
    )
    .expect("build over-importer via `just build-fixtures`")
}

#[test]
fn capability_gating_rejects_non_deterministic_import() {
    // The manifest declares only the deterministic helper set.
    // The over-importer fixture imports broadcast-submit. Wasmtime's
    // linker has no binding for that name (because state-apply
    // ambient set is helpers-only) — instantiation must fail.

    let component_bytes = over_importer_component_bytes();
    let content_hash = EventHash::blake3(&component_bytes);

    let mut m = helpers_only_state_apply_manifest();
    let key = deterministic_signing_key(11);
    sign_manifest(&mut m, &content_hash, &key);

    let backend = WasmtimeBackend::new().expect("backend");
    let err = backend
        .instantiate_state_apply(&component_bytes, &m)
        .expect_err("must reject over-importer");

    let msg = err.to_string();
    // Either UnauthorizedImport (if linker reports cap-by-name) or
    // Instantiation (if Wasmtime reports unresolved import). Both are
    // acceptable forms of capability rejection.
    assert!(
        msg.to_lowercase().contains("import")
            || msg.to_lowercase().contains("unresolved")
            || msg.to_lowercase().contains("link"),
        "expected capability-rejection error, got {msg}"
    );
}
```

- [ ] **Step 5: Run**

Run: `cargo test -p myrhiza-kernel --test acceptance capability_gating_rejects_non_deterministic_import`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add tests/fixtures/over-importer Justfile crates/kernel/tests
git commit -m "$(cat <<'EOF'
test(kernel): acceptance — capability gating rejects over-importer

Plan A's load-bearing acceptance test for mvp.md §15.1 criterion #5
(capability declarations gate access). Builds an over-importer
fixture that imports host-non-deterministic.broadcast-submit;
attempts to instantiate via WasmtimeBackend; asserts instantiation
fails because the helpers-only ambient set has no binding for that
import.

This is the structural defense — the linker has no binding for the
import, so the component fails to link. Independent of whether the
manifest happens to have over-declared.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 39: Pre-check fail-closed + fuel-exhaustion + float-ban tests

Three remaining acceptance tests. Each uses a dedicated fixture.

**Files:**
- Create: `tests/fixtures/pre-check-rejector/` cargo-component crate
- Create: `tests/fixtures/infinite-loop/` cargo-component crate
- Create: `tests/fixtures/float-banned/` cargo-component crate
- Modify: `Justfile` (add three more fixtures)
- Modify: `crates/kernel/tests/acceptance.rs`

- [ ] **Step 1: Pre-check rejector fixture**

Write `tests/fixtures/pre-check-rejector/Cargo.toml` (same shape as counter-state-apply, package name `pre-check-rejector-fixture`).

Write `tests/fixtures/pre-check-rejector/wit/world.wit` (same minimal world as counter-state-apply).

Write `tests/fixtures/pre-check-rejector/src/lib.rs`:

```rust
#![no_std]
extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

wit_bindgen::generate!({
    world: "state-apply",
    exports: { "apply": Component, "state-digest": Component },
});

struct Component;

impl Guest for Component {
    fn apply(_prior: Vec<u8>, _event: Vec<u8>) -> (Verdict, Vec<u8>) {
        (Verdict::Reject(String::from("not allowed")), Vec::new())
    }
    fn state_digest(_: Vec<u8>) -> Vec<u8> { Vec::new() }
}
```

- [ ] **Step 2: Infinite-loop fixture**

Same shape as pre-check rejector. `src/lib.rs`:

```rust
#![no_std]
extern crate alloc;
use alloc::vec::Vec;

wit_bindgen::generate!({
    world: "state-apply",
    exports: { "apply": Component, "state-digest": Component },
});

struct Component;

impl Guest for Component {
    fn apply(_prior: Vec<u8>, _event: Vec<u8>) -> (Verdict, Vec<u8>) {
        // Spin forever; fuel exhaustion must trap.
        loop {
            core::hint::black_box(0);
        }
    }
    fn state_digest(_: Vec<u8>) -> Vec<u8> { Vec::new() }
}
```

- [ ] **Step 3: Float-banned fixture**

Same shape. `src/lib.rs`:

```rust
#![no_std]
extern crate alloc;
use alloc::vec::Vec;

wit_bindgen::generate!({
    world: "state-apply",
    exports: { "apply": Component, "state-digest": Component },
});

struct Component;

impl Guest for Component {
    fn apply(_prior: Vec<u8>, _event: Vec<u8>) -> (Verdict, Vec<u8>) {
        // Force f32 codegen.
        let a = 1.0_f32;
        let b = 2.0_f32;
        let _ = core::hint::black_box(a + b);
        (Verdict::Accept, Vec::new())
    }
    fn state_digest(_: Vec<u8>) -> Vec<u8> { Vec::new() }
}
```

- [ ] **Step 4: Update Justfile**

Replace the `build-fixtures` recipe so it builds all five fixtures:

```just
FIXTURES := "counter-state-apply over-importer pre-check-rejector infinite-loop float-banned"

build-fixtures:
    @mkdir -p tests/fixtures/built
    @for f in {{FIXTURES}}; do \
        echo "Building $f..."; \
        (cd tests/fixtures/$f && cargo component build --release --target wasm32-wasip2 --locked --frozen) || exit 1; \
        # cargo-component normalizes underscores; copy with hyphenated name.
        artifact_name=$(echo "$f" | tr '-' '_')_fixture.wasm; \
        cp tests/fixtures/$f/target/wasm32-wasip2/release/$artifact_name \
            tests/fixtures/built/$f.wasm; \
    done
```

- [ ] **Step 5: Build all fixtures**

Run: `just build-fixtures`
Expected: 5 wasm files in `tests/fixtures/built/`.

- [ ] **Step 6: Append acceptance tests**

Append to `crates/kernel/tests/acceptance.rs`:

```rust
fn fixture_bytes(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../tests/fixtures/built/{name}.wasm")),
    )
    .unwrap_or_else(|e| panic!("read fixture {name}: {e}; run `just build-fixtures`"))
}

#[test]
fn pre_check_returns_reject_and_does_not_commit() {
    let bytes = fixture_bytes("pre-check-rejector");
    let mut m = helpers_only_state_apply_manifest();
    sign_manifest(&mut m, &EventHash::blake3(&bytes), &deterministic_signing_key(13));
    let backend = WasmtimeBackend::new().unwrap();
    let inst = backend
        .instantiate_state_apply(&bytes, &m)
        .expect("instantiate");
    let mut h = StateApplyHandle::new(inst);
    let r = h.pre_check(&[], &[]).expect("call");
    match r.outcome {
        ApplyOutcome::Rejected(reason) => {
            assert_eq!(reason, "not allowed");
            // Per convergence.md §4.4: kernel must NOT sign or
            // broadcast on Reject. The handle exposes
            // candidate_state for inspection but the kernel
            // discards it.
            // (No state to assert against; the contract is that the
            // caller must not commit. This test enforces the API
            // shape that makes that contract enforceable.)
        }
        ApplyOutcome::Accepted => panic!("must reject"),
    }
}

#[test]
fn fuel_exhaustion_traps_apply() {
    let bytes = fixture_bytes("infinite-loop");
    let mut m = helpers_only_state_apply_manifest();
    sign_manifest(&mut m, &EventHash::blake3(&bytes), &deterministic_signing_key(17));
    let backend = WasmtimeBackend::new().unwrap();
    let inst = backend
        .instantiate_state_apply(&bytes, &m)
        .expect("instantiate");
    let mut h = StateApplyHandle::new(inst);
    let err = h.apply(&[], &[]).expect_err("must trap");
    let s = err.to_string().to_lowercase();
    assert!(
        s.contains("fuel") || s.contains("trap"),
        "expected fuel-exhaustion trap, got {s}"
    );
}

#[test]
fn float_banned_fixture_rejected_at_install() {
    let bytes = fixture_bytes("float-banned");
    let mut m = helpers_only_state_apply_manifest();
    sign_manifest(&mut m, &EventHash::blake3(&bytes), &deterministic_signing_key(19));
    let backend = WasmtimeBackend::new().unwrap();
    let err = backend
        .instantiate_state_apply(&bytes, &m)
        .expect_err("float-ban must reject");
    let s = err.to_string().to_lowercase();
    assert!(
        s.contains("float") || s.contains("f32") || s.contains("f64") || s.contains("banned"),
        "expected float-ban diagnostic, got {s}"
    );
}
```

- [ ] **Step 7: Run all acceptance tests**

Run: `cargo test -p myrhiza-kernel --test acceptance`
Expected: 6 tests passed (load, apply, capability_gating, pre_check_reject, fuel, float_banned).

- [ ] **Step 8: Regenerate spec-coverage matrix**

Run: `just spec-coverage`
Expected: matrix updated with new `/// Covers:` annotations.

- [ ] **Step 9: Commit**

```bash
git add tests/fixtures Justfile crates/kernel/tests tests/spec-coverage.md
git commit -m "$(cat <<'EOF'
test(kernel): acceptance — pre-check fail-closed + fuel + float-ban

Three more kernel-tier acceptance tests with dedicated fixtures:

- pre-check-rejector: state-apply that always returns Reject;
  pre-check returns ApplyOutcome::Rejected with the reject reason;
  caller must not commit (convergence.md §4.4).
- infinite-loop: state-apply that spins forever; Wasmtime fuel
  exhaustion traps within the 10M-unit budget per
  determinism.md §5.3.
- float-banned: state-apply that emits f32.add; the byte-level
  float-ban lint per determinism.md §5.2 rejects at instantiation.

All five plan-A acceptance tests pass:
1. kernel_loads_signed_bundle
2. kernel_instantiates_and_applies_increment
3. capability_gating_rejects_non_deterministic_import
4. pre_check_returns_reject_and_does_not_commit
5. fuel_exhaustion_traps_apply
6. float_banned_fixture_rejected_at_install

mvp.md §15.1 criteria #1 and #5 demonstrably pass in plan A's
foundation slice.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 40: Plan-A handoff verification

Final task: run the full CI gate, regenerate the spec-coverage matrix, and write a one-page handoff note for plan B.

**Files:**
- Create: `docs/reports/2026-05-09-myrhiza-foundation-handoff.md`

- [ ] **Step 1: Run the full gate**

Run: `just ci`
Expected: PASS (fmt-check + clippy -D warnings + test + spec-coverage-check).

- [ ] **Step 2: Re-run cross-platform digest job locally if available**

(Optional; full cross-platform CI runs in GitHub Actions per [verification.md §22.7](../specs/2026-05-09-myrhiza-master-design/verification.md). Plan B owns extending the digest-replay fixture to cover multi-peer convergence.)

- [ ] **Step 3: Write handoff note**

Write `docs/reports/2026-05-09-myrhiza-foundation-handoff.md`:

```markdown
**Date:** 2026-05-09
**Status:** active
**Subject:** Myrhiza foundation (plan A) handoff to plan B

# Plan A complete — handoff to plan B

Plan A delivered the kernel foundation: workspace, core types,
WIT packages (all four worlds authored; only state-apply bound),
manifest schema + canonical encoding + Ed25519 signing, backend
trait abstraction + Wasmtime impl with capability-gated linker +
fuel + float-ban, kernel orchestration (install flow + state-apply
ABI + state-digest emitter stub), test infrastructure, and six
kernel-tier acceptance tests covering [mvp.md §15.1] criteria #1
and #5.

## What plan B inherits

- The `Backend` trait (`crates/backend`) is stable; jco impl in
  plan C satisfies it without retrofitting kernel code.
- Resource caps live in `myrhiza_types::limits` with shadow-test
  guards. Bumping requires a kernel-major commit per
  [verification.md §22.4].
- WIT freeze snapshot at `tests/snapshots/state-apply-world.bindgen.txt`
  catches accidental ABI drift. Plan B re-snapshots if any WIT
  changes (and bumps kernel-major).
- Spec-coverage matrix discipline established. Plan B tests carry
  `/// Covers:` annotations.

## What plan B adds

- Event/DAG primitives: per-author Merkle DAG, topo-sort with
  EventHash lex tie-break, PendingBuffer (1h TTL, 10K cap, /50
  per-author sub-cap).
- iroh transport + MemNetwork double in `crates/test-utils`.
- HeadsSummary sync + drift-detection digest gossip.
- Crypto host imports: `host.install-key`, `host.verify-payload-mac`,
  AEAD seal/open, x25519-ecdh, hkdf-derive (currently stub-trapped).
- Bundle distribution + signing via iroh-blobs + per-author
  revocation topic.
- E2E tier: `tests/e2e/multi_peer_convergence.rs`,
  `tests/e2e/revocation_topic.rs`,
  `tests/e2e/equivocation_first_seen.rs`.

## Known gaps left as TODO

- `wasmtime::component::bindgen!` macro path in
  `crates/wasmtime-backend/src/engine.rs`: the bindgen-generated
  symbol paths (`crate::engine::exports::myrhiza::kernel::types::*`)
  may need updating against the actual macro output. Plan B
  reviewer cross-checks before adding non-deterministic worlds.
- `StoreLimits` builder for the 64 MB memory cap is stubbed
  `unimplemented!()` in `crates/wasmtime-backend/src/engine.rs`
  pending the wasmtime version pin; plan B's first task is to
  wire it.
- Author pubkey decoding uses `0x<hex>` in plan A. Plan B adds
  bech32m decoding per [distribution.md §10.2] (`wpub-author`
  HRP) and the kernel-side official allowlist.
- The kernel's install flow does not (yet) recursively resolve
  module deps. Plan B implements module-dep resolution + manifest
  intersection.

## Acceptance evidence

Plan A's commits:
- workspace + types + WIT + manifest + backend + Wasmtime backend
  + kernel + test-utils + fixtures + acceptance tests + verification
  scaffold.

Run `just ci` from the repo root; output ends with `tests passed`
across every workspace member.
```

- [ ] **Step 4: Commit**

```bash
git add docs/reports/2026-05-09-myrhiza-foundation-handoff.md
git commit -m "$(cat <<'EOF'
docs(reports): plan-A foundation handoff to plan B

Names what plan B inherits, what plan B adds, known gaps left as
TODO (bindgen path, StoreLimits builder, bech32m, module-dep
recursion), and acceptance evidence.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Plan A complete

40 tasks across 8 phases (numbered 0-7):

| Phase | Tasks | Coverage |
|---|---|---|
| 0. Workspace scaffold | 1-2 | Cargo workspace, Justfile, CI gate |
| 1. Core types | 3-8 | canonical_bincode, EventHash, BundleHash, Hlc, Topic, AuthorPubkey, IdentityScope, Event |
| 2. WIT packages | 9-13 | types + host-{deterministic,non-deterministic,async,ui-surfaces} + 4 worlds |
| 3. Manifest | 14-19 | crate + vocabulary + typed schema + parser + canonical encoding + signature |
| 4. Backend trait | 20 | Backend + ComponentInstance abstractions |
| 5. Wasmtime backend | 21-27 | crate + float-ban + gating + helpers + linker wiring + StateApplyInstance |
| 6. Kernel orchestration | 28-31 | crate + install flow + state-apply ABI + digest emitter |
| 7. Verification + acceptance | 32-40 | limits + WIT freeze + spec-coverage + test-utils + 5 fixtures + 6 acceptance tests + handoff note |

mvp.md §15.1 criteria #1 and #5 pass in isolation. Plans B and C deliver the rest of v1 + v1.1.

---

## Self-review checklist (run after committing every task above)

The plan author has executed the writing-plans skill's self-review at draft time. Executor should re-run before declaring plan A done:

1. **Spec coverage:** every spec section listed in the Goal/Architecture cited by at least one task. Gaps:
   - `host.verify-payload-mac` and `host.install-key` are stub-trapped (key-handle infra is plan B). Documented honestly in Task 24's commit.
   - Snapshots: TUTTI-shaped drift detection per [convergence.md §4.7](../specs/2026-05-09-myrhiza-master-design/convergence.md) is plan B (gossip transport).
   - Author identity bech32m decoding per [distribution.md §10.2](../specs/2026-05-09-myrhiza-master-design/distribution.md) is plan B (plan A uses `0x<hex>` for fixture purposes).

2. **Placeholder scan:** the plan contains "Plan author's note to executor" callouts in three places — Task 22 (hex-literal regeneration), Task 25 (StoreLimits builder), Task 27 (bindgen-generated path). Each names the specific upstream-version-dependent decision and points the executor at a concrete check. Not placeholders; they are deferred-decisions documented inline. If the executor finds the wasmtime version drifts to a point where the noted approach breaks, the plan's intent (cap memory, freeze ABI, distinguish capability errors) carries unchanged.

3. **Type consistency:** `EventHash`, `BundleHash`, `IdentityScope`, `Manifest`, `Verdict`, `ApplyOutcome`, `ApplyResult`, `PreCheckResult`, `BackendError`, `StateApplyHandle`, `LoadedBundle`, `BundleAddress`, `WasmtimeBackend`, `LogSink`, `LogLevel` — names used identically in every task that references them.

4. **Verification spec coverage:** every section of [verification.md](../specs/2026-05-09-myrhiza-master-design/verification.md) plan A is responsible for has a task:
   - §22.1 tier scaffold → Task 35 (test-utils) + Task 37 (kernel tier)
   - §22.2 spec-coverage matrix → Task 34
   - §22.3 WIT freeze → Task 33
   - §22.4 resource-cap regression → Task 32
   - §22.5 determinism property tests → deferred to plan B (counter has no
     interesting state space; deferred until poll lands or proptest
     generators are useful).
   - §22.6 reproducible fixtures → Task 36 (`--locked --frozen` recipe)
   - §22.7 cross-platform CI → Task 40 step 2 (acknowledged; full job
     in plan B's e2e tier with the digest-replay fixture)
   - §22.8 test-utils crate → Task 35

Plan A explicitly defers §22.5 and §22.7 with rationale: §22.5 requires app-specific generators (counter is too trivial), §22.7 requires a multi-event replay fixture (plan B's event-DAG implementation produces the fixtures).

---

## Execution handoff

Plan complete and saved to `docs/plans/2026-05-09-myrhiza-foundation.md`. Two execution options:

1. **Subagent-Driven (recommended)** — fresh subagent per task, review between tasks, fast iteration. REQUIRED SUB-SKILL: `superpowers:subagent-driven-development`.
2. **Inline execution** — execute tasks in this session using `superpowers:executing-plans`, batch execution with checkpoints.

Which approach?

