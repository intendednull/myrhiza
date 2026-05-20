**Date:** 2026-05-20
**Status:** draft
**Parent:** [docs/specs/2026-05-09-myrhiza-master-design/README.md](2026-05-09-myrhiza-master-design/README.md)
**Subject:** Plan B-4.0 — Iroh transport skeleton (deps + `IrohNetwork` compile shell)

# Plan B-4.0 design — Iroh transport skeleton

## 1. Goal

Add iroh as a load-bearing transport dependency behind a `network-iroh` cargo feature. Land:

- **Pinned deps**: `iroh = "=1.0.0-rc.0"` (2026-05-07), `iroh-gossip = "=0.99.0"` (2026-05-08), both with exact-version pins per Willow / Iroh prior-art `Avoid` guidance ("Vendor-pin iroh in Cargo.toml and bump deliberately" — `prior-art/iroh/lessons.md` §Avoid row 1).
- **`IrohNetwork` struct** in `crates/network/src/iroh_transport.rs`, holding owned handles to the host's `iroh::Endpoint` + `iroh_gossip::Gossip` instances (both are `Arc`-backed in iroh's API, so the move semantics are cheap — see §3.2 for the rationale).
- **Trait impl** for `Network` (and the associated `Subscription`) that **compiles, constructs, and returns a clear "not yet implemented" error from every method**. The error variant explicitly names the follow-up slice (B-4.1).
- **One smoke-level acceptance test** that **binds a real local iroh endpoint** (UDP socket on a random port), constructs `IrohNetwork`, and asserts the endpoint's NodeID round-trips through Myrhiza's `PeerPubkey` newtype. The "skeleton" framing is about *behavior* (no real subscribe/publish yet), not *zero-UDP*; the test exercises real iroh initialization to catch dep + linkage failures early.
- **`PeerPubkey ↔ iroh::EndpointId` conversion helpers** (both are 32-byte Ed25519 public keys per `prior-art/iroh/identity.md`).

This slice lands **none** of:

