# E2E-1 Implementation Plan — Close in-process e2e coverage gap

> **For agentic workers:** REQUIRED SUB-SKILL: Use `subagent-driven-development` to implement this plan task-by-task. Spec at [`docs/specs/2026-05-22-e2e-test-coverage-design.md`](../specs/2026-05-22-e2e-test-coverage-design.md) is the design contract; this plan is the execution order. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the in-process portion of [reports/2026-05-21-mvp-gap-analysis.md](../reports/2026-05-21-mvp-gap-analysis.md) item 19 — wire real `IrohNetwork` through real `Runtime` through real WASM state-apply (convergence + coexistence) and add binary-shellout tests for the `myrhiza-cli` entrypoint.

**Architecture:** 12 tasks T0–T11 (renumbered after plan review found one mandatory production-code precursor). Each task is one commit. Three new test files (`crates/kernel/tests/iroh_convergence.rs`, `crates/kernel/tests/iroh_coexistence.rs`, `crates/myrhiza-cli/tests/cli_binary.rs`), one new test-utils module (`crates/test-utils/src/iroh_harness.rs`), one Cargo.toml feature addition, one Justfile recipe extension, one docs update. **Two small production-source changes are required:** (i) T0 plumbs `bootstrap: Vec<PeerPubkey>` through `Runtime::start` (the runtime hardcodes `vec![]` today per [`runtime.rs:583`](../../crates/kernel/src/runtime.rs), which prevents any two iroh peers from forming a swarm through `Runtime::start`), and (ii) T3 adds `#[derive(Clone)]` to `IrohNetwork` so the harness can pass clones to per-runtime construction.

**Tech Stack:** Existing only — Rust 2024, tokio, iroh `1.0.0-rc.0`, iroh-gossip `0.99.0`. No new production dependencies; no new dev dependencies (per spec Choice C, hand-roll `std::process::Command` rather than add `assert_cmd`).

**Common workflow per task:**
1. Dispatch implementer subagent with the task's "Files" + "Steps" sections + spec section reference.
2. After implementer completes: dispatch fresh spec-compliance reviewer.
3. After spec compliance ✅: dispatch fresh code-quality reviewer.
4. After both ✅: commit and proceed.
5. After T11 lands: dispatch a fresh final-review agent across the entire branch before opening the PR.

---

## Task T0 — Plumb `bootstrap: Vec<PeerPubkey>` through `Runtime::start`

**Spec ref:** Plan review I1; precursor required for T4–T7 to function.

**Why this is needed:** [`crates/kernel/src/runtime.rs:583`](../../crates/kernel/src/runtime.rs) hardcodes `let sub = erased.subscribe(topic, vec![]).await?;` with the inline comment "B-4.* will plumb peer-discovery into Runtime::start; for now pass an empty bootstrap." That deferred work is the blocker — with both peers subscribing on empty bootstrap, no iroh-gossip swarm ever forms. `MemNetwork::subscribe` ignores the bootstrap parameter (per [`memory.rs`](../../crates/network/src/memory.rs)), so existing MemNetwork-backed tests are unaffected.

**Files:**
- Modify: `crates/kernel/src/runtime.rs` — add 9th parameter to `Runtime::start`
- Modify: `crates/test-utils/src/harness.rs` — `InProcessHarness::spawn_peer` passes `vec![]` through
- Modify: every `Runtime::start(...)` call site in `crates/kernel/tests/*.rs` (~20 sites — `convergence.rs`, `coexistence.rs`, `attribution.rs`, `peer_authority_index.rs`, `halt_detection.rs`, etc.) — append `vec![]` as the new last arg

**Steps:**

- [ ] **T0.1**: Edit `crates/kernel/src/runtime.rs`. Locate `pub async fn start<N: Network>(` at line 569. Currently:
  ```rust
  pub async fn start<N: Network>(
      network: N,
      topic: Topic,
      app_bundle_hash: BundleHash,
      topic_name: String,
      handle: StateApplyHandle,
      peer_key: PeerKeypair,
      author_key: Option<AuthorKeypair>,
      cfg: RuntimeCfg,
  ) -> Result<RuntimeHandle, RuntimeError> {
  ```
  Add `bootstrap: Vec<PeerPubkey>` as the new 9th parameter (after `cfg`):
  ```rust
  pub async fn start<N: Network>(
      network: N,
      topic: Topic,
      app_bundle_hash: BundleHash,
      topic_name: String,
      handle: StateApplyHandle,
      peer_key: PeerKeypair,
      author_key: Option<AuthorKeypair>,
      cfg: RuntimeCfg,
      bootstrap: Vec<PeerPubkey>,
  ) -> Result<RuntimeHandle, RuntimeError> {
  ```