- **Actual gossip pub/sub** — `subscribe` and `publish` return `Err`. Lands in B-4.1.
- **Q-4 sender attribution** — extending `GossipMessage` with `from_peer: PeerPubkey` on receive. Lands in B-4.2.
- **Real cross-process / cross-machine tests** — B-4.0's single test is intra-process. Real network tests land in B-4.3.
- **`iroh::Endpoint` lifecycle management** — the kernel embedder owns endpoint construction in B-4.0; `IrohNetwork::new(endpoint, gossip)` consumes the handles by value (iroh's `Endpoint` is internally `Arc`-backed, so the move is cheap and the embedder can keep its own clone for router-level work). The kernel-level "single endpoint per host" surface (per `prior-art/iroh/lessons.md` §Borrow row 1) becomes a future spec when the kernel grows a Router-like dispatch layer.
- **Discovery, relay configuration, ALPN namespacing, blob distribution** — all deferred to later slices or future plans.

## 2. Scope decisions (locked during brainstorming + prior-art consultation, 2026-05-20)

| Decision | Chosen | Runner-up | Why |
|---|---|---|---|
| iroh version pin | `=1.0.0-rc.0` exact | caret range | `prior-art/iroh/lessons.md` §Avoid: "Every minor is breaking." Exact pin matches workspace convention (`bincode = "=1.3.3"`, `wasmtime = "=36.0.9"`). |
| iroh-gossip version pin | `=0.99.0` exact | caret range | Same reasoning; tracks iroh release cadence (v0.99.0 was a single "Update iroh and noq to 1.0-rc.0" release — `prior-art/iroh/gossip.md` §Versions). |
| Cargo feature gating | `network-iroh` feature on `myrhiza-network` crate; **default-off** | Hard dep | Existing comment in `crates/network/src/lib.rs:8` already names this feature. Default-off keeps the in-process `MemNetwork`-only test path cheap (iroh pulls in QUIC + DNS + DHT — bumpy build cost). |
| Code location | `crates/network/src/iroh_transport.rs` (single file) | `iroh.rs` / module dir | Module named `iroh_transport` not `iroh` to avoid shadowing the extern crate inside the module body — `use iroh::Endpoint` would resolve ambiguously if the local module were also named `iroh`. Promote to `iroh_transport/` directory if it grows past ~300 lines. |
| Skeleton method behavior | All `Network` methods return `Err(NetError::Unimplemented { method, planned_in })` | `unimplemented!()` macro | `unimplemented!()` panics at runtime — workspace `panic = warn` lint would refuse without `#[allow]`. Returning a structured error is more disciplined and gives integration tests a clean assertion. |
| `NetError::Unimplemented` variant | **In scope** for B-4.0 (added now) | Use an existing variant | The existing variants (`SubscribeClosed`, `PublishFailed`) don't describe "this transport doesn't do this yet" — semantically distinct. Adding the variant once, here, prevents B-4.1+ from accidentally papering over a real bug with a misleading existing variant. |
| `PeerPubkey ↔ EndpointId` | Distinct types; **free-function** conversions `peer_pubkey_from_iroh` + `iroh_endpoint_id_from_peer_pubkey` in `iroh_transport.rs` | (a) Type alias `PeerPubkey = iroh::EndpointId`; (b) `From`/`TryFrom` trait impls | Type alias would leak iroh's API churn into Myrhiza's public surface (`prior-art/iroh/lessons.md` §Avoid row 1). `From`/`TryFrom` trait impls **are blocked by Rust's orphan rule**: `iroh::EndpointId` is a `pub type EndpointId = PublicKey;` alias (per iroh-base 1.0.0-rc.0 `key.rs:70`), and both `PeerPubkey` (`myrhiza-types`) and `iroh::PublicKey` are foreign to `myrhiza-network`. Free functions preserve the distinct-types discipline. A future plan may promote to trait impls by moving them into `myrhiza-types` behind an `iroh-compat` feature; the function bodies port verbatim. |
| Endpoint ownership | `IrohNetwork::new(endpoint: iroh::Endpoint, gossip: iroh_gossip::Gossip) -> Self` — caller owns endpoint construction | `IrohNetwork::open(builder: iroh::EndpointBuilder) -> Self` (builder ownership) | Caller-owned endpoint matches `prior-art/iroh/lessons.md` §Borrow row 1: "One `Endpoint` per host, owned by the kernel." The kernel embedder constructs once and hands references. |
| Acceptance test | One smoke test in `crates/network/tests/iroh_smoke.rs` (or `tests/iroh_skeleton.rs`) — constructs `iroh::Endpoint`, `iroh_gossip::Gossip`, `IrohNetwork`; asserts `IrohNetwork::peer_pubkey()` matches the endpoint's NodeID | Multiple tests covering each unimplemented method | YAGNI; B-4.1 will add behavioral tests. The skeleton test's job is "the trait shape compiles against iroh's real types." |
| Test gating | `#[cfg(feature = "network-iroh")]` on the new test file (or use `cargo test -p myrhiza-network --features network-iroh`) | Always-on | Default-feature-off keeps `cargo test --workspace` fast. CI runs `cargo test --features network-iroh` separately. |
| Workspace `[features]` for the network crate | Add `[features] default = [] network-iroh = ["dep:iroh", "dep:iroh-gossip", "dep:tokio"]` | Make iroh non-optional | Aligns with the feature-gating decision; `dep:` syntax for optional deps. |
| CI integration | Add `cargo test -p myrhiza-network --features network-iroh` to `just ci` (or `just test` recipe) | Skip CI gating for B-4.0 | CI must catch iroh integration failures before merge. The current `just ci` runs full workspace tests; adding the feature run is one extra line. |

## 3. Code surface

### 3.1 Crate-level changes

**Workspace `Cargo.toml` `[workspace.dependencies]`** — add (after existing entries):

```toml
# Iroh transport substrate. Pinned tight per
# prior-art/iroh/lessons.md §Avoid row 1 (iroh pre-1.0 API churn —
# "every minor is breaking"). Bump deliberately.
iroh = "=1.0.0-rc.0"
iroh-gossip = "=0.99.0"
```

**`crates/network/Cargo.toml`** — add optional deps + feature, all using `workspace = true` per project convention:

```toml
[dependencies]
# ... existing deps unchanged ...
iroh = { workspace = true, optional = true }
iroh-gossip = { workspace = true, optional = true }

[features]
default = []
network-iroh = ["dep:iroh", "dep:iroh-gossip"]
```

**No tokio-feature changes needed.** `tokio` is already a non-optional workspace dep in `crates/network/Cargo.toml` (`features = ["sync", "rt", "macros", "time"]`); cannot be in a `dep:` feature spec. iroh brings its own tokio configuration internally. If the smoke test or future B-4.1 work needs additional tokio features (e.g. `"net"`), add them at the existing workspace tokio line — but the current set is likely sufficient.

**`crates/network/src/lib.rs`** gains a feature-gated module re-export. To avoid shadowing the external `iroh` crate inside the module body and to give consumers an unambiguous import path, the module is named `iroh_transport`:

```rust
#[cfg(feature = "network-iroh")]
pub mod iroh_transport;
#[cfg(feature = "network-iroh")]
pub use iroh_transport::IrohNetwork;
```

### 3.2 `crates/network/src/iroh_transport.rs` — the new module

(Module named `iroh_transport` not `iroh` to avoid shadowing the extern crate inside the module body — see §3.1.)

```rust
//! Iroh transport implementation of the [`Network`] trait.
//!
//! B-4.0 SKELETON: this module compiles against iroh 1.0.0-rc.0 +
//! iroh-gossip 0.99.0 and exposes the type surface but every
//! `Network` method returns [`NetError::Unimplemented`]. B-4.1 will
//! wire `subscribe` + `publish` to real iroh-gossip semantics; B-4.2
//! will thread per-connection sender identity (Q-4); B-4.3 adds
//! real cross-process acceptance tests.
//!
//! ## Why "skeleton"
//!
//! `prior-art/iroh/lessons.md` §Avoid row 1: "Every minor is
//! breaking" — pre-1.0 iroh API churn means landing the compile
//! shell first (pin-against-rc-0, prove the type surface aligns)
//! reduces the blast radius of a future re-pin. The behavioral
//! work lands in B-4.1+.

use crate::{NetError, Network, Subscription};
use myrhiza_types::{PeerPubkey, Topic};

/// Iroh-backed [`Network`] implementation.
///
/// Holds owned (Arc-backed, cheaply cloneable) handles to a
/// host-level [`iroh::Endpoint`] + an [`iroh_gossip::Gossip`]
/// instance. Per `prior-art/iroh/lessons.md` §Borrow row 1, the
/// kernel embedder constructs these once and may hand one clone
/// here while retaining another for router-level work.
pub struct IrohNetwork {
    endpoint: iroh::Endpoint,
    gossip: iroh_gossip::Gossip,
    /// Cached `PeerPubkey` derived from `endpoint.node_id()` at
    /// construction time. Avoids the per-call conversion for code
    /// paths that need the local peer identity.
    peer_pubkey: PeerPubkey,
}

impl IrohNetwork {
    /// Construct an `IrohNetwork` from a pre-built [`iroh::Endpoint`]
    /// and [`iroh_gossip::Gossip`].
    ///
    /// # Errors
    /// None at construction time — the conversion from
    /// [`iroh::EndpointId`] to [`PeerPubkey`] is infallible because
    /// both are 32-byte raw public keys.
    #[must_use]
    pub fn new(endpoint: iroh::Endpoint, gossip: iroh_gossip::Gossip) -> Self {
        let endpoint_id = endpoint.node_id();
        let peer_pubkey = PeerPubkey::from(endpoint_id);
        Self {
            endpoint,
            gossip,
            peer_pubkey,
        }
    }

    /// Return the local peer's public key (32-byte Ed25519).
    #[must_use]
    pub fn peer_pubkey(&self) -> PeerPubkey {
        self.peer_pubkey
    }

    /// Return a borrow of the underlying [`iroh::Endpoint`] for
    /// kernel-embedder use (e.g. relay configuration, ALPN
    /// registration). Kept narrow so future refactors can hide
    /// endpoint internals behind a capability-gated surface.
    #[must_use]
    pub fn endpoint(&self) -> &iroh::Endpoint {
        &self.endpoint
    }
}

#[async_trait::async_trait]
impl Network for IrohNetwork {
    type Subscription = IrohSubscription;

    async fn subscribe(&self, _topic: Topic) -> Result<Self::Subscription, NetError> {
        Err(NetError::Unimplemented {
            method: "Network::subscribe",
            planned_in: "B-4.1",
        })
    }

    async fn publish(
        &self,
        _topic: Topic,
        _message: crate::GossipMessage,
    ) -> Result<(), NetError> {
        Err(NetError::Unimplemented {
            method: "Network::publish",
            planned_in: "B-4.1",
        })
    }

    async fn unsubscribe(&self, _topic: Topic) -> Result<(), NetError> {
        Err(NetError::Unimplemented {
            method: "Network::unsubscribe",
            planned_in: "B-4.1",
        })
    }
}

/// Skeleton `Subscription` impl; instances cannot be constructed
/// outside this module in B-4.0 (the `subscribe` method that would
/// return one always returns `Err`).
pub struct IrohSubscription {
    _private: (),
}

#[async_trait::async_trait]
impl Subscription for IrohSubscription {
    async fn recv(&mut self) -> Result<Option<crate::GossipMessage>, crate::SubError> {
        unreachable!(
            "IrohSubscription cannot be constructed in B-4.0 — \
             Network::subscribe always returns Err(NetError::Unimplemented). \
             Reaching this code path indicates a future B-4.1+ refactor \
             that constructed a subscription without implementing recv."
        )
    }
}

// ---- PeerPubkey <-> EndpointId conversions ----
//
// Free functions, NOT trait impls: `iroh::EndpointId` is a type alias
// for `iroh::PublicKey`, so both types in any `From`/`TryFrom` we'd
// write are foreign to `myrhiza-network` — Rust's orphan rule blocks
// the impl. Free functions preserve distinct-types discipline without
// the orphan-rule violation. See §2 decision-table row for the
// runner-up paradigms.

pub fn peer_pubkey_from_iroh(endpoint_id: iroh::EndpointId) -> PeerPubkey {
    // Both types are raw 32-byte Ed25519 public keys per
    // prior-art/iroh/identity.md §"NodeID = Ed25519 public key".
    PeerPubkey::from_bytes(*endpoint_id.as_bytes())
}

pub fn iroh_endpoint_id_from_peer_pubkey(
    peer: PeerPubkey,
) -> Result<iroh::EndpointId, iroh::KeyParsingError> {
    // iroh::EndpointId::from_bytes validates the bytes form a valid
    // Ed25519 curve point — hence the fallible Result.
    iroh::EndpointId::from_bytes(peer.as_bytes())
}
```

**Note on iroh's exact API**: identifiers like `iroh::EndpointId`, `iroh::Endpoint::id()`, `iroh::EndpointId::from_bytes`, `iroh::KeyParsingError`, and the `iroh::endpoint::presets` module are the **current** names per iroh 1.0.0-rc.0 (verified at impl time). Notable adaptations from the prior-art-folder (dated 2026-05-08) snapshot:

- `Endpoint::node_id() → Endpoint::id()` (rename).
- `endpoint_id::ParseError → KeyParsingError` (different module path AND name).
- `iroh::EndpointId` is a `pub type` alias for `iroh::PublicKey`, not a distinct newtype (drives the free-function choice above).
- `Endpoint::builder()` requires a preset arg (e.g. `presets::Minimal`); smoke test uses `Minimal` to avoid n0 DNS/relay egress.
- `iroh_gossip::Gossip::builder().spawn(endpoint)` is **synchronous**, returns `Gossip` directly (no `Future`, no `Result`).

### 3.3 `NetError::Unimplemented` variant

Add to `crates/network/src/lib.rs`:

```rust
#[derive(Debug, Error)]
pub enum NetError {
    // ... existing variants ...

    /// The transport recognizes the call but the impl is not yet
    /// landed. Carries the method name + the slice in which it is
    /// planned. Used by skeleton transports (B-4.0) before behavioral
    /// implementations land.
    #[error("network transport does not yet implement {method} (planned in {planned_in})")]
    Unimplemented {
        method: &'static str,
        planned_in: &'static str,
    },
}
```

The variant lives forever; once all skeleton transports finish their behavior, the variant becomes dead-code-warn'd — at which point the workspace's `missing_docs = "warn"` + dead-code analysis will surface it. Future cleanup removes the variant. **Not** time-bombed: leaving it in place after B-4.3 is a clean "no transport implements this" signal for any future skeleton (jco backend, custom-transport extension points, etc.).

## 4. Acceptance test

`crates/network/tests/iroh_skeleton.rs` (new file; feature-gated):

```rust
//! B-4.0 smoke test: prove IrohNetwork compiles against iroh
//! 1.0.0-rc.0 and the PeerPubkey ↔ EndpointId conversion roundtrips.
//!
//! Per docs/specs/2026-05-20-plan-b-4-0-iroh-skeleton-design.md §4.

#![cfg(feature = "network-iroh")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use myrhiza_network::IrohNetwork;
use myrhiza_types::PeerPubkey;

/// Covers: identity.md §6 (kernel custody — verify iroh's NodeID
/// pubkey routes through `PeerPubkey` cleanly) + B-4.0 §3.2.
#[tokio::test]
async fn iroh_network_constructs_and_exposes_endpoint_id_as_peer_pubkey() {
    // Construct a minimal in-process iroh endpoint. Defaults: secret
    // key auto-generated, binds to a random local UDP port.
    //
    // API NOTE — verify at impl time:
    // - `Endpoint::builder()` may require a preset arg in 1.0.0-rc.0
    //   (per prior-art/iroh/architecture.md the documented signature
    //   was `Endpoint::builder(presets::N0)...`). Check `cargo doc
    //   --open -p iroh` and adapt.
    // - Default builder may attempt pkarr discovery publishing on
    //   `bind()`. CI sandboxes may block egress. Look for
    //   `.clear_discovery()` / `.disable_discovery()` / a no-op
    //   discovery preset. If none exists, document the CI requirement
    //   (outbound UDP to default n0 relays must be allowed) in the
    //   test docstring rather than papering over.
    let endpoint = iroh::Endpoint::builder()
        // .clear_discovery()  // ← uncomment if such method exists
        .bind()
        .await
        .expect("iroh endpoint bind");

    let gossip = iroh_gossip::Gossip::builder()
        .spawn(endpoint.clone())
        .await
        .expect("iroh-gossip spawn");

    let network = IrohNetwork::new(endpoint.clone(), gossip);
    let peer_pk_via_struct = network.peer_pubkey();

    let endpoint_id = endpoint.node_id();
    let peer_pk_via_conversion = PeerPubkey::from(endpoint_id);

    assert_eq!(
        peer_pk_via_struct, peer_pk_via_conversion,
        "IrohNetwork::peer_pubkey() must match From<EndpointId>::from conversion"
    );
    assert_eq!(
        peer_pk_via_struct.as_bytes(),
        endpoint_id.as_bytes(),
        "PeerPubkey bytes must match EndpointId bytes (both 32-byte Ed25519 pubkey)"
    );
}

/// Covers: B-4.0 §3.2 — skeleton methods return structured errors,
/// not panics. Regression for "skeleton should not crash CI."
#[tokio::test]
async fn iroh_network_subscribe_returns_unimplemented() {
    use myrhiza_network::{NetError, Network};
    use myrhiza_types::Topic;

    let endpoint = iroh::Endpoint::builder()
        .bind()
        .await
        .expect("iroh endpoint bind");
    let gossip = iroh_gossip::Gossip::builder()
        .spawn(endpoint.clone())
        .await
        .expect("iroh-gossip spawn");
    let network = IrohNetwork::new(endpoint, gossip);

    let topic = Topic::from_bytes([0xAB; 32]);
    let err = network.subscribe(topic).await.expect_err("must return Err");
    match err {
        NetError::Unimplemented { method, planned_in } => {
            assert_eq!(method, "Network::subscribe");
            assert_eq!(planned_in, "B-4.1");
        }
        other => panic!("expected Unimplemented, got {other:?}"),
    }
}
```

Two tests because:

1. The constructor test verifies iroh's endpoint integration + pubkey conversion.
2. The unimplemented test verifies the skeleton's error-return discipline (no panics, structured `NetError::Unimplemented` with the right metadata).

**Caveat — discovery configuration**: iroh 1.0.0-rc.0's default `Endpoint::builder()` may attempt to publish a discovery record to n0's pkarr relay (per `prior-art/iroh/identity.md` §Discovery). For a unit test we want to avoid network egress. Check the iroh API for `clear_discovery()` or `disable_discovery()`; if absent, use `discovery_static(...)` with a no-op. The plan task spec will own resolving this exact API call.

## 5. CI integration

Add a `test-iroh` recipe to the existing `Justfile`:

```just
test-iroh:
    cargo test -p myrhiza-network --features network-iroh --tests