- [ ] **T0.2**: Locate `let sub = erased.subscribe(topic, vec![]).await?;` at line 583. Replace `vec![]` with `bootstrap`:
  ```rust
  let sub = erased.subscribe(topic, bootstrap).await?;
  ```
  Remove the surrounding "B-4.* will plumb peer-discovery..." comment (it's now resolved).

- [ ] **T0.3**: Imports at the top of `runtime.rs` likely already pull in `PeerPubkey` (used as `signed_by_peer` field); verify by grep. If not, add `use myrhiza_types::PeerPubkey;` to the existing imports.

- [ ] **T0.4**: Run `cargo check -p myrhiza-kernel`. Expected: this fails at every test-side call site (~20 errors of the form "this function takes 9 arguments but 8 were supplied"). This is the signal to proceed with the mechanical update in T0.5.

- [ ] **T0.5**: For each compile error, append `, vec![]` as the new last argument to the failing `Runtime::start(...)` call. Use `cargo check -p myrhiza-kernel --tests --all-targets 2>&1 | grep -E '\-\->.*\.rs:[0-9]+' | sort -u` to enumerate. Suggested order: `convergence.rs`, `coexistence.rs`, `attribution.rs`, `peer_authority_index.rs`, `halt_detection.rs`. Also update `crates/test-utils/src/harness.rs::InProcessHarness::spawn_peer` line 351 (the `Runtime::start(net, ...)` call) to append `vec![]`.

- [ ] **T0.6**: Run `cargo check -p myrhiza-kernel --tests --all-targets`. Expect: clean.

- [ ] **T0.7**: Run `cargo test -p myrhiza-kernel --tests`. Expect: all existing tests still pass — no behavior change for MemNetwork (it ignores bootstrap).

- [ ] **T0.8**: Run `cargo test -p myrhiza-kernel --features network-iroh --tests` to validate the iroh-feature path still compiles after the signature change. (No new iroh tests yet — T4–T7 add them.) Expect: pass.

- [ ] **T0.9**: Run `cargo clippy --workspace --all-targets -- -D warnings`. Fix any new lint findings.

- [ ] **T0.10**: Commit. Message:
  ```
  feat(kernel): E2E-1 T0 — plumb bootstrap parameter through Runtime::start
  ```
  Body: cite [`runtime.rs:582-583`](../../crates/kernel/src/runtime.rs) original comment; explain MemNetwork-call-site preservation via `vec![]`; cite plan review I1.

---

## Task T1 — Add `network-iroh` feature to `myrhiza-test-utils`

**Spec ref:** §3.2, Choice A.

**Files:**
- Modify: `crates/test-utils/Cargo.toml` — add `network-iroh` feature; add optional iroh/iroh-gossip deps
- Modify: `crates/test-utils/src/lib.rs` — declare `iroh_harness` module gated on the feature; add gated re-exports

**Steps:**

- [ ] **T1.1**: Edit `crates/test-utils/Cargo.toml`. Under `[dependencies]`, add:
  ```toml
  iroh = { workspace = true, optional = true }
  iroh-gossip = { workspace = true, optional = true }
  ```
  Add a new `[features]` section:
  ```toml
  [features]
  # Iroh-backed test fixtures (IrohHarness) for kernel-tier acceptance
  # tests that need a real `IrohNetwork` under a real `Runtime`. Default
  # off; mirrors the `network-iroh` feature on `myrhiza-network`. Per
  # docs/specs/2026-05-22-e2e-test-coverage-design.md §3.2.
  network-iroh = [
      "myrhiza-network/network-iroh",
      "dep:iroh",
      "dep:iroh-gossip",
  ]
  ```

- [ ] **T1.2**: Edit `crates/test-utils/src/lib.rs`. After `pub mod manifest;`, add:
  ```rust
  #[cfg(feature = "network-iroh")]
  pub mod iroh_harness;
  ```

- [ ] **T1.3**: Create the empty module file `crates/test-utils/src/iroh_harness.rs`:
  ```rust
  //! Iroh-backed multi-peer test harness for kernel-tier acceptance
  //! tests. Mirrors the shape of `InProcessHarness` (MemNetwork) but
  //! wires `Runtime::start` to a real `IrohNetwork` over loopback UDP
  //! via a shared `iroh::address_lookup::MemoryLookup`.
  //!
  //! Per docs/specs/2026-05-22-e2e-test-coverage-design.md §3.2.

  #![cfg(feature = "network-iroh")]
  ```

- [ ] **T1.4**: Run `cargo check -p myrhiza-test-utils --features network-iroh`. Expect: clean compile, no warnings.

- [ ] **T1.5**: Run `cargo check -p myrhiza-test-utils` (no feature). Expect: clean compile — feature gate guarantees iroh deps stay optional.

- [ ] **T1.6**: Commit. Message:
  ```
  feat: E2E-1 T1 — add network-iroh feature to myrhiza-test-utils
  ```
  Body cites spec §3.2.

---

## Task T2 — Add `spawn_iroh_peer` + `IrohPeerStack` to test-utils

**Spec ref:** §3.2 (`IrohPeerStack`), Choice A (~30 LOC duplication accepted).

**Source to mirror:** [`crates/network/tests/direct_streams_iroh.rs:62-90`](../../crates/network/tests/direct_streams_iroh.rs). **Do NOT modify the source file.** Copy the shape into the new module.

**Files:**
- Modify: `crates/test-utils/src/iroh_harness.rs`

**Steps:**

- [ ] **T2.1**: Open `crates/test-utils/src/iroh_harness.rs`. Add imports (note: `HEADS_REQUEST_ALPN` is already re-exported at the `myrhiza-network` crate root per [`lib.rs:29-30`](../../crates/network/src/lib.rs), so use the short path):
  ```rust
  use iroh::address_lookup::MemoryLookup;
  use myrhiza_network::{HEADS_REQUEST_ALPN, IrohNetwork};
  ```

- [ ] **T2.2**: Add the `IrohPeerStack` struct:
  ```rust
  /// One iroh peer's complete stack: endpoint, gossip handle, router,
  /// and the IrohNetwork. Ownership lives on the harness so endpoints
  /// are not dropped mid-test (dropping the endpoint tears down the
  /// UDP socket and silently breaks every running peer).
  ///
  /// Fields are pub for the rare test that needs to reach below the
  /// harness API (e.g. publishing raw bytes through `gossip` to
  /// exercise decode failure). Prefer the harness API where it suffices.
  pub struct IrohPeerStack {
      pub endpoint: iroh::Endpoint,
      pub gossip: iroh_gossip::Gossip,
      pub router: iroh::protocol::Router,
      pub network: IrohNetwork,
  }
  ```

- [ ] **T2.3**: Add `spawn_iroh_peer`. This is the ~30 LOC duplication from `direct_streams_iroh.rs:66`, **extended with an `iroh_secret: Option<[u8; 32]>` parameter** so the harness can align iroh's endpoint identity with the kernel's `PeerKeypair` (load-bearing for direct-stream backfill; see T3 prep step T3.0).
  ```rust
  /// Spin up a fresh iroh endpoint + gossip + router for a test peer.
  ///
  /// The caller provides a shared `MemoryLookup` — each peer's `addr()`
  /// is registered into it after bind so that bootstrap by `EndpointId`
  /// resolves to a real socket address. Without this, `endpoint.connect`
  /// has no addressing information and the dial silently times out.
  ///
  /// If `iroh_secret` is `Some(bytes)`, the endpoint is constructed with
  /// that Ed25519 secret. The kernel's `PeerKeypair::deterministic(seed)`
  /// derives `secret = SigningKey::from_bytes(seed.to_le_bytes()-padded
  /// to 32 bytes)`. Passing the same bytes here makes
  /// `network.peer_pubkey() == peer_key.public`, which is required for
  /// kernel-issued `request_heads(target, ...)` to dial the correct iroh
  /// endpoint (the target identifier comes from `signed_by_peer` in a
  /// HeadsSummary, which is `peer_key.public`).
  ///
  /// If `register_heads_alpn` is true, the Router also accepts
  /// `HEADS_REQUEST_ALPN` against `network.protocol_handler()`. Kernel-
  /// tier tests always need this, so the IrohHarness always passes true.
  ///
  /// Mirrors `crates/network/tests/direct_streams_iroh.rs::spawn_iroh_peer`.
  /// Duplication accepted per spec §2 Choice A (avoiding `network →
  /// test-utils` dev-dep cycle).
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
      let endpoint = endpoint_builder
          .bind()
          .await
          .expect("iroh endpoint bind");
      lookup.add_endpoint_info(endpoint.addr());
      let gossip = iroh_gossip::Gossip::builder().spawn(endpoint.clone());
      let network = IrohNetwork::new(endpoint.clone(), gossip.clone());
      let mut builder = iroh::protocol::Router::builder(endpoint.clone())
          .accept(iroh_gossip::ALPN, gossip.clone());
      if register_heads_alpn {
          builder = builder.accept(HEADS_REQUEST_ALPN, network.protocol_handler());
      }
      let router = builder.spawn();
      IrohPeerStack {
          endpoint,
          gossip,
          router,
          network,
      }
  }
  ```
  (Note: the source file returns a 4-tuple and does not take `iroh_secret`; we wrap into the named struct because the harness owns them collectively, and we add `iroh_secret` because the kernel-tier tests need identity alignment. Verify `iroh::SecretKey::from_bytes` exists with the `&[u8; 32]` signature per [`/home/intendednull/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iroh-1.0.0-rc.0/src/endpoint.rs:505`](file:///home/intendednull/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iroh-1.0.0-rc.0/src/endpoint.rs); the API is `SecretKey::from_bytes(&[u8; 32])`.)

- [ ] **T2.4**: Run `cargo build -p myrhiza-test-utils --features network-iroh --tests`. Expect: clean compile, zero warnings.

- [ ] **T2.5**: Run `cargo clippy -p myrhiza-test-utils --features network-iroh --all-targets -- -D warnings`. Fix any lint findings.

- [ ] **T2.6**: Commit. Message:
  ```
  feat: E2E-1 T2 — IrohPeerStack + spawn_iroh_peer helper in test-utils
  ```
  Body notes spec §3.2 + Choice A; cite source line in `direct_streams_iroh.rs`.

---

## Task T3 — `IrohHarness::new` + `spawn_peer`

**Spec ref:** §3.2 (the harness struct), §3.6 (load-bearing detail on heads-ALPN registration). Plan review I1 (bootstrap), B1/B5 (identity alignment).

**Pre-task investigation (T3.0):** verify that the iroh endpoint's identity must align with the kernel's `peer_key.public`. Quick audit: `crates/kernel/src/runtime.rs` line 1182, 1250, 1261, 1832, 1871 all compare HeadsSummary's `signed_by_peer` against `self.peer_key.public`. When a kernel issues `request_heads(target_peer_pubkey, ...)`, the iroh transport must dial that pubkey as an iroh endpoint ID. Therefore: **iroh endpoint identity MUST equal the kernel's `peer_key.public`** for direct-stream backfill (T6) to work. `PeerKeypair::deterministic(seed)` derives `secret = SigningKey::from_bytes(bytes)` where `bytes[..8] = seed.to_le_bytes(); bytes[8..] = [0; 24]` (per [`crates/kernel/src/identity/mod.rs:61-65`](../../crates/kernel/src/identity/mod.rs)). We can recompute these bytes inside the harness and pass them as `iroh_secret` to `spawn_iroh_peer` — no new accessor on PeerKeypair needed.

**Files:**
- Modify: `crates/test-utils/src/iroh_harness.rs`
- Modify: `crates/test-utils/src/harness.rs` — `pub(crate)` accessor on `PeerHandle::from_runtime`
- Modify: `crates/network/src/iroh_transport.rs` — `#[derive(Clone)]` on `IrohNetwork` (load-bearing — see T3.5)

**Steps:**

- [ ] **T3.1**: Add imports to `iroh_harness.rs`:
  ```rust
  use myrhiza_kernel::identity::{AuthorKeypair, PeerKeypair};
  use myrhiza_kernel::runtime::{Runtime, RuntimeCfg, RuntimeHandle};
  use myrhiza_kernel::state_apply::StateApplyHandle;
  use myrhiza_types::{BundleHash, PeerPubkey, Topic};

  use crate::harness::PeerHandle;
  ```

- [ ] **T3.2**: `PeerHandle` is in a sibling module. Make its constructor reachable from `iroh_harness` by adding a crate-visible constructor in `crates/test-utils/src/harness.rs`. Find the existing `PeerHandle { runtime }` literal construction at the end of `InProcessHarness::spawn_peer` and replace it with a call to a new `pub(crate) fn from_runtime(runtime: RuntimeHandle) -> Self { Self { runtime } }`. Update `InProcessHarness::spawn_peer` to call `PeerHandle::from_runtime(runtime)`.
  Concretely, in `harness.rs` impl block for `PeerHandle`, add:
  ```rust
  pub(crate) fn from_runtime(runtime: RuntimeHandle) -> Self {
      Self { runtime }
  }
  ```
  And in `InProcessHarness::spawn_peer`, replace `PeerHandle { runtime }` with `PeerHandle::from_runtime(runtime)`.

- [ ] **T3.3**: Add the `IrohHarness` struct to `iroh_harness.rs`:
  ```rust
  /// Multi-peer fixture for iroh-backed convergence + coexistence tests.
  ///
  /// Owns the shared `MemoryLookup` so each spawned peer's address is
  /// discoverable by every other peer. Peer stacks are owned by the
  /// harness; dropping the harness tears them all down together,
  /// avoiding the "endpoint died mid-test" hazard from manual
  /// lifecycle management.
  ///
  /// Constructor difference from `InProcessHarness`: there is no
  /// `bus_capacity` arg — iroh has no bus. Otherwise the field set
  /// matches `InProcessHarness` exactly so test bodies remain
  /// near-identical between MemNetwork and IrohNetwork variants.
  pub struct IrohHarness {
      pub lookup: MemoryLookup,
      pub app_bundle_hash: BundleHash,
      pub topic_name: String,
      pub seed: [u8; 32],
      pub topic: Topic,
      peers: Vec<IrohPeerStack>,
  }

  impl IrohHarness {
      /// Construct a fresh harness with a private `MemoryLookup`.
      /// Bundle hash + topic name are fixed at construction to match
      /// `InProcessHarness::new`'s defaults so test bodies stay
      /// near-identical between MemNetwork and IrohNetwork variants.
      #[must_use]
      pub fn new(seed: [u8; 32]) -> Self {
          let lookup = MemoryLookup::default();
          let app_bundle_hash = BundleHash::from_bytes([0xAB; 32]);
          let topic_name = "main".to_string();
          let topic = Topic::derive(&app_bundle_hash, &seed, &topic_name);
          Self {
              lookup,
              app_bundle_hash,
              topic_name,
              seed,
              topic,
              peers: Vec::new(),
          }
      }
  }
  ```

- [ ] **T3.4**: Add the `spawn_peer` method inside `impl IrohHarness`:
  ```rust
  /// Spawn a peer with the given identity seeds. `bootstrap` is the
  /// pubkey of an already-spawned peer this one should dial; pass an
  /// empty vec for the first peer (it waits for inbound joins).
  ///
  /// Internally derives the iroh endpoint's secret key from the same
  /// `peer_seed` so the endpoint identity equals `peer_key.public` —
  /// required for direct-stream backfill since kernel-issued
  /// `request_heads(target, ...)` dials `target` as an iroh endpoint id,
  /// where `target` is `peer_key.public` from a peer HeadsSummary
  /// signature (see T3.0 prep).
  ///
  /// Always registers the heads-request ALPN on every peer because
  /// kernel-tier tests rely on `Runtime`'s `install_request_handler`
  /// call to wire the responder (spec §3.2 load-bearing detail).
  ///
  /// # Panics
  /// Panics if `Runtime::start` fails. The iroh subscribe path can
  /// fail in principle (e.g. invalid bootstrap pubkey), but every
  /// test fixture passes well-formed bootstrap data; a panic here
  /// is a test-infrastructure bug, not a runtime error.
  #[allow(clippy::expect_used)]
  pub async fn spawn_peer(
      &mut self,
      peer_seed: u64,
      author_seed: Option<u64>,
      handle: StateApplyHandle,
      cfg: RuntimeCfg,
      bootstrap: Vec<PeerPubkey>,
  ) -> PeerHandle {
      // Recompute the same seed bytes that `PeerKeypair::deterministic`
      // uses internally (crates/kernel/src/identity/mod.rs:61). Both
      // PeerKeypair and the iroh endpoint then derive from the same
      // Ed25519 secret, so `network.peer_pubkey() == peer_key.public`.
      let mut iroh_secret_bytes = [0u8; 32];
      iroh_secret_bytes[..8].copy_from_slice(&peer_seed.to_le_bytes());

      let stack = spawn_iroh_peer(&self.lookup, Some(iroh_secret_bytes), true).await;
      let peer_key = PeerKeypair::deterministic(peer_seed);
      let author_key = author_seed.map(AuthorKeypair::deterministic);

      // Clone the IrohNetwork handle out of the stack so we can pass
      // it to Runtime::start. The original is retained on the stack
      // for cleanup ordering. Clone is structurally sound — see T3.5.
      let network = stack.network.clone();

      let runtime = Runtime::start(
          network,
          self.topic,
          self.app_bundle_hash,
          self.topic_name.clone(),
          handle,
          peer_key,
          author_key,
          cfg,
          bootstrap,
      )
      .await
      .expect("Runtime::start (iroh)");

      self.peers.push(stack);
      PeerHandle::from_runtime(runtime)
  }
  ```
  Note the 9th argument to `Runtime::start` is `bootstrap` — added in T0. Without T0 this won't compile.

- [ ] **T3.5**: Open `crates/network/src/iroh_transport.rs`. Locate `pub struct IrohNetwork {` at line 63. Add `#[derive(Clone)]` directly above the struct keyword. All four fields are cheap-clone (`iroh::Endpoint` is Arc-backed, `iroh_gossip::Gossip` Clones, `PeerPubkey` is Copy, `Arc<Mutex<Option<_>>>` Clones). The derive intentionally shares request-handler state across clones, matching the comment at lines 73-76 ("...so that protocol-handler clones returned from `protocol_handler()` share state with this instance").

- [ ] **T3.6**: Add a confirmatory smoke assertion to the harness (NOT a full smoke test — T4 is the first real integration). At the bottom of `iroh_harness.rs`, add:
  ```rust
  #[cfg(test)]
  #[allow(clippy::expect_used, clippy::unwrap_used)]
  mod tests {
      use super::*;

      /// Sanity: two `IrohPeerStack`s spawned with different `iroh_secret`
      /// values resolve to distinct peer pubkeys, and identity-aligned
      /// `iroh_secret` produces matching `network.peer_pubkey()` and
      /// `PeerKeypair::deterministic(seed).public`. This is a structural
      /// check (no Runtime, no swarm), so it stays fast.
      #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
      async fn iroh_secret_aligns_network_pubkey_with_peer_key() {
          use myrhiza_kernel::identity::PeerKeypair;
          let lookup = MemoryLookup::default();
          let seed = 7_u64;
          let mut bytes = [0u8; 32];
          bytes[..8].copy_from_slice(&seed.to_le_bytes());
          let stack = spawn_iroh_peer(&lookup, Some(bytes), true).await;
          let pk = PeerKeypair::deterministic(seed);
          assert_eq!(
              stack.network.peer_pubkey(),
              pk.public,
              "iroh endpoint identity must equal PeerKeypair::deterministic(seed).public",
          );
      }
  }
  ```
  This is faster than spawning a full Runtime and validates the load-bearing identity-alignment property in isolation. Full integration validation lives in T4.

- [ ] **T3.7**: Run `cargo test -p myrhiza-test-utils --features network-iroh --tests iroh_harness::tests::iroh_secret_aligns_network_pubkey_with_peer_key`. Expect: pass.

- [ ] **T3.8**: Run `cargo clippy -p myrhiza-test-utils --features network-iroh --all-targets -- -D warnings`. Fix lints.

- [ ] **T3.9**: Commit. Message:
  ```
  feat: E2E-1 T3 — IrohHarness + identity-aligned spawn_peer + IrohNetwork Clone
  ```
  Body cites spec §3.2, plan review B1+B5, and explains the identity-alignment rationale.

---

## Task T4 — `iroh_single_originator_single_receiver_converges`

**Spec ref:** §3.3 row 1.

**Files:**
- Create: `crates/kernel/tests/iroh_convergence.rs`
- Modify: `crates/kernel/Cargo.toml` — flip the `network-iroh` feature to also activate `myrhiza-test-utils/network-iroh`

**Steps:**

- [ ] **T4.1**: Edit `crates/kernel/Cargo.toml`. The `[features]` section currently has:
  ```toml
  network-iroh = [
      "myrhiza-network/network-iroh",
      "dep:iroh",
      "dep:iroh-gossip",
  ]
  ```
  Add `"myrhiza-test-utils/network-iroh"` to the list:
  ```toml
  network-iroh = [
      "myrhiza-network/network-iroh",
      "myrhiza-test-utils/network-iroh",
      "dep:iroh",
      "dep:iroh-gossip",
  ]
  ```

- [ ] **T4.2**: Create `crates/kernel/tests/iroh_convergence.rs`. Top of file:
  ```rust
  //! Cross-peer convergence over real IrohNetwork — closes the in-process
  //! portion of [reports/2026-05-21-mvp-gap-analysis.md] item 19.
  //!
  //! Mirrors `crates/kernel/tests/convergence.rs` but routes through a
  //! real iroh-gossip swarm in-process. See spec §3.3.

  #![cfg(feature = "network-iroh")]
  #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

  use std::collections::BTreeSet;
  use std::time::Duration;

  use bincode::Options;
  use myrhiza_kernel::identity::AuthorKeypair;
  use myrhiza_kernel::pending::PendingCfg;
  use myrhiza_kernel::runtime::RuntimeCfg;
  use myrhiza_test_utils::iroh_harness::IrohHarness;
  use myrhiza_types::{GenesisV1, canonical_bincode};

  mod helpers;

  fn fast_cfg() -> RuntimeCfg {
      RuntimeCfg {
          drift_interval: 1,
          drift_min_interval: Duration::from_secs(0),
          drift_daily_cap: u32::MAX,
          heads_summary_tick: Duration::from_millis(100),
          pending_cfg: PendingCfg::default(),
          broadcast_capacity: 256,
          kernel_fuel_table_version: 1,
          drift_stash_cap: 256,
          transport_error_halt_threshold: 5,
      }
  }
  ```

- [ ] **T4.3**: Add the test:
  ```rust
  /// Covers: mvp.md §15.1 #2, spec §3.3 row 1.
  /// Mirrors `convergence.rs::single_originator_single_receiver_converges`
  /// but routes events through real IrohNetwork (loopback UDP, iroh-gossip
  /// Plumtree forwarding) rather than MemNetwork's in-memory bus.
  #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
  async fn iroh_single_originator_single_receiver_converges() {
      let mut harness = IrohHarness::new([0x11; 32]);
      let cfg = fast_cfg();

      let peer_a = harness
          .spawn_peer(1, Some(1), helpers::counter_handle(), cfg.clone(), vec![])
          .await;
      let peer_a_pk = harness.peers[0].network.peer_pubkey();
      let mut peer_b = harness
          .spawn_peer(2, None, helpers::counter_handle(), cfg, vec![peer_a_pk])
          .await;

      // Allow the iroh-gossip swarm a moment to form before peer A starts
      // publishing. Without this, Plumtree may drop the first event because
      // B's join is still in flight (see iroh_gossip.rs:133 for the
      // empirical 200ms convention).
      tokio::time::sleep(Duration::from_millis(300)).await;

      let kp_a = AuthorKeypair::deterministic(1);
      let initial = 0_i64.to_be_bytes().to_vec();
      let genesis_payload = GenesisV1 {
          seed: harness.seed,
          founder_pubkey: kp_a.author,
          app_payload: initial,
      };
      let genesis_bytes = canonical_bincode()
          .serialize(&genesis_payload)
          .expect("encode genesis payload");
      peer_a
          .author(genesis_bytes, BTreeSet::new())
          .await
          .expect("genesis");

      for delta in [1_i64, 2, -1] {
          peer_a
              .author(delta.to_be_bytes().to_vec(), BTreeSet::new())
              .await
              .expect("increment");
      }

      let expected_state = 2_i64.to_be_bytes().to_vec();
      assert!(
          peer_b
              .await_digest(expected_state.clone(), Duration::from_secs(10))
              .await,
          "peer B must converge to state {expected_state:?} over real iroh"
      );
  }
  ```

- [ ] **T4.4**: `IrohHarness::peers` is currently private. Either make it `pub` (test convenience) or add an accessor `pub fn peer_pubkey(&self, index: usize) -> PeerPubkey`. Choose the latter — it's the smaller surface; only the pubkey is needed externally. Add to `IrohHarness` in `iroh_harness.rs`:
  ```rust
  /// Pubkey of the i-th peer spawned via `spawn_peer`. Panics if
  /// `index` is out of range.
  #[must_use]
  #[allow(clippy::expect_used)]
  pub fn peer_pubkey(&self, index: usize) -> PeerPubkey {
      self.peers
          .get(index)
          .expect("peer index out of range")
          .network
          .peer_pubkey()
  }
  ```
  Then update T4.3 to use `harness.peer_pubkey(0)` instead of `harness.peers[0].network.peer_pubkey()`.

- [ ] **T4.5**: The `mod helpers;` line in `iroh_convergence.rs` points at the existing `crates/kernel/tests/helpers/mod.rs` — Cargo's integration-test discovery requires the helpers module be reachable from each integration test file. Verify by reading `crates/kernel/tests/convergence.rs:9` which does `mod helpers;` the same way; the file `crates/kernel/tests/helpers/mod.rs` lives in `tests/helpers/`. The `mod helpers;` declaration in `iroh_convergence.rs` resolves there.

- [ ] **T4.6**: Run `cargo test -p myrhiza-kernel --features network-iroh --test iroh_convergence iroh_single_originator_single_receiver_converges`. Expect: pass within ~10s.

- [ ] **T4.7**: Run `cargo clippy -p myrhiza-kernel --features network-iroh --all-targets -- -D warnings`. Fix lints.

- [ ] **T4.8**: Commit. Message:
  ```
  feat: E2E-1 T4 — iroh_single_originator convergence test (real IrohNetwork)
  ```
  Body cites spec §3.3.

---

## Task T5 — `iroh_concurrent_multi_author_converges`

**Spec ref:** §3.3 row 2 — flake-sensitive, use 500ms settle.

**Files:**
- Modify: `crates/kernel/tests/iroh_convergence.rs`

**Steps:**

- [ ] **T5.1**: Append to `iroh_convergence.rs`:
  ```rust
  /// Covers: mvp.md §15.1 #2, convergence.md §4.1, spec §3.3 row 2.
  /// Mirrors `convergence.rs::concurrent_multi_author_converges`. The
  /// 500ms pre-publish settle (vs 200ms in single-originator) matches
  /// the three-peer settle in iroh_gossip.rs:172 — concurrent authoring
  /// during gossip warm-up is the most flake-prone path.
  #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
  async fn iroh_concurrent_multi_author_converges() {
      let mut harness = IrohHarness::new([0x22; 32]);
      let cfg = fast_cfg();

      let mut peer_a = harness
          .spawn_peer(1, Some(1), helpers::counter_handle(), cfg.clone(), vec![])
          .await;
      let peer_a_pk = harness.peer_pubkey(0);
      let mut peer_b = harness
          .spawn_peer(2, Some(2), helpers::counter_handle(), cfg, vec![peer_a_pk])
          .await;

      // Give the swarm time to settle before any author event. Three-peer
      // case uses 500ms (iroh_gossip.rs:172); this two-peer concurrent
      // case is comparably timing-sensitive because both peers publish
      // during the warm-up window.
      tokio::time::sleep(Duration::from_millis(500)).await;

      // Peer A authors genesis (founder = A).
      let kp_a = AuthorKeypair::deterministic(1);
      let genesis = GenesisV1 {
          seed: harness.seed,
          founder_pubkey: kp_a.author,
          app_payload: 0_i64.to_be_bytes().to_vec(),
      };
      let g_bytes = canonical_bincode().serialize(&genesis).expect("encode");
      peer_a
          .author(g_bytes, BTreeSet::new())
          .await
          .expect("genesis");

      // Wait up to 10s for B to ingest genesis before B authors.
      let initial_state = 0_i64.to_be_bytes().to_vec();
      assert!(
          peer_b
              .await_digest(initial_state, Duration::from_secs(10))
              .await,
          "peer B must ingest genesis before concurrent authoring begins"
      );

      // Concurrent authoring: A authors +1 and +2; B authors +10 and +20.
      // Canonical topo-sort yields 0 + 1 + 2 + 10 + 20 = 33 on both peers.
      for delta in [1_i64, 2] {
          peer_a
              .author(delta.to_be_bytes().to_vec(), BTreeSet::new())
              .await
              .expect("a inc");
      }
      for delta in [10_i64, 20] {
          peer_b
              .author(delta.to_be_bytes().to_vec(), BTreeSet::new())
              .await
              .expect("b inc");
      }

      let expected_state = 33_i64.to_be_bytes().to_vec();
      assert!(
          peer_a
              .await_digest(expected_state.clone(), Duration::from_secs(10))
              .await,
          "peer A must converge to state {expected_state:?} over real iroh"
      );
      assert!(
          peer_b
              .await_digest(expected_state.clone(), Duration::from_secs(10))
              .await,
          "peer B must converge to state {expected_state:?} over real iroh"
      );
  }
  ```

- [ ] **T5.2**: Run `cargo test -p myrhiza-kernel --features network-iroh --test iroh_convergence iroh_concurrent_multi_author_converges`. Expect: pass within ~15s.

- [ ] **T5.3**: Run with `--release` once to validate timing under optimized builds: `cargo test --release -p myrhiza-kernel --features network-iroh --test iroh_convergence`. Expect: pass.

- [ ] **T5.4**: Commit. Message:
  ```
  feat: E2E-1 T5 — iroh_concurrent_multi_author convergence test
  ```

---

## Task T6 — `iroh_late_joiner_backfills_via_heads_summary`

**Spec ref:** §3.3 row 3 — validates the heads-ALPN load-bearing detail from §3.2.

**Files:**
- Modify: `crates/kernel/tests/iroh_convergence.rs`

**Steps:**

- [ ] **T6.1**: Append to `iroh_convergence.rs`:
  ```rust
  /// Covers: mvp.md §15.1 #2, convergence.md §4.2, spec §3.3 row 3.
  /// Mirrors `convergence.rs::late_joiner_backfills_via_heads_summary`.
  /// Validates the Runtime-issued backfill path end-to-end over real
  /// iroh: late-joining B observes a `HeadsSummary` from A's
  /// `heads_summary_tick`, issues `request_heads` over real iroh, and
  /// catches up via direct-stream backfill (B-4.4/4.5).
  #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
  async fn iroh_late_joiner_backfills_via_heads_summary() {
      let mut harness = IrohHarness::new([0x33; 32]);
      let cfg = fast_cfg();
      let peer_a = harness
          .spawn_peer(1, Some(1), helpers::counter_handle(), cfg.clone(), vec![])
          .await;

      // A authors genesis + 5 increments BEFORE B joins.
      let kp_a = AuthorKeypair::deterministic(1);
      let genesis = GenesisV1 {
          seed: harness.seed,
          founder_pubkey: kp_a.author,
          app_payload: 0_i64.to_be_bytes().to_vec(),
      };
      let g_bytes = canonical_bincode().serialize(&genesis).expect("encode");
      peer_a
          .author(g_bytes, BTreeSet::new())
          .await
          .expect("genesis");
      for delta in [1_i64, 1, 1, 1, 1] {
          peer_a
              .author(delta.to_be_bytes().to_vec(), BTreeSet::new())
              .await
              .expect("inc");
      }

      // Now B joins. Its bootstrap is A's pubkey so it dials A
      // immediately and joins A's iroh-gossip swarm.
      let peer_a_pk = harness.peer_pubkey(0);
      let mut peer_b = harness
          .spawn_peer(2, None, helpers::counter_handle(), cfg, vec![peer_a_pk])
          .await;

      // Expected: 0 + 5*1 = 5. The path is: A's
      // `heads_summary_tick` fires → HeadsSummary published → B sees gap
      // (its DAG has nothing for A) → B issues `request_heads` over real
      // iroh direct-stream → A's installed `KernelRequestHandler`
      // responds with all 6 events → B applies them.
      let expected_state = 5_i64.to_be_bytes().to_vec();
      assert!(
          peer_b
              .await_digest(expected_state, Duration::from_secs(15))
              .await,
          "late-joiner B must converge via HeadsSummary backfill over real iroh"
      );
  }
  ```

- [ ] **T6.2**: Run `cargo test -p myrhiza-kernel --features network-iroh --test iroh_convergence iroh_late_joiner_backfills_via_heads_summary`. Expect: pass within ~15s.

- [ ] **T6.3**: If this test fails, the most likely cause is the heads-request ALPN not being registered on one of the peers' Routers. Verify by reading `crates/test-utils/src/iroh_harness.rs` `spawn_peer` calls into `spawn_iroh_peer(&self.lookup, true)` (the `true` is the load-bearing argument from T2 / T3). If the test fails with `RequestFailed { reason: "alpn mismatch" }` or similar, audit the call site.

- [ ] **T6.4**: Commit. Message:
  ```
  feat: E2E-1 T6 — iroh late-joiner backfill test (validates heads-ALPN wiring)
  ```

---

## Task T7 — `crates/kernel/tests/iroh_coexistence.rs`

**Spec ref:** §3.4 — distinct author keypairs per app, mirrored from `coexistence.rs:259-260`.

**Files:**
- Create: `crates/kernel/tests/iroh_coexistence.rs`

**Steps:**

- [ ] **T7.1**: Create the file with header:
  ```rust
  //! Two-app coexistence over real IrohNetwork — closes the iroh-realism
  //! gap for mvp.md §15.1 criterion 4. Mirrors
  //! `crates/kernel/tests/coexistence.rs::two_apps_coexist_no_event_crossing`
  //! verbatim on identity binding (distinct AuthorKeypairs per app) and
  //! asserts the same isolation properties through a real iroh swarm.
  //!
  //! Per spec §3.4. One in-process node participates in two iroh-gossip
  //! swarms (counter + echo); address-discovery scope is per-process via
  //! a shared `MemoryLookup`.

  #![cfg(feature = "network-iroh")]
  #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

  use std::collections::BTreeSet;
  use std::time::Duration;

  use bincode::Options;
  use myrhiza_kernel::identity::{AuthorKeypair, PeerKeypair};
  use myrhiza_kernel::pending::PendingCfg;
  use myrhiza_kernel::runtime::{AuthorCommand, Runtime, RuntimeCfg};
  use myrhiza_test_utils::iroh_harness::spawn_iroh_peer;
  use myrhiza_types::{BundleHash, GenesisV1, Topic, canonical_bincode};

  mod helpers;

  fn fast_cfg() -> RuntimeCfg {
      RuntimeCfg {
          drift_interval: 1,
          drift_min_interval: Duration::from_secs(0),
          drift_daily_cap: u32::MAX,
          heads_summary_tick: Duration::from_millis(100),
          pending_cfg: PendingCfg::default(),
          broadcast_capacity: 256,
          kernel_fuel_table_version: 1,
          drift_stash_cap: 256,
          transport_error_halt_threshold: 5,
      }
  }
  ```
  (Note: do **not** import `build_signed_echo_bundle` — `helpers::echo_handle()` already calls it internally per [`crates/kernel/tests/helpers/mod.rs:43-45`](../../crates/kernel/tests/helpers/mod.rs). Importing here would invite a dead-code `let _ = build_signed_echo_bundle();` call.)

- [ ] **T7.2**: The test builds two `Runtime` instances against a single iroh peer stack (two topics over one node). It cannot use `IrohHarness::spawn_peer` because the harness assigns one topic per peer; the coexistence test needs one peer with two topics. Use `spawn_iroh_peer` directly:
  ```rust
  /// Covers: mvp.md §15.1 #4. Two WASM bundles (counter + echo), two
  /// Runtime instances sharing one iroh endpoint+gossip+router stack,
  /// two distinct topics. Events authored on one runtime must NOT
  /// appear in the other's state.
  ///
  /// Distinct author keypairs per app (501 for counter, 502 for echo)
  /// per coexistence.rs:259-260.
  #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
  async fn iroh_two_apps_coexist_no_event_crossing() {
      use iroh::address_lookup::MemoryLookup;

      let lookup = MemoryLookup::default();

      // Derive the iroh secret bytes from the same peer_seed (501) that
      // PeerKeypair::deterministic uses — same alignment trick as
      // IrohHarness::spawn_peer (T3.4). Both runtimes on this node use
      // the same peer identity (single peer, two apps).
      let peer_seed: u64 = 501;
      let mut iroh_secret_bytes = [0u8; 32];
      iroh_secret_bytes[..8].copy_from_slice(&peer_seed.to_le_bytes());
      let stack = spawn_iroh_peer(&lookup, Some(iroh_secret_bytes), true).await;

      let counter_bundle_hash = BundleHash::from_bytes([0xC0; 32]);
      let echo_bundle_hash = BundleHash::from_bytes([0xEC; 32]);
      let seed = [0xBB; 32];
      let topic_name = "main".to_string();
      let counter_topic = Topic::derive(&counter_bundle_hash, &seed, &topic_name);
      let echo_topic = Topic::derive(&echo_bundle_hash, &seed, &topic_name);
      assert_ne!(counter_topic, echo_topic);

      let cfg = fast_cfg();
      let kp_counter_author = AuthorKeypair::deterministic(501);
      let kp_echo_author = AuthorKeypair::deterministic(502);

      // Single iroh peer; two Runtimes on two topics. Each gets its own
      // IrohNetwork clone — they share the underlying endpoint + gossip
      // + request_handler state via the #[derive(Clone)] added in T3.5.
      let net_counter = stack.network.clone();
      let net_echo = stack.network.clone();

      // PeerKeypair seeded to 501 to match the iroh_secret_bytes above —
      // mirrors `coexistence.rs:265-266`'s pattern of calling
      // deterministic(501) twice (PeerKeypair is not Clone).
      let peer_key_counter = PeerKeypair::deterministic(501);
      let peer_key_echo = PeerKeypair::deterministic(501);

      let runtime_counter = Runtime::start(
          net_counter,
          counter_topic,
          counter_bundle_hash,
          topic_name.clone(),
          helpers::counter_handle(),
          peer_key_counter,
          Some(AuthorKeypair::deterministic(501)),
          cfg.clone(),
          vec![], // bootstrap — same-process; no peer to dial.
      )
      .await
      .expect("runtime_counter start");

      let runtime_echo = Runtime::start(
          net_echo,
          echo_topic,
          echo_bundle_hash,
          topic_name.clone(),
          helpers::echo_handle(),
          peer_key_echo,
          Some(AuthorKeypair::deterministic(502)),
          cfg,
          vec![], // bootstrap — same-process; no peer to dial.
      )
      .await
      .expect("runtime_echo start");

      // Give the swarms a moment to settle.
      tokio::time::sleep(Duration::from_millis(300)).await;

      // Counter genesis + increment.
      let counter_genesis = GenesisV1 {
          seed,
          founder_pubkey: kp_counter_author.author,
          app_payload: 0_i64.to_be_bytes().to_vec(),
      };
      author_blocking(
          &runtime_counter.author_tx,
          canonical_bincode()
              .serialize(&counter_genesis)
              .expect("encode counter genesis"),
      )
      .await;
      author_blocking(
          &runtime_counter.author_tx,
          5_i64.to_be_bytes().to_vec(),
      )
      .await;

      // Echo genesis.
      let echo_genesis = GenesisV1 {
          seed,
          founder_pubkey: kp_echo_author.author,
          app_payload: b"hello".to_vec(),
      };
      author_blocking(
          &runtime_echo.author_tx,
          canonical_bincode()
              .serialize(&echo_genesis)
              .expect("encode echo genesis"),
      )
      .await;

      // Wait for both digests to settle (longer than MemNetwork variant
      // because iroh-gossip Plumtree forwarding adds latency).
      let mut rx_counter = runtime_counter.digest_watch.clone();
      let mut rx_echo = runtime_echo.digest_watch.clone();
      let counter_target = 5_i64.to_be_bytes().to_vec();
      let echo_target = b"hello".to_vec();
      assert!(
          await_digest(&mut rx_counter, &counter_target, Duration::from_secs(10)).await,
          "counter runtime must reach state {counter_target:?}; got {:?}",
          rx_counter.borrow().clone()
      );
      assert!(
          await_digest(&mut rx_echo, &echo_target, Duration::from_secs(10)).await,
          "echo runtime must reach state {:?}; got {:?}",
          echo_target,
          rx_echo.borrow().clone()
      );

      // Isolation: no cross-topic events on either side.
      let dropped_counter = runtime_counter.dropped_at_apply.lock().expect("lock").clone();
      let dropped_echo = runtime_echo.dropped_at_apply.lock().expect("lock").clone();
      assert!(
          dropped_counter.is_empty(),
          "counter dropped_at_apply must be empty; saw {dropped_counter:?}"
      );
      assert!(
          dropped_echo.is_empty(),
          "echo dropped_at_apply must be empty; saw {dropped_echo:?}"
      );

      let warns_counter = runtime_counter.peer_warnings.lock().expect("lock").clone();
      let warns_echo = runtime_echo.peer_warnings.lock().expect("lock").clone();
      assert!(
          !warns_counter
              .iter()
              .any(|w| matches!(w, myrhiza_kernel::runtime::PeerWarning::SignatureInvalid { .. })),
          "counter must not surface SignatureInvalid; saw {warns_counter:?}"
      );
      assert!(
          !warns_echo
              .iter()
              .any(|w| matches!(w, myrhiza_kernel::runtime::PeerWarning::SignatureInvalid { .. })),
          "echo must not surface SignatureInvalid; saw {warns_echo:?}"
      );
  }

  /// Helper: send an `AuthorCommand::Author` and await the reply.
  async fn author_blocking(
      tx: &tokio::sync::mpsc::Sender<AuthorCommand>,
      payload: Vec<u8>,
  ) {
      let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
      tx.send(AuthorCommand::Author {
          payload,
          deps: BTreeSet::new(),
          reply: reply_tx,
      })
      .await
      .expect("send AuthorCommand");
      reply_rx
          .await
          .expect("author reply")
          .expect("author ok");
  }

  /// Helper: poll a digest watch until it reports `expected` or `timeout`.
  /// Mirrors `coexistence.rs::await_runtime_digest`.
  async fn await_digest(
      rx: &mut tokio::sync::watch::Receiver<Vec<u8>>,
      expected: &[u8],
      timeout: Duration,
  ) -> bool {
      let deadline = std::time::Instant::now() + timeout;
      if rx.has_changed().unwrap_or(false) {
          if *rx.borrow_and_update() == expected {
              return true;
          }
      } else {
          rx.mark_unchanged();
      }
      loop {
          let remaining = deadline.saturating_duration_since(std::time::Instant::now());
          if remaining.is_zero() {
              return false;
          }
          let r = tokio::time::timeout(
              remaining.min(Duration::from_millis(50)),
              rx.changed(),
          )
          .await;
          match r {
              Ok(Ok(())) => {
                  if *rx.borrow() == expected {
                      return true;
                  }
              }
              Ok(Err(_)) => return *rx.borrow() == expected,
              Err(_) => {}
          }
      }
  }
  ```

- [ ] **T7.3**: Run `cargo test -p myrhiza-kernel --features network-iroh --test iroh_coexistence iroh_two_apps_coexist_no_event_crossing`. Expect: pass within ~15s.

- [ ] **T7.4**: Run `cargo clippy -p myrhiza-kernel --features network-iroh --all-targets -- -D warnings`. Fix.

- [ ] **T7.5**: Commit. Message:
  ```
  feat: E2E-1 T7 — iroh_two_apps_coexist coexistence test
  ```

---

## Task T8 — CLI binary smoke test (stdout view progression)

**Spec ref:** §3.5 row 1.

**Files:**
- Create: `crates/myrhiza-cli/tests/cli_binary.rs`

**Steps:**

- [ ] **T8.1**: Create `crates/myrhiza-cli/tests/cli_binary.rs`:
  ```rust
  //! Binary-shellout tests for myrhiza-cli. Drives the actual built
  //! binary (resolved via `env!("CARGO_BIN_EXE_myrhiza-cli")`) with
  //! scripted stdin, captures stdout/stderr/exit-code via
  //! `Child::wait_with_output()`.
  //!
  //! Per spec §3.5. Closes the gap that `tests/e2e.rs` leaves: the
  //! library-level e2e calls `myrhiza_cli::run` directly, never
  //! exercising clap parsing, `main()`, or stdio handling.

  #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

  use std::io::Write;
  use std::process::{Command, Stdio};

  use myrhiza_test_utils::bundle::build_signed_counter_bundle_three_components;

  /// Spawn `myrhiza-cli` with the given bundle dir + author seed, pipe
  /// `stdin_bytes` to stdin, collect stdout/stderr/exit-code.
  fn run_cli(bundle_dir: &std::path::Path, author_seed: u64, stdin_bytes: &[u8]) -> CliOutput {
      let mut child = Command::new(env!("CARGO_BIN_EXE_myrhiza-cli"))
          .arg("--bundle")
          .arg(bundle_dir)
          .arg("--author-seed")
          .arg(author_seed.to_string())
          .stdin(Stdio::piped())
          .stdout(Stdio::piped())
          .stderr(Stdio::piped())
          .spawn()
          .expect("spawn myrhiza-cli");
      child
          .stdin
          .as_mut()
          .expect("stdin pipe")
          .write_all(stdin_bytes)
          .expect("write stdin");
      let output = child.wait_with_output().expect("wait_with_output");
      CliOutput {
          status: output.status.code(),
          stdout: String::from_utf8(output.stdout).expect("stdout utf-8"),
          stderr: String::from_utf8(output.stderr).expect("stderr utf-8"),
      }
  }

  struct CliOutput {
      status: Option<i32>,
      stdout: String,
      stderr: String,
  }

  /// Covers: spec §3.5 row 1. Binary entrypoint wires --bundle +
  /// --author-seed + stdin + stdout correctly. After scripted input
  /// `inc 5\ninc 3\nquit\n`, asserts:
  ///   (a) exit code 0,
  ///   (b) stdout contains progressive views `counter: 0\n`, `counter: 5\n`, `counter: 8\n`,
  ///   (c) stderr contains `final state: [0, 0, 0, 0, 0, 0, 0, 8]`
  ///       (the eprintln! at main.rs:38 — "final state" goes to stderr).
  #[test]
  fn cli_binary_increment_loop_yields_final_state_via_stdout_views() {
      let (_bundle, addr) = build_signed_counter_bundle_three_components();
      let output = run_cli(&addr.bundle_dir, 0, b"inc 5\ninc 3\nquit\n");

      assert_eq!(
          output.status,
          Some(0),
          "exit code must be 0; got {:?}; stderr={:?}",
          output.status,
          output.stderr
      );
      assert!(
          output.stdout.contains("counter: 0\n"),
          "stdout must contain initial view 'counter: 0'; got: {:?}",
          output.stdout
      );
      assert!(
          output.stdout.contains("counter: 5\n"),
          "stdout must contain view after inc 5; got: {:?}",
          output.stdout
      );
      assert!(
          output.stdout.contains("counter: 8\n"),
          "stdout must contain final view 'counter: 8'; got: {:?}",
          output.stdout
      );
      assert!(
          output.stderr.contains("final state: [0, 0, 0, 0, 0, 0, 0, 8]"),
          "stderr must contain 'final state: [0, 0, 0, 0, 0, 0, 0, 8]'; got: {:?}",
          output.stderr
      );
  }
  ```

- [ ] **T8.2**: Run `cargo test -p myrhiza-cli --test cli_binary cli_binary_increment_loop_yields_final_state_via_stdout_views`. Expect: pass.

- [ ] **T8.3**: Run `cargo clippy -p myrhiza-cli --all-targets -- -D warnings`. Fix.

- [ ] **T8.4**: Commit. Message:
  ```
  feat: E2E-1 T8 — CLI binary smoke test (stdout view progression)
  ```

---

## Task T9 — CLI binary error-path tests

**Spec ref:** §3.5 rows 2 + 3.

**Files:**
- Modify: `crates/myrhiza-cli/tests/cli_binary.rs`

**Steps:**

- [ ] **T9.1**: Append to `cli_binary.rs`:
  ```rust
  /// Covers: spec §3.5 row 2. A --bundle path that does not exist must
  /// produce a non-zero exit code and a diagnostic on stderr — not a
  /// panic, not a hang.
  #[test]
  fn cli_binary_missing_bundle_exits_nonzero_with_diagnostic() {
      // /nonexistent/bundle is structurally absent; clap accepts the
      // path because it's just a string, then myrhiza_cli::run hits
      // an open() failure that propagates as Err(_).
      let output = run_cli(
          std::path::Path::new("/nonexistent/bundle/path-that-does-not-exist"),
          0,
          b"quit\n",
      );

      assert_ne!(
          output.status,
          Some(0),
          "exit code must be non-zero for missing bundle; got {:?}; stderr={:?}",
          output.status,
          output.stderr
      );
      assert!(
          !output.stderr.is_empty(),
          "stderr must contain a diagnostic for missing bundle; got empty stderr"
      );
  }

  /// Covers: spec §3.5 row 3. Mirrors
  /// `tests/e2e.rs::counter_dispatch_rejection_does_not_abort_loop`
  /// through the binary entrypoint.
  #[test]
  fn cli_binary_dispatch_rejection_does_not_abort_loop() {
      let (_bundle, addr) = build_signed_counter_bundle_three_components();
      let output = run_cli(&addr.bundle_dir, 2, b"bogus_action\ninc 1\nquit\n");

      assert_eq!(
          output.status,
          Some(0),
          "exit code must be 0 (rejected dispatch is recoverable); got {:?}; stderr={:?}",
          output.status,
          output.stderr
      );
      assert!(
          output.stdout.contains("dispatch rejected:"),
          "stdout must surface 'dispatch rejected:' for bogus action; got: {:?}",
          output.stdout
      );
      assert!(
          output.stderr.contains("final state: [0, 0, 0, 0, 0, 0, 0, 1]"),
          "stderr must contain final state [0,0,0,0,0,0,0,1] (inc 1 applied after rejection); got: {:?}",
          output.stderr
      );
  }
  ```

- [ ] **T9.2**: Run `cargo test -p myrhiza-cli --test cli_binary`. Expect: all three tests pass.

- [ ] **T9.3**: Run `cargo clippy -p myrhiza-cli --all-targets -- -D warnings`. Fix.

- [ ] **T9.4**: Commit. Message:
  ```
  feat: E2E-1 T9 — CLI binary error-path tests (missing bundle, dispatch rejection)
  ```

---

## Task T10 — Extend `Justfile` test-iroh recipe

**Spec ref:** §3.6.

**Files:**
- Modify: `Justfile`

**Steps:**

- [ ] **T10.1**: Edit `Justfile`. Locate the existing recipe:
  ```
  test-iroh:
      cargo test -p myrhiza-network --features network-iroh --tests
  ```
  Replace with:
  ```
  test-iroh:
      cargo test -p myrhiza-network --features network-iroh --tests
      cargo test -p myrhiza-kernel --features network-iroh --tests
      cargo test -p myrhiza-test-utils --features network-iroh --tests
  ```

- [ ] **T10.2**: Run `just test-iroh`. Expect: all three target runs pass; total runtime under 90s.

- [ ] **T10.3**: Run `just ci` end-to-end. Expect: exit 0, zero warnings, all gates green (fmt-check, lint, test, test-iroh, spec-coverage-check).

- [ ] **T10.4**: As a defense against debug-vs-release timing divergence, run the iroh tests once in release mode: `cargo test --release -p myrhiza-kernel --features network-iroh --tests`. Expect: pass. (Not gated in CI; this is a one-time local check before commit.)

- [ ] **T10.5**: Commit. Message:
  ```
  chore(ci): E2E-1 T10 — extend test-iroh to cover kernel + test-utils
  ```

---

## Task T11 — Update gap analysis (docs-only)

**Spec ref:** §4 — note that this task requires T1–T10 green in local `just ci`.

**Files:**
- Modify: `docs/reports/2026-05-21-mvp-gap-analysis.md`

**Steps:**

- [ ] **T11.1**: Confirm T0–T10 commits exist and `just ci` is currently green. Concretely: run `just ci` and confirm exit 0. If not, STOP — do not edit the doc; fix the failing gate first.

- [ ] **T11.2**: Edit `docs/reports/2026-05-21-mvp-gap-analysis.md`. Locate item 19 in the "What's shipped" table:
  ```
  | 19. E2E test suite | ❌ | No real iroh-cross-process tests; B-4.4 acceptance tests use in-process two-`IrohNetwork`-peers (sufficient for protocol shape but not a true E2E). |
  ```
  Replace the status cell from `❌` to `🟡` and update the description:
  ```
  | 19. E2E test suite | 🟡 partial | In-process iroh integration tests landed in E2E-1 (2026-05-22) — `crates/kernel/tests/iroh_convergence.rs` + `iroh_coexistence.rs` route real `IrohNetwork` through real `Runtime` through real WASM; `crates/myrhiza-cli/tests/cli_binary.rs` exercises the binary entrypoint via subprocess. Remaining gap: cross-OS-process iroh convergence (deferred to E2E-2). See [docs/specs/2026-05-22-e2e-test-coverage-design.md](../specs/2026-05-22-e2e-test-coverage-design.md). |
  ```

- [ ] **T11.3**: Update the tally line at the bottom of the table:
  ```
  **Tally**: 12 items ✅, 4 items 🟡, 8 items ❌ ...
  ```
  Increment 🟡 from 4 to 5, decrement ❌ from 8 to 7:
  ```
  **Tally**: 12 items ✅, 5 items 🟡, 7 items ❌ ...
  ```
  (Verify the original counts before editing — if the doc has been updated since 2026-05-21, recount.)

- [ ] **T11.4**: Run `just ci` to confirm `spec-coverage-check` still passes (the gap-analysis doc is not in the spec-coverage index, but defensive).

- [ ] **T11.5**: Commit. Message:
  ```
  docs(report): E2E-1 T11 — flip gap analysis item 19 from ❌ to 🟡
  ```

---

## Post-T11 — Final review + PR

After T11 commits and `just ci` is green:

1. Run `git log --oneline main..HEAD` — verify 12 commits in order (T0 through T11).
2. Run `git diff main..HEAD --stat` — confirm only the files this plan names are touched. The two intended production-source changes (T0's `Runtime::start` bootstrap parameter + T3.5's `#[derive(Clone)]` on `IrohNetwork`) should be the only changes under `crates/*/src/`.
3. Dispatch a fresh **opus** final-review code-reviewer agent against the full branch diff. Prompt: "Review the E2E-1 branch from main. Spec: docs/specs/2026-05-22-e2e-test-coverage-design.md. Plan: docs/plans/2026-05-22-e2e-test-coverage.md. Identify any: (a) spec-compliance gaps, (b) flake risks in the iroh-backed tests (10s/15s `await_digest` timeouts vs the existing MemNetwork 5s), (c) determinism violations introduced inadvertently, (d) production code touched beyond T0's `Runtime::start` bootstrap and T3.5's `IrohNetwork::Clone`."
4. Address findings.
5. Open PR with title `feat: E2E-1 — close in-process e2e coverage gap (real-iroh kernel tests + CLI binary tests)` and body summarizing the 12-task slice, citing spec + plan.

**On `await_digest` timeout choices (plan review I4):** the existing MemNetwork convergence tests use 5-second timeouts. The iroh-backed tests in T4 use 10s (single-originator — extra slack for swarm formation), T5 uses 10s (concurrent multi-author — most flake-prone), and T6/T7 use 15s (late-joiner + coexistence — each requires gossip warm-up + at least one HeadsSummary tick + direct-stream round-trip). These are deliberate; if a test passes consistently with a smaller timeout, prefer the smaller value (it bounds latency more tightly), but do not chase flakes below the iroh-warm-up floor of ~500ms.

---

## File summary

**New files (4):**
- `crates/test-utils/src/iroh_harness.rs`
- `crates/kernel/tests/iroh_convergence.rs`
- `crates/kernel/tests/iroh_coexistence.rs`
- `crates/myrhiza-cli/tests/cli_binary.rs`

**Modified files (9):**
- `crates/kernel/src/runtime.rs` (T0) — `Runtime::start` gains 9th param `bootstrap: Vec<PeerPubkey>`
- ~20 `Runtime::start(...)` call sites in `crates/kernel/tests/*.rs` + `crates/test-utils/src/harness.rs` (T0) — append `vec![]`
- `crates/test-utils/Cargo.toml` (T1) — `network-iroh` feature + optional deps
- `crates/test-utils/src/lib.rs` (T1) — gated module decl
- `crates/test-utils/src/harness.rs` (T3) — `pub(crate) fn from_runtime` accessor + bootstrap-aware `spawn_peer` (already covered under T0's update)
- `crates/network/src/iroh_transport.rs` (T3.5) — `#[derive(Clone)]` on `IrohNetwork`
- `crates/kernel/Cargo.toml` (T4) — flip `myrhiza-test-utils/network-iroh` in feature list
- `Justfile` (T10) — extend `test-iroh` recipe
- `docs/reports/2026-05-21-mvp-gap-analysis.md` (T11)

**Production source changes (intentional, called out per plan review B1):**
1. `Runtime::start` gains the `bootstrap` parameter (T0). This is a real signature change with mechanical updates across ~20 test call sites; behavior is preserved by passing `vec![]` everywhere except in the new iroh tests.
2. `IrohNetwork` gains `#[derive(Clone)]` (T3.5). Structurally a one-line change; all four fields already Clone/Copy.

**Total LOC delta:** ~700 LOC added (test code + harness + bootstrap parameter + plumbing), ~25 LOC modified across the ~20 `Runtime::start` call-site updates. Tally is honest — claim of "test-only" elsewhere should be read as "no behavior-altering production changes," not literally zero source-file edits in `src/`.