```

Then update the existing `ci:` recipe to include the new step. Current shape is roughly `ci: fmt-check lint test spec-coverage-check`; the edited line becomes:

```just
ci: fmt-check lint test test-iroh spec-coverage-check
```

GitHub Actions CI runs `just ci` (per the existing setup that produced PR #4 / #5's CI checks). Adding `test-iroh` to the gate ensures the iroh dep is exercised on every push — catches version-pin drift, broken iroh API uses, and CI runner UDP-port availability issues early.

The default `cargo test --workspace --all-targets` invocation in the existing `test:` recipe does NOT activate `network-iroh` (default-feature-off), so the new test file's `#![cfg(feature = "network-iroh")]` gate makes it inert under that path. The `test-iroh` recipe is the only path that actually compiles + runs the iroh skeleton tests.

## 6. Cross-references

- `prior-art/iroh/identity.md` §"NodeID = Ed25519 public key" — confirms 32-byte raw key shape; B-4.0's `PeerPubkey::from(EndpointId)` is sound.
- `prior-art/iroh/lessons.md` §Avoid row 1 — "every minor is breaking" → exact version pin.
- `prior-art/iroh/lessons.md` §Borrow row 1 — "One `Endpoint` per host, owned by the kernel" → caller-owned endpoint construction.
- `prior-art/iroh/gossip.md` §Versions — iroh-gossip 0.99.0 was a single-line iroh-bump release; expected to track iroh's 1.0 final closely.
- `prior-art/iroh/critiques.md` — n0 governance / commercial-steward risk; informs the dep-vendor-pin discipline but doesn't change B-4.0's scope.
- `prior-art/iroh/mobile-and-wasm.md` — iroh-ffi unmaintained warning; out of scope for B-4.0 (native-only).
- [identity.md](2026-05-09-myrhiza-master-design/identity.md) §6.1 — `PeerPubkey` semantics; B-4.0's iroh integration treats `EndpointId` as the source of truth at the transport layer, `PeerPubkey` as the kernel-side display.
- [2026-05-10-plan-b-1-dag-memnet-design.md](2026-05-10-plan-b-1-dag-memnet-design.md) §6 — `Network` trait shape; B-4.0 implements it (skeleton).
- `crates/network/src/lib.rs` (current) — `Network` + `Subscription` trait + `NetError` enum + `GossipMessage` envelope. B-4.0 adds one `NetError` variant; everything else is consumed unchanged.

## 7. Surface change summary

New public surface in `myrhiza_network`:

- `IrohNetwork` struct (feature-gated).
- `IrohSubscription` struct (feature-gated).
- `peer_pubkey_from_iroh(endpoint_id) -> PeerPubkey` free function (feature-gated; orphan rule blocks trait impl — see §2 / §3.2).
- `iroh_endpoint_id_from_peer_pubkey(peer) -> Result<iroh::EndpointId, iroh::KeyParsingError>` free function (feature-gated; same reason).
- `NetError::Unimplemented { method, planned_in }` variant.

Unchanged public surface:

- `Network` trait shape.
- `Subscription` trait shape.
- `GossipMessage` envelope.
- `MemNetwork` impl.

Cargo feature added:

- `myrhiza-network::network-iroh` (default-off).

CI changes:

- New `just test-iroh` recipe.
- `just ci` runs `just test-iroh`.

## 8. Non-goals (explicit)

- **No real gossip subscribe/publish.** Skeleton only; behavior in B-4.1.
- **No Q-4 sender attribution.** Lands in B-4.2 once subscribe is real.
- **No cross-process tests.** Smoke test is intra-process. Real network tests in B-4.3.
- **No browser / jco integration.** Iroh's wasm-bindgen story is out of scope; B-4.* is native-only.
- **No endpoint lifecycle management.** Caller owns construction; future kernel-Router work may absorb it.
- **No relay configuration / ALPN registration / discovery tuning.** All deferred to behavioral slices (B-4.1+).
- **No `iroh::EndpointBuilder` surface.** Caller passes a pre-built endpoint.
- **No iroh-blobs.** Bundle distribution is a separate plan.

## 9. Prior-art consultation

Decisions in §2 were grounded in the following prior-art folders (consulted via `using-prior-art` skill, 2026-05-20):

- **`prior-art/iroh/lessons.md`** §Avoid + §Borrow — the consult-this-when-designing file. Validates the entire shape: kernel-owned endpoint, exact-pin discipline, ALPN-namespaced multi-tenant dispatch (B-4.* future), distinct `PeerPubkey` from iroh's `EndpointId`. The "anchor against concepts, not API surface" guidance shapes B-4.0's documentation style: cite primitives by concept ("32-byte Ed25519 pubkey", "topic-based pub/sub via Plumtree") and import iroh's current names as terminology only.
- **`prior-art/iroh/identity.md`** §"NodeID = Ed25519 public key" — confirms the conversion roundtrip is sound; both types are 32 raw bytes of Ed25519 public key.
- **`prior-art/iroh/gossip.md`** — iroh-gossip primitive shape (Plumtree + HyParView). Relevant for B-4.1+; B-4.0 only needs the type surface (`iroh_gossip::Gossip`).
- **`prior-art/iroh/critiques.md`** + **`prior-art/iroh/governance.md`** — single-vendor stewardship risk; informs the exact-pin discipline.
- **`prior-art/iroh/architecture.md`** — `Endpoint` API as kernel transport surface. Confirms the caller-owned-endpoint decision.

**Runner-up paradigms rejected:**

- **Type alias `PeerPubkey = iroh::EndpointId`** — rejected per "anchor against concepts" (lessons.md §Avoid row 1). A type alias would propagate iroh's `EndpointId` rename history into Myrhiza's public surface.
- **`unimplemented!()` macro returns instead of `NetError::Unimplemented`** — rejected because workspace `panic = warn` lint would refuse; structured-error path is more disciplined and integration-testable.
- **Always-on iroh dep (no feature gate)** — rejected because it bumps the workspace build cost substantially (QUIC + DNS + DHT + crypto). Feature-gated keeps `cargo test --workspace` fast for crates that don't need iroh.

**Remaining gaps in the prior-art corpus** (candidate triggers for future research):

- iroh API exact-name validation against 1.0.0-rc.0 (the prior-art folder is dated 2026-05-08 — should match closely but should be verified at impl time).
- iroh-gossip's `subscribe`/`publish` exact API surface — needed for B-4.1, not B-4.0.

## 10. Edge cases

- **iroh endpoint binds to a random UDP port** — the smoke test takes whatever the OS gives; no port conflict expected. If CI's sandbox blocks UDP, the test will hang on `bind().await`. Mitigation: timeout via `tokio::time::timeout` if test 1 turns out to be flaky in CI.
- **iroh discovery attempts a network call on `bind()`** — if the default `Endpoint::builder()` publishes to pkarr at bind time, the smoke test will network-egress. Mitigation: explicit discovery override (TBD at impl time per §4 caveat).
- **Pre-1.0 API rename between rc-0 and rc-N** — the implementer must verify exact names at impl time. The spec's API claims are based on prior-art dated 2026-05-08; if iroh has rotated names since, adapt and document in the commit body.
- **`iroh::EndpointId::from_bytes` validation failure** — `TryFrom<PeerPubkey>` returns the iroh-defined parse error. Acceptable because (a) `PeerPubkey` is constructed only from known-good sources in Myrhiza (verified Ed25519 signatures), so the failure path is unreachable in practice; (b) explicit `TryFrom` semantics are correct even if practically the failure can't happen.

## 11. Out-of-scope future work — explicit deferrals

- **B-4.1**: real `Network::subscribe` + `Network::publish` implementations via iroh-gossip; behavioral acceptance tests using two in-process IrohNetwork instances over real iroh.
- **B-4.2**: Q-4 — thread `from_peer: PeerPubkey` through `GossipMessage` envelope on receive (iroh's per-connection NodeID provides this natively); plumb to `PendingBuffer::insert(event, peer_id)` + `EquivocationFlag::peer`.
- **B-4.3**: real cross-process tests using two OS processes connected via iroh's relay or direct UDP.
- **B-4.4 (likely):** ALPN namespacing + iroh `Router` integration for multi-app dispatch.
- **B-5+** (later slices): blob distribution via iroh-blobs, custom transports, revocation topic.

## 12. Sources

- [`prior-art/iroh/lessons.md`](../prior-art/iroh/lessons.md) §Validates / §Avoid / §Borrow — primary input.
- [`prior-art/iroh/identity.md`](../prior-art/iroh/identity.md) — NodeID = 32-byte Ed25519 pubkey.
- [`prior-art/iroh/gossip.md`](../prior-art/iroh/gossip.md) §Versions — iroh-gossip 0.99.0 release semantics.
- [`prior-art/iroh/architecture.md`](../prior-art/iroh/architecture.md) — Endpoint + Router shape.
- [identity.md](2026-05-09-myrhiza-master-design/identity.md) §6.1 — PeerPubkey semantics.
- [2026-05-10-plan-b-1-dag-memnet-design.md](2026-05-10-plan-b-1-dag-memnet-design.md) §6 — Network trait.
- `crates/network/src/lib.rs:7` — existing comment naming the `network-iroh` feature.
- iroh 1.0.0-rc.0 release blog (cited via prior-art `history.md`).
- iroh-gossip 0.99.0 release notes (cited via prior-art `gossip.md`).
