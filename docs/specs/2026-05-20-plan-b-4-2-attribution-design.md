**Date:** 2026-05-20
**Status:** draft
**Parent:** [docs/specs/2026-05-09-myrhiza-master-design/README.md](2026-05-09-myrhiza-master-design/README.md)
**Subject:** Plan B-4.2 — Sync-message sender attribution + real unsubscribe

# Plan B-4.2 design — Q-4 attribution on HeadsSummary / HeadsRequest, drop-as-unsubscribe

## 1. Goal

Pay off the two B-4.1 deferrals that block honest deployment of `IrohNetwork` against real peers:

1. **Q-4 — sender attribution at the protocol layer.** `HeadsSummary` and `HeadsRequest` cross iroh-gossip with no signed-by field; iroh-gossip's `Event::Received.delivered_from` is the Plumtree last-hop neighbor, not the original publisher. Without authenticated sender identity, a malicious peer can forge backfill nudges or range requests under any pubkey. B-4.2 mirrors `DriftMessage`'s existing pattern (per `crates/types/src/dag.rs:150-193` — `signed_by_peer: PeerPubkey` + `signature: [u8; 64]` covering a separate `*SignedPayload` struct) onto both sync-message types, plus a `topic: Topic` field in the signed payload that anchors each signature to a single topic so cross-topic replay fails verification.
2. **`IrohNetwork::unsubscribe`.** iroh-gossip 0.99.0 exposes no explicit "leave swarm" API — drop on the `GossipTopic`'s last sender/receiver pair IS the leave (verified at `iroh-gossip/api.rs` lines 355-363 — "Once the `GossipTopic` is dropped, the network actor will leave the gossip topic … the topic will be left once both the `GossipSender` and `GossipReceiver` halves are dropped"). B-4.1's `Err(NetError::Unimplemented { method: "Network::unsubscribe", planned_in: "B-4.2" })` is misleading: dropping the subscription IS the implementation in v1. B-4.2 makes `unsubscribe` return `Ok(())`, with rustdoc documenting that the load-bearing cleanup happens on subscription drop.

**Wire-freeze break acknowledged-OK.** Adding `signed_by_peer` + `signature` to `HeadsSummary` and `HeadsRequest` regenerates the canonical-bincode byte layout for both. Pre-launch, no peers in the wild — no kernel-major bump needed. Old wire bytes from B-4.1 will fail to decode against B-4.2 binaries (loudly, via `canonical_bincode().deserialize::<GossipMessage>`); a heterogeneous mesh during the changeover window is impossible by construction because there is no production mesh.

This slice lands **none** of:

- **HeadsRequest direct-streams.** Per-publisher attribution via iroh point-to-point streams (new ALPN, Router protocol-handler dispatch) is correct long-term for HeadsRequest specifically (point-to-point semantics, no broadcast fanout needed), but is colocated with the B-4.3 cross-process test harness because both depend on the same Router protocol-registration machinery. B-4.2 sticks with signed-envelope-over-gossip for HeadsRequest to ship attribution without that infrastructure churn.
- **Halt detection** on persistent `ApiError` mid-stream — `IrohSubscription::recv` still spins-on-error per B-4.1 §6.
- **Lag-count fidelity** — `SubError::Lagged(0)` sentinel unchanged.
- **NeighborUp / NeighborDown observability** — still silently consumed.
- **NEW ALPN registration / Router protocol-handler dispatch.** Deferred to B-4.3 with the direct-streams work.

## 2. Scope decisions (locked during brainstorming + dag.rs / runtime.rs survey, 2026-05-20)

| Decision | Chosen | Runner-up | Why |
|---|---|---|---|
| **Signed-envelope shape** | Per-variant attribution fields — add `signed_by_peer: PeerPubkey` + `signature: [u8; 64]` to `HeadsSummary` and `HeadsRequest` directly, plus a `topic: Topic` field in their respective signed-payload structs. Mirrors `DriftMessage` / `DriftSignedPayload` exactly (`crates/types/src/dag.rs:150-193`). | (a) Global `Signed(Box<GossipMessage>)` envelope wrapping every variant; (b) Embed `signed_by_peer` + `signature` only on the variants that need it but skip a separate `SignedPayload` struct (sign the canonical bincode of the message-with-zeroed-sig-bytes instead) | The existing precedent in `dag.rs` is per-variant + separate `SignedPayload`. Repeating it for `HeadsSummary` + `HeadsRequest` keeps the verification site uniform (each variant's handler reconstructs *its* `*SignedPayload`, serializes, verifies) and keeps the wire-freeze test pattern uniform (each variant pins its prefix-bytes-match-SignedPayload property). Option (a) would require either bumping the `GossipMessage` variant discriminator scheme (wire-breaking on every variant, including `Event` which already carries its own Ed25519 sig inside `Event::signature`) or losing the per-variant byte freeze. Option (b) (zero-the-sig-bytes-before-signing) is fragile to field reorders and harder to test in isolation. **Decisive rationale:** the runtime already follows the "construct the SignedPayload twice" pattern at `runtime.rs:1414-1466`; uniformity beats clever. |
| **Topic-binding location** | `topic: Topic` is a field on `HeadsSummarySignedPayload` and `HeadsRequestSignedPayload` (NOT on the outer message). The signature covers it; the wire does not (recipient reconstructs `topic` from the gossip subscription that delivered the message). | (a) Add `topic: Topic` to the outer message (signed-over-AND-on-the-wire); (b) Inject the topic as additional bytes appended to the signed-byte stream rather than as a struct field | DriftMessage handles topic-binding transitively via `anchor.event_hash` (events themselves bind topic via the genesis-derivation chain at convergence.md §4.6); HeadsSummary and HeadsRequest have no equivalent transitive binding because their `authors` / `requests` payload is topic-agnostic structurally. Without explicit topic-binding, a HeadsSummary signed for topic X under peer P's key could be replayed verbatim on topic Y by an attacker (no signature break — the bytes are valid; the gossip overlay is the only thing routing it to a topic). Putting `topic` *in the signed payload only* (not on the wire) avoids redundancy: the recipient already knows the topic from `self.topic` at the verification site, and not putting it on the wire keeps the canonical bytes one field smaller. **Runner-up (a) rejected** because outer-on-wire duplication invites the bug where two peers serialize the same logical message under different wire-`topic` values (after a mid-runtime refactor) — wire and signed bytes would still match each other but the peer can't catch a forged-but-wrong-topic message. **Runner-up (b) rejected** because a sibling struct definition is far easier to keep in sync with the message-struct field set (the wire-freeze test pins the relationship). |
| **Verification location** | In `Runtime::handle_message`'s `HeadsSummary` / `HeadsRequest` arms — verify the signature BEFORE the body-consuming handler runs. Mirrors `process_drift_message` at `runtime.rs:1448-1466` (loopback filter; reconstruct signed payload; `myrhiza_manifest::verify_signature`; bail on err). | (a) Verify inside `IrohSubscription::recv` so verification is transport-layer; (b) Verify inside `handle_heads_summary` / `handle_heads_request` after the handler starts work | Transport-layer verification (a) would put signature semantics inside `myrhiza-network`, which currently knows nothing about per-message attribution policy — that's a cross-crate scope expansion not warranted by B-4.2. The runtime already has the `peer_key` machinery and the verify_signature dependency; adding two arms next to `process_drift_message` keeps signature policy in one crate. Inside the handler (b) loses the "drop bad-sig before any state work" property; verification-at-dispatch is the cleaner failure mode. |
| **`PeerWarning::SignatureInvalid` variant** | Add a new variant `PeerWarning::SignatureInvalid { peer }` parallel to `PeerWarning::DecodeFailed`. Pushed when `verify_signature` returns Err on either of the two new arms. `peer` is the *claimed* `signed_by_peer` from the message (not the last-hop delivered-from neighbor); the runtime does not yet know if the claim is fraudulent — only that the signature didn't verify against the claimed key. | (a) Reuse `DecodeFailed { peer }` for sig failures; (b) Halt the runtime on any sig failure; (c) Silently drop with no warning | Sig failures and decode failures are categorically different — `DecodeFailed` is "I cannot parse this"; `SignatureInvalid` is "I parsed it but the cryptographic claim doesn't hold." Surfacing them separately lets app-level dashboards count each correctly (per `prior-art/willow/identity.md` §"Ed25519 as identity root" — sig failures are diagnostically distinct from envelope-malformed errors). Reuse-DecodeFailed (a) collapses two debug paths into one. Halt (b) is hostile — one malicious peer could halt every honest peer's runtime; signature failures are routine in adversarial conditions and must be non-fatal. Silent drop (c) violates CLAUDE.md "no swallowing errors." |
| **Wire-freeze strategy** | Regenerate the affected snapshot bytes / sizes in `crates/types/tests/wire_freeze.rs` (specifically: `heads_summary_wire_layout`, `heads_request_wire_layout`, plus three new tests pinning the prefix-bytes property between message and signed-payload structs and three more pinning the four-variant `GossipMessage` tag stability). Existing `gossip_message_*_variant_tag_is_*_u32_be` tests stay green — the variant *order* in `GossipMessage` does not change, only the variant *payload* bytes lengthen. Pre-launch wire-freeze break, no kernel-major bump (acknowledged-OK in §1). | Bump a wire-version envelope (`enum WireV2 { V1(GossipMessage), V2(GossipMessageV2) }` style) | Pre-launch the wire-version envelope is pure ceremony; landed peers do not exist. Reserve wire-version machinery for the first post-1.0 wire change. |
| **Real unsubscribe** | `IrohNetwork::unsubscribe` returns `Ok(())`. Rustdoc documents that the load-bearing cleanup IS subscription drop (iroh-gossip 0.99.0 has no explicit leave API; verified at `iroh-gossip/api.rs` lines 355-363). | (a) Wait for an explicit leave API upstream; (b) Best-effort send-then-drop the GossipSender to nudge a leave signal | (a) blocks B-4.2 on n0's roadmap. (b) is over-engineering: the actor cleans up when its receivers + senders all drop; an extra zero-byte broadcast wouldn't even be a leave signal at the protocol level (iroh-gossip doesn't synthesize one). Drop IS the v1 implementation. **However**: the `IrohNetwork::unsubscribe` method by itself does NOT trigger a drop — `IrohNetwork` doesn't hold any subscriptions to drop. The caller (the runtime) holds the `IrohSubscription` and drops it when its scope ends. So `unsubscribe` returning `Ok(())` is honest only if rustdoc explicitly notes the method is a no-op semantically because the cleanup happens through caller-side subscription drop. See §3.3 for the rustdoc draft. |
| **Loopback filter** | Add loopback filter to both new verification arms — if `signed_by_peer == self.peer_key.public`, `verify_*` returns `false` so the body-consuming handler doesn't run (we'd be self-diffing our own claim — a guaranteed no-op). Mirrors `process_drift_message:1450-1452` which uses `return;` in a void fn for the same purpose. | No loopback filter — verify own messages too | **`MemNetwork` (used in kernel acceptance tests) uses a tokio broadcast channel, which delivers own publishes back to own receivers.** Without a loopback filter, a single-peer runtime would attempt to verify its own HeadsSummary as inbound, paying sig-verify CPU on every own-publish for no semantic benefit. **`IrohNetwork` does NOT echo own broadcasts** (Plumtree's `broadcast()` at `iroh-gossip-0.99.0/src/proto/plumtree.rs:467-487` pushes to eager peers but never emits `OutEvent::EmitEvent(Received(...))` for own messages), so the filter is a safety no-op for production traffic. Filter is still load-bearing for the MemNetwork test path and for consistency with the existing drift handler. |
| **`peer_key.sign` source-of-truth** | Both publish-side signing sites call `self.peer_key.sign(canonical_bincode(payload))` exactly as `maybe_emit_drift` does at `runtime.rs:1425`. No new identity machinery. | New helper `sign_envelope(payload)` on `PeerKeypair` | One-call sign-site is already small enough; adding an envelope helper would only justify itself if there were three or more callers. There are exactly three drift+heads-summary+heads-request publishes after B-4.2; revisit if a fourth lands. |
| **Verification helper extraction** | Add `verify_heads_summary` and `verify_heads_request` as private fn-on-`Runtime` (mirrors how the drift verify is inline in `process_drift_message` but the body is small and self-contained). Each takes `&HeadsSummary` / `&HeadsRequest` + `&Topic`, returns `bool`. Inline at the dispatch site. | Single generic `verify_signed_message<P: Serialize>(payload: &P, sig: &[u8; 64], pk: &PeerPubkey) -> bool` helper | The two payload-shape reconstructions differ (different fields), so the "generic" helper would just be `verify_signature` itself wrapped. Cost > benefit. |
| **`Signed*Payload` struct visibility** | `pub` in `myrhiza_types` (matching `DriftSignedPayload`). Both struct definitions live in `crates/types/src/dag.rs` alongside the message types they shadow. | `pub(crate)` | Tests in `crates/kernel/tests/perf_carryovers.rs:732` construct `DriftSignedPayload` directly to forge a hand-signed drift message; the equivalent B-4.2 acceptance tests need the same shape for `HeadsSummarySignedPayload` and `HeadsRequestSignedPayload`. Cross-crate test visibility requires `pub`. |
| **Test runtime** | `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]` for tests that go through `IrohNetwork`; default single-threaded for pure-types serialization tests. | Single-threaded everywhere | Multi-thread is necessary only when iroh-gossip's spawned internal tasks need a runtime to drive (per B-4.1 spec §2). Pure-bincode roundtrip tests don't touch iroh. |
| **Bootstrap-from-runtime concern** | NOT in scope. B-4.2 does not plumb `bootstrap: Vec<PeerPubkey>` from the kernel `Runtime` to `IrohNetwork::subscribe`. The `Runtime::start` call site continues to pass `vec![]` (B-4.1 spec §3.1 plumbing comment unchanged). | Plumb bootstrap through `RuntimeCfg` | Bootstrap-discovery is independent of attribution; mixing them widens the PR diff for no semantic gain. Discovery is a B-4.4+ concern (see B-4.1 §11). |

## 3. Code surface

### 3.0 Type changes — `crates/types/src/dag.rs`

Mirror the `DriftMessage` / `DriftSignedPayload` shape onto `HeadsSummary` and `HeadsRequest`. Both struct definitions gain two new fields; both gain a sibling `*SignedPayload` struct.

**`HeadsSummary` (modify existing):**

```rust
/// `HeadsSummary` per convergence.md §4.2 + plan-B-4.2 spec §3.0.
///
/// The `signature` field covers [`HeadsSummarySignedPayload`] canonical
/// bytes (NOT the full `HeadsSummary`) and binds the message to its
/// topic via the explicit `topic` field on the signed payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeadsSummary {
    /// Per-author DAG-tip entries. Order is significant for canonical
    /// encoding; consult §7.1 for the sorting rule.
    pub authors: Vec<AuthorHead>,
    /// Version of the kernel's fuel table at the time of emission.
    /// Recipients with a different version know their pre-check
    /// metering may diverge from the authority verdict.
    pub kernel_fuel_table_version: u32,
    /// Ed25519 pubkey of the peer that emitted this heads summary.
    /// Excluded from the signed payload — the signer asserts the
    /// (`authors`, `kernel_fuel_table_version`, `topic`) triple, not
    /// the emitter identity. Per plan-B-4.2 spec §3.0.
    pub signed_by_peer: PeerPubkey,
    /// Ed25519 signature over the canonical bincode encoding of
    /// [`HeadsSummarySignedPayload`] constructed from this message's
    /// `authors` + `kernel_fuel_table_version` fields plus the
    /// recipient-known topic. See spec §3.0.
    #[serde(with = "crate::serde_helpers::serde_signature_64")]
    pub signature: [u8; 64],
}

/// Exact byte target signed by [`HeadsSummary::signature`].
///
/// Field order: `authors`, `kernel_fuel_table_version`, `topic` —
/// the first two mirror `HeadsSummary` declaration order; the third
/// (`topic`) is appended so the wire-freeze test can pin "first N
/// bytes of HeadsSummary equal first N bytes of HeadsSummarySignedPayload"
/// without disturbing the existing `authors` / `kernel_fuel_table_version`
/// order. Emit-side and verify-side MUST construct this struct
/// identically; deviation = signature divergence per spec §3.0.
/// `signed_by_peer` and `signature` are excluded from the signed
/// payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeadsSummarySignedPayload {
    /// Mirrors [`HeadsSummary::authors`].
    pub authors: Vec<AuthorHead>,
    /// Mirrors [`HeadsSummary::kernel_fuel_table_version`].
    pub kernel_fuel_table_version: u32,
    /// Topic that this signature is bound to. Prevents cross-topic
    /// replay — a signature valid for topic X must not verify when
    /// the same wire bytes are gossiped on topic Y. The wire form
    /// of `HeadsSummary` does NOT carry this field; the recipient
    /// reconstructs it from the local subscription that delivered
    /// the message (`self.topic` in `Runtime`).
    pub topic: crate::Topic,
}
```

**`HeadsRequest` (modify existing):**

```rust
/// Bundle of [`EventRequest`] values sent in a single wire message.
///
/// Per plan-B-4.2 spec §3.0: `signature` covers
/// [`HeadsRequestSignedPayload`] canonical bytes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeadsRequest {
    /// Range requests included in this bundle. Recipients SHOULD treat
    /// the bundle as a unit but MAY service entries independently.
    pub requests: Vec<EventRequest>,
    /// Ed25519 pubkey of the peer that emitted this heads request.
    pub signed_by_peer: PeerPubkey,
    /// Ed25519 signature over [`HeadsRequestSignedPayload`].
    #[serde(with = "crate::serde_helpers::serde_signature_64")]
    pub signature: [u8; 64],
}

/// Exact byte target signed by [`HeadsRequest::signature`].
///
/// Field order: `requests`, `topic`. `signed_by_peer` and `signature`
/// are excluded. Per plan-B-4.2 spec §3.0.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeadsRequestSignedPayload {
    /// Mirrors [`HeadsRequest::requests`].
    pub requests: Vec<EventRequest>,
    /// Topic that this signature is bound to. See
    /// [`HeadsSummarySignedPayload::topic`] for cross-topic-replay
    /// rationale; same shape.
    pub topic: crate::Topic,
}
```

**`crates/types/src/lib.rs`** re-exports:

```rust
pub use dag::{
    AuthorHead, AuthorSeq, DriftAnchor, DriftMessage, DriftSignedPayload, EventRequest, GenesisV1,
    HeadsRequest, HeadsRequestSignedPayload, HeadsSummary, HeadsSummarySignedPayload,
};
```

**Field-order rationale (normative).** For `HeadsSummary`, the message and signed-payload structs agree on the first two fields (`authors`, `kernel_fuel_table_version`); the signed payload appends `topic`, the message appends `signed_by_peer` + `signature`. For `HeadsRequest`, the message and signed-payload agree on the first field (`requests`); the signed payload appends `topic`, the message appends `signed_by_peer` + `signature`. This preserves the `DriftMessage` / `DriftSignedPayload` invariant that the FIRST n bytes of the message canonical encoding match the canonical encoding of the signed-payload's leading fields (modulo the appended topic — see §5 acceptance test "prefix-bytes-property"). The wire-freeze tests pin the property.

### 3.1 Publish-side signing — `crates/kernel/src/runtime.rs`

Three sign-and-publish sites need updates:

1. **`Runtime::publish_heads_summary`** at line 548. Currently constructs a bare `HeadsSummary` and publishes it. After:

```rust
async fn publish_heads_summary(&mut self) -> Result<(), RuntimeError> {
    let authors = self.dag.author_heads();
    let kernel_fuel_table_version = self.cfg.kernel_fuel_table_version;
    let signed_payload = HeadsSummarySignedPayload {
        authors: authors.clone(),
        kernel_fuel_table_version,
        topic: self.topic,
    };
    #[allow(clippy::expect_used)]
    let sign_bytes = canonical_bincode()
        .serialize(&signed_payload)
        .expect("canonical bincode of HeadsSummarySignedPayload is infallible");
    let signature = self.peer_key.sign(&sign_bytes);
    let summary = HeadsSummary {
        authors,
        kernel_fuel_table_version,
        signed_by_peer: self.peer_key.public,
        signature,
    };
    self.network
        .publish(self.topic, GossipMessage::HeadsSummary(summary))
        .await?;
    Ok(())
}
```

2. **`Runtime::request_author_chain_gap`** at line 847 (publishes a `HeadsRequest`). After:

```rust
async fn request_author_chain_gap(&mut self, author: AuthorPubkey, from_seq: u64, to_seq: u64) {
    if to_seq < from_seq || to_seq == 0 {
        return;
    }
    let mut requests = Vec::new();
    Self::paginate_into(author, from_seq, to_seq, &mut requests);
    if requests.is_empty() {
        return;
    }
    let req = self.build_signed_heads_request(requests);
    let _ = self
        .network
        .publish(self.topic, GossipMessage::HeadsRequest(req))
        .await;
}
```

3. **`Runtime::handle_heads_summary`** at line 922 (publishes a `HeadsRequest` after diffing). Same `build_signed_heads_request` helper.

New private helper on `Runtime`:

```rust
/// Construct a signed [`HeadsRequest`] for this runtime's topic.
/// Used by the two HeadsRequest publish sites (`request_author_chain_gap`
/// and `handle_heads_summary`). Inlining at both sites is identical
/// boilerplate; the helper deduplicates it without abstracting away
/// the sign-then-publish pattern. Per plan-B-4.2 spec §3.1.
fn build_signed_heads_request(
    &self,
    requests: Vec<myrhiza_types::EventRequest>,
) -> HeadsRequest {
    let signed_payload = myrhiza_types::HeadsRequestSignedPayload {
        requests: requests.clone(),
        topic: self.topic,
    };
    #[allow(clippy::expect_used)]
    let sign_bytes = canonical_bincode()
        .serialize(&signed_payload)
        .expect("canonical bincode of HeadsRequestSignedPayload is infallible");
    let signature = self.peer_key.sign(&sign_bytes);
    HeadsRequest {
        requests,
        signed_by_peer: self.peer_key.public,
        signature,
    }
}
```

Note: a parallel `build_signed_heads_summary` would only deduplicate two lines (the struct literal and the publish call); kept inline at the single publish site instead.

### 3.2 Receive-side verification — `crates/kernel/src/runtime.rs`

In `Runtime::handle_message` at line 614, change the dispatch to verify signatures before consuming the body. The new shape (parallels how `process_drift_message` does its own internal verify-first at lines 1448-1466):

```rust
async fn handle_message(&mut self, msg: GossipMessage) -> Result<(), RuntimeError> {
    match msg {
        GossipMessage::Event(e) => self.handle_event(e).await?,
        GossipMessage::HeadsSummary(h) => {
            if !self.verify_heads_summary(&h) {
                return Ok(());
            }
            self.handle_heads_summary(h).await?;
        }
        GossipMessage::HeadsRequest(r) => {
            if !self.verify_heads_request(&r) {
                return Ok(());
            }
            self.handle_heads_request(r).await?;
        }
        GossipMessage::Drift(d) => self.process_drift_message(d).await,
    }
    Ok(())
}
```

The two `verify_*` private fns:

```rust
/// Verify the signature on a [`HeadsSummary`]. Returns `true` if the
/// message should be processed, `false` if it should be dropped.
///
/// Follows the structural shape of `process_drift_message`
/// (`runtime.rs:1450-1466`) but extends it: the drift handler
/// **silently drops** on bad sig; the two `verify_*` fns push
/// `PeerWarning::SignatureInvalid { peer }` at the same decision
/// point, then return `false` so the body-consuming handler skips.
/// (Backfilling `PeerWarning::SignatureInvalid` into
/// `process_drift_message` is a follow-up — see §10.)
///
/// Loopback: `verify_*` returns `false` for own-published messages,
/// which causes the dispatch site to skip the body handler.
/// MemNetwork echoes own publishes (broadcast channel); IrohNetwork
/// does NOT (Plumtree). See §2 "Loopback filter" row.
///
/// Per plan-B-4.2 spec §3.2.
fn verify_heads_summary(&self, h: &HeadsSummary) -> bool {
    // Loopback filter — MemNetwork echoes own publishes through its
    // tokio broadcast channel; IrohNetwork does not (Plumtree).
    // Either way, our own HeadsSummary is a self-diff no-op; skip.
    if h.signed_by_peer == self.peer_key.public {
        return false;
    }
    let signed_payload = myrhiza_types::HeadsSummarySignedPayload {
        authors: h.authors.clone(),
        kernel_fuel_table_version: h.kernel_fuel_table_version,
        topic: self.topic,
    };
    let Ok(bytes) = canonical_bincode().serialize(&signed_payload) else {
        return false;
    };
    if myrhiza_manifest::verify_signature(h.signed_by_peer.as_bytes(), &bytes, &h.signature)
        .is_err()
    {
        #[allow(clippy::expect_used)]
        self.peer_warnings
            .lock()
            .expect("peer_warnings mutex poisoned")
            .push(PeerWarning::SignatureInvalid {
                peer: Some(h.signed_by_peer),
            });
        return false;
    }
    true
}

/// Verify the signature on a [`HeadsRequest`]. Same shape as
/// [`Self::verify_heads_summary`].
fn verify_heads_request(&self, r: &HeadsRequest) -> bool {
    if r.signed_by_peer == self.peer_key.public {
        return true;
    }
    let signed_payload = myrhiza_types::HeadsRequestSignedPayload {
        requests: r.requests.clone(),
        topic: self.topic,
    };
    let Ok(bytes) = canonical_bincode().serialize(&signed_payload) else {
        return false;
    };
    if myrhiza_manifest::verify_signature(r.signed_by_peer.as_bytes(), &bytes, &r.signature)
        .is_err()
    {
        #[allow(clippy::expect_used)]
        self.peer_warnings
            .lock()
            .expect("peer_warnings mutex poisoned")
            .push(PeerWarning::SignatureInvalid {
                peer: Some(r.signed_by_peer),
            });
        return false;
    }
    true
}
```

**New `PeerWarning` variant** at line 157:

```rust
/// Wire-signature verification failed for an inbound `HeadsSummary` or
/// `HeadsRequest`. The peer claimed to sign under `peer` but the
/// signature did not verify under that pubkey. Distinct from
/// [`PeerWarning::DecodeFailed`] — DecodeFailed means "I can't parse
/// the bytes"; SignatureInvalid means "I parsed it, the cryptographic
/// claim doesn't hold." Per plan-B-4.2 spec §2 (`SignatureInvalid` row).
SignatureInvalid {
    /// The claimed `signed_by_peer` from the message. NOT the
    /// last-hop delivered-from neighbor (that's `DecodeFailed`'s
    /// `peer` semantic). `None` is structurally unreachable for this
    /// variant in v1 (sig-failure paths always have a claimed peer);
    /// kept `Option` for future-shape consistency with
    /// [`PeerWarning::DecodeFailed`].
    peer: Option<PeerPubkey>,
},
```

### 3.3 Real unsubscribe — `crates/network/src/iroh_transport.rs`

Replace the `Err(NetError::Unimplemented { ... })` body with `Ok(())`. Update rustdoc to explain why the method is a semantic no-op and where the cleanup actually happens. After:

```rust
async fn unsubscribe(&self, _topic: Topic) -> Result<(), NetError> {
    // iroh-gossip 0.99.0 exposes no explicit "leave swarm" public API:
    // `iroh-gossip/src/api.rs` lines 355-363 documents that "Once the
    // GossipTopic is dropped, the network actor will leave the gossip
    // topic. … the topic will be left once both the GossipSender and
    // GossipReceiver halves are dropped." Drop IS the v1 leave
    // mechanism.
    //
    // `IrohNetwork` itself does not own any `GossipTopic` handles —
    // those are inside each `IrohSubscription` instance held by
    // callers (typically `Runtime::start`'s `sub` local). The
    // load-bearing cleanup happens when the caller's subscription
    // exits scope, not when this method returns.
    //
    // Returning `Ok(())` here is therefore honest: the method itself
    // has nothing to do at the `IrohNetwork` level, and signaling
    // success matches the trait contract (the transport recognizes
    // the call and does not error). Calling `unsubscribe` then
    // continuing to hold the `IrohSubscription` will NOT stop the
    // subscription — callers MUST drop the subscription to actually
    // leave the topic. Per plan-B-4.2 spec §3.3.
    Ok(())
}
```

Optional bonus diagnostic (not in §1 scope but free): the method could log a `tracing::debug!` line at this point so observers can see "caller called unsubscribe; cleanup pending on subscription drop." Tracing infrastructure isn't yet wired in B-4.* — defer.

### 3.4 Wire-freeze regeneration — `crates/types/tests/wire_freeze.rs`

Six tests change, three tests are added:

**Modified tests:**

1. `heads_summary_wire_layout` at line 128 — size grows from 100 bytes. New size calculation:
   - `authors`: 8 (vec len = 1) + AuthorHead (40 + 8 + 40 = 88) = 96
   - `kernel_fuel_table_version`: 4 (u32 BE)
   - `signed_by_peer`: 40 (8 len + 32 bytes)
   - `signature`: 72 (8 len + 64 bytes)
   - **Total: 96 + 4 + 40 + 72 = 212 bytes**

2. `heads_request_wire_layout` at line 157 — size grows from 8 bytes. New size:
   - `requests`: 8 (vec len = 0)
   - `signed_by_peer`: 40
   - `signature`: 72
   - **Total: 120 bytes**

3. `gossip_message_heads_summary_variant_tag_is_one_u32_be` at line 233 + `gossip_message_heads_request_variant_tag_is_two_u32_be` at line 246 stay structurally unchanged; the variant tag stays 1 / 2. Replace `sample_heads_summary()` / `sample_heads_request()` helpers to produce signed values (any deterministic zero-bytes signature works for the tag test).

4. `sample_heads_summary` + `sample_heads_request` helpers at lines 195-203 — update bodies to include `signed_by_peer: PeerPubkey::from_bytes([0; 32])` and `signature: [0; 64]`.

**New tests** (mirror the existing drift trio):

```rust
#[test]
fn heads_summary_signed_payload_field_order_is_authors_fuel_topic() {
    let p = HeadsSummarySignedPayload {
        authors: vec![],
        kernel_fuel_table_version: 1,
        topic: Topic::from_bytes([0xAB; 32]),
    };
    let bytes = canonical_bincode().serialize(&p).expect("encode");
    // authors: 8 (vec len = 0)
    // kernel_fuel_table_version: 4 (u32 BE)
    // topic: 40 (8 u64-BE len-prefix + 32 raw bytes, serde_bytes shape
    //        per crates/types/src/topic.rs:13 using serde_bytes_32_pub)
    // total: 8 + 4 + 40 = 52
    assert_eq!(bytes.len(), 52);
    // Topic's 32 raw bytes follow its 8-byte length prefix: [20..52].
    assert_eq!(&bytes[20..52], &[0xAB; 32]);
}

#[test]
fn heads_request_signed_payload_field_order_is_requests_topic() {
    let p = HeadsRequestSignedPayload {
        requests: vec![],
        topic: Topic::from_bytes([0xCD; 32]),
    };
    let bytes = canonical_bincode().serialize(&p).expect("encode");
    // requests: 8 (vec len = 0)
    // topic: 40 (8 len-prefix + 32 raw)
    // total: 48
    assert_eq!(bytes.len(), 48);
    assert_eq!(&bytes[16..48], &[0xCD; 32]);
}

#[test]
fn heads_summary_first_n_bytes_match_signed_payload_leading_fields() {
    // Spec §3.0 normative: HeadsSummary's `authors` +
    // `kernel_fuel_table_version` canonical bytes (the first two fields
    // in declaration order) MUST byte-match the first two fields of
    // HeadsSummarySignedPayload. The signed payload then appends `topic`
    // which does NOT appear on the wire; the message appends
    // `signed_by_peer` + `signature` which DO appear on the wire but
    // are NOT in the signed payload.
    let authors = vec![AuthorHead {
        author: AuthorPubkey::from_bytes([1; 32]),
        seq: 5,
        hash: EventHash::ZERO,
    }];
    let kernel_fuel_table_version = 7;

    let signed = HeadsSummarySignedPayload {
        authors: authors.clone(),
        kernel_fuel_table_version,
        topic: Topic::from_bytes([0xAA; 32]),
    };
    let signed_bytes = canonical_bincode().serialize(&signed).expect("encode");

    let msg = HeadsSummary {
        authors,
        kernel_fuel_table_version,
        signed_by_peer: PeerPubkey::from_bytes([0xFF; 32]),
        signature: [0x11; 64],
    };
    let msg_bytes = canonical_bincode().serialize(&msg).expect("encode");

    // First (signed_bytes.len() - 40) bytes match — i.e. all of
    // signed_bytes EXCEPT the trailing `topic` field (40 = 8 len-prefix
    // + 32 raw per Topic's serde_bytes_32_pub shape).
    let common_prefix_len = signed_bytes.len() - 40;
    assert_eq!(
        &msg_bytes[..common_prefix_len],
        &signed_bytes[..common_prefix_len],
        "HeadsSummary canonical bytes must prefix-match HeadsSummarySignedPayload's leading fields (spec §3.0)"
    );
}

#[test]
fn heads_request_first_n_bytes_match_signed_payload_leading_fields() {
    let requests = vec![EventRequest {
        author: AuthorPubkey::from_bytes([8; 32]),
        from_seq: 1,
        to_seq: 10,
    }];

    let signed = HeadsRequestSignedPayload {
        requests: requests.clone(),
        topic: Topic::from_bytes([0xAA; 32]),
    };
    let signed_bytes = canonical_bincode().serialize(&signed).expect("encode");

    let msg = HeadsRequest {
        requests,
        signed_by_peer: PeerPubkey::from_bytes([0xFF; 32]),
        signature: [0x11; 64],
    };
    let msg_bytes = canonical_bincode().serialize(&msg).expect("encode");

    // Topic is 40 bytes (8 len-prefix + 32 raw); see preceding test.
    let common_prefix_len = signed_bytes.len() - 40;
    assert_eq!(
        &msg_bytes[..common_prefix_len],
        &signed_bytes[..common_prefix_len],
        "HeadsRequest canonical bytes must prefix-match HeadsRequestSignedPayload's leading fields (spec §3.0)"
    );
}
```

**Inline `dag.rs` round-trip tests** (in the `tests_drift_heads` module at line 230 of `dag.rs`): two new tests + update the existing `heads_summary_round_trips` and `heads_request_round_trips` to include the new fields. New tests pin the `*SignedPayload` round-trip property:

```rust
#[test]
fn heads_summary_signed_payload_round_trips() {
    let p = HeadsSummarySignedPayload {
        authors: vec![],
        kernel_fuel_table_version: 1,
        topic: crate::Topic::from_bytes([0xAB; 32]),
    };
    let bytes = canonical_bincode().serialize(&p).expect("encode");
    let decoded: HeadsSummarySignedPayload =
        canonical_bincode().deserialize(&bytes).expect("decode");
    assert_eq!(decoded.kernel_fuel_table_version, p.kernel_fuel_table_version);
    assert_eq!(decoded.topic, p.topic);
}

#[test]
fn heads_request_signed_payload_round_trips() {
    let p = HeadsRequestSignedPayload {
        requests: vec![],
        topic: crate::Topic::from_bytes([0xCD; 32]),
    };
    let bytes = canonical_bincode().serialize(&p).expect("encode");
    let decoded: HeadsRequestSignedPayload =
        canonical_bincode().deserialize(&bytes).expect("decode");
    assert_eq!(decoded.requests.len(), 0);
    assert_eq!(decoded.topic, p.topic);
}
```

### 3.5 Call-site impact

Within the kernel crate, every literal construction of `HeadsSummary { ... }` and `HeadsRequest { ... }` needs the two new fields populated. Sites identified via grep:

- `crates/kernel/src/runtime.rs:549` — `publish_heads_summary` (only construction of `HeadsSummary` in the runtime; handled in §3.1).
- `crates/kernel/src/runtime.rs:860` — `request_author_chain_gap` HeadsRequest construction (handled in §3.1).
- `crates/kernel/src/runtime.rs:928` — `handle_heads_summary`'s response-HeadsRequest construction (handled in §3.1).

The `handle_heads_summary(h)` and `handle_heads_request(r)` receivers do not pattern-match into the new fields — they consume `authors` / `kernel_fuel_table_version` / `requests` and ignore `signed_by_peer` + `signature` (verification happened at dispatch). No body change required there.

Test sites that construct these types directly:

- `crates/types/tests/wire_freeze.rs:195-203` — `sample_heads_summary` + `sample_heads_request` helpers (§3.4).
- `crates/types/src/dag.rs:271-297` — existing `heads_summary_round_trips` + `heads_request_round_trips` (§3.4).
- `crates/kernel/tests/*` — none of the existing kernel tests construct `HeadsSummary` or `HeadsRequest` literals; they go through the runtime. The acceptance tests added in §4 will be the only test-site changes.

## 4. Acceptance tests

### 4.1 Pure-types tests — `crates/types/src/dag.rs`'s `tests_drift_heads` module

Updates + new tests are listed in §3.4 above. No new file.

### 4.2 Wire-freeze tests — `crates/types/tests/wire_freeze.rs`

Updates + new tests listed in §3.4. Existing variant-tag tests stay green; sizes for HeadsSummary / HeadsRequest layouts regenerate.

### 4.3 Runtime sign/verify tests — `crates/kernel/tests/attribution.rs` (new file)

| # | Test name | Flavor | Pattern |
|---|---|---|---|
| 1 | `heads_summary_sign_then_verify_roundtrips` | default | Construct a `HeadsSummary` + `HeadsSummarySignedPayload` via the same `build_signed_heads_summary` shape as the runtime; verify the sig via `myrhiza_manifest::verify_signature` against the canonical bincode of the signed payload reconstructed by the verifier. Test passes when `Ok(())`. Covers: `convergence.md §4.2` + `dag.rs:150-193` shape. |
| 2 | `heads_request_sign_then_verify_roundtrips` | default | Same shape as #1 for `HeadsRequest` / `HeadsRequestSignedPayload`. |
| 3 | `verify_rejects_bad_signature_heads_summary` | default | Construct a signed `HeadsSummary`, flip a bit in `signature`, assert verify returns `Err(_)`. Covers `dag.rs` sig-flip robustness. |
| 4 | `verify_rejects_bad_signature_heads_request` | default | Same shape for `HeadsRequest`. |
| 5 | `verify_rejects_cross_topic_replay_heads_summary` | default | Sign a `HeadsSummary` for topic X. Construct a `HeadsSummarySignedPayload` with the same `authors` + `kernel_fuel_table_version` but `topic = Y`. Serialize, attempt to verify the X-signature against the Y-bytes — assert `Err(_)`. This proves the topic field in the signed payload is what blocks replay (NOT the wire bytes; the message wire bytes don't carry topic). Covers spec §3.0 "topic-binding choice" + §2 row. |
| 6 | `verify_rejects_cross_topic_replay_heads_request` | default | Same shape for `HeadsRequest`. |
| 7 | `runtime_drops_heads_summary_with_bad_signature` | `multi_thread, worker_threads = 2` | Construct two `Runtime` instances over a shared `MemNetwork`. Peer A publishes a HAND-FORGED `HeadsSummary` via `MemNetwork::publish` directly (bypassing `Runtime::publish_heads_summary`), with a deliberately wrong signature (zero bytes). Peer B's runtime receives it; assert (a) `B.peer_warnings` accumulates `PeerWarning::SignatureInvalid { peer: Some(a_peer) }`; (b) the body-consuming handler does NOT run — verify this by inspecting that peer B did not publish a backfill `HeadsRequest` in response (the bad-sig HeadsSummary should be dropped before `handle_heads_summary` runs the diff). Covers spec §3.2 + the runtime drop semantics. **MemNetwork** is the load-bearing transport here: the test does not need iroh's network stack, just the runtime's dispatch logic. |
| 8 | `runtime_drops_heads_request_with_bad_signature` | `multi_thread, worker_threads = 2` | Same shape as #7 for `HeadsRequest`. Verify: (a) `SignatureInvalid` warning surfaces; (b) `handle_heads_request` does NOT run — peer B did not publish the requested events. |
| 9 | `runtime_accepts_heads_summary_with_good_signature` | `multi_thread, worker_threads = 2` | Sanity check counterpart to #7: peer A's *real* `publish_heads_summary` produces a HeadsSummary peer B accepts (no `SignatureInvalid` warning; handler runs; the existing behavior — `KernelFuelTableMismatch` / backfill — is preserved). Ensures B-4.2 doesn't break existing convergence tests. |
| 10 | `runtime_loopback_filter_skips_own_heads_summary_verify` | `multi_thread, worker_threads = 2` | One-peer runtime over **`MemNetwork`** (which echoes own publishes via its tokio broadcast channel) publishes a `HeadsSummary`. Assert `peer_warnings` does NOT accumulate `SignatureInvalid` (loopback filter triggered, `verify_heads_summary` returns `false`, body handler skipped). NOTE: this scenario only fires on MemNetwork — `IrohNetwork` doesn't echo own broadcasts (Plumtree never emits `OutEvent::EmitEvent(Received(_))` for own messages). Verifies spec §2 "Loopback filter" row. |

### 4.4 Iroh transport unsubscribe test — extend `crates/network/tests/iroh_gossip.rs`

The existing test `unsubscribe_returns_unimplemented` (B-4.1 test #4) becomes invalid because the invariant flips. **DELETE that test** and replace with:

| # | Test name | Flavor | Pattern |
|---|---|---|---|
| 11 | `unsubscribe_returns_ok` | default | Construct an `IrohNetwork` + topic, call `unsubscribe(topic).await`, assert `Ok(())`. Pure invariant test — does NOT need a real swarm, does NOT need to verify that future `recv()` calls block (that's drop-on-subscription, not unsubscribe). Covers spec §3.3. |
| 12 | `iroh_subscription_drop_actually_leaves_swarm` | `multi_thread, worker_threads = 2` | Two iroh peers; peer A subscribes; peer B subscribes with `bootstrap = [a_id]`. Wait for swarm formation. Peer B drops its subscription. Peer A publishes a new message; assert peer B's network handle no longer receives it (validated by construction: the dropped subscription cannot `recv`). **TRADEOFF (acceptance-test scope):** verifying the *swarm-level* leave (peer A's iroh-gossip actor receives a "B left" signal) requires API access iroh-gossip doesn't expose at v0.99.0; this test proves the user-visible behavior (drop = stop receiving) which is what the spec contracts on. The actor-internal swarm-state assertion is a B-4.3 cross-process concern. |

### 4.5 Spec-coverage annotations (informative, applied in plan not spec)

Tests 1, 2 → `dag.rs:150-193` + `convergence.md §4.2`.
Tests 3, 4 → `convergence.md §4.4` (signature-integrity).
Tests 5, 6 → `convergence.md §4.6` (topic identity / cross-topic isolation).
Tests 7, 8 → `runtime.rs handle_message` dispatch / spec §3.2.
Test 9 → `convergence.md §4.2` happy path.
Test 10 → `runtime.rs:1450-1452` loopback parallel.
Tests 11, 12 → spec §3.3 + `iroh-gossip/api.rs:207` (`GossipTopic` rustdoc: drop = leave swarm).

## 5. Justfile changes

None expected. The existing `just test` and `just test-iroh` recipes cover the new + modified tests.

## 6. Edge cases

- **Empty authors list still signs/verifies.** `HeadsSummary { authors: vec![], kernel_fuel_table_version: N, signed_by_peer: P, signature: S }` is the boot-time first-publish shape. The signed payload is `{ authors: vec![], kernel_fuel_table_version: N, topic: T }` — empty vec serializes as `vec len = 0` (8 bytes), then 4 bytes for `kernel_fuel_table_version`, then 32 bytes for topic. Signature covers 44 deterministic bytes, all under peer P's control. No special-case in sign/verify; the empty-vec path is a regular case. Test 1 + test 9 both touch this implicitly; if an explicit "empty authors" regression test ever becomes needed, add it as test #13.
- **Empty requests list still signs/verifies.** Same logic for `HeadsRequest`. `HeadsRequest { requests: vec![], signed_by_peer, signature }` is a valid wire message; verification doesn't reject it. The runtime's `handle_heads_request` already handles empty `req.requests` correctly (the `for r in req.requests` loop just doesn't iterate). Test 2 + test 9 cover.
- **Old wire bytes from B-4.1 fail decode loudly.** A peer running B-4.1 binaries sends a `HeadsSummary` without `signed_by_peer` / `signature` fields. B-4.2 binaries attempt `canonical_bincode().deserialize::<GossipMessage>(&bytes)`; bincode's strict-decode mode (per `crates/types/src/encoding.rs:39-50` and the `decode_canonical` discipline) rejects the bytes as truncated (the trailing bytes for `signed_by_peer` + `signature` are missing). The `IrohSubscription::recv` decode-failure path at `iroh_transport.rs:218-225` returns `Err(SubError::DecodeFailed { peer })`, which the runtime surfaces as `PeerWarning::DecodeFailed`. Pre-launch this is the desired loud-failure behavior — no peers exist in production yet. **NOT acceptance-tested** (would require building a B-4.1 binary in CI); the trustworthy assertion is the existing `decode_failure_surfaces_as_subscribe_decode_failed` test (B-4.1 test #3) which already covers garbage-bytes → DecodeFailed.
- **A peer claiming `signed_by_peer = X` but with a sig that doesn't verify under X's pubkey.** Verify returns Err; runtime pushes `PeerWarning::SignatureInvalid { peer: Some(X) }`; message dropped. Test 7 covers. The runtime cannot tell if the message was *fraudulently* attributed to X (some other peer Y forged it under X's pubkey claim) or X simply messed up its own signing — both surface identically as "sig didn't verify under X's claimed pubkey." This is correct: the runtime knows only what the signature verifies against, not the underlying peer's intent.
- **A peer at iroh-gossip layer (delivered_from = Y) forwarding a HeadsSummary legitimately signed by X.** Plumtree forwarding is exactly this case. Verification under X's pubkey succeeds (the bytes are X's bytes; Y is just a transport hop). The `SubError::DecodeFailed { peer }` field would carry `Some(Y)` if decode failed, but for *signature* verification we use `signed_by_peer` (claimed = X), NOT the last-hop. The runtime treats the message as authentic from X; correct.
- **Cross-topic replay attempt under attribution.** Attacker captures peer X's signed `HeadsSummary` for topic A. Re-broadcasts the exact same bytes on topic B. Recipient on topic B reconstructs `HeadsSummarySignedPayload` with `topic = B` (the recipient's local self.topic). Canonical bincode of the reconstructed payload differs from canonical bincode of the *signed* payload (which had `topic = A`). Verify under X's pubkey fails. `PeerWarning::SignatureInvalid { peer: Some(X) }` surfaces. Tests 5 + 6 are the deterministic version of this property; test 7 + 8 are the runtime-integration version.
- **Cross-peer drift detection (`process_drift_message`) is unchanged.** B-4.2 does not touch DriftMessage / DriftSignedPayload; that path is already attribution-attested via the existing peer-key signature.
- **`Topic` newtype uses `#[serde(with = "crate::hash::serde_bytes_32_pub")]` over `[u8; 32]`** (per `crates/types/src/topic.rs:13`). Canonical bincode encodes this as `8-byte u64 length-prefix + 32 raw bytes = 40 bytes total` (NOT 32 raw — the `serde_bytes::Bytes` shape adds the length prefix). The `*SignedPayload` size calculations in §3.4 use this 40-byte total; if `Topic`'s serde shape ever changes (e.g. switched to `#[serde(transparent)]`), the wire-freeze tests catch the discrepancy.
- **`PeerKeypair::sign` is infallible** (per `crates/kernel/src/identity/mod.rs:69`). The `clippy::expect_used` annotations on canonical_bincode().serialize calls match the pattern already in `maybe_emit_drift:1421-1424`; the schema is fixed-shape and bincode does not fail on it. If a future schema change introduces a serialization-failure mode, those expect annotations become bugs — caught by review, not by tests.

## 7. Surface change summary

**Type changes in `myrhiza_types::dag`** (wire-breaking, pre-launch acknowledged-OK):
- `HeadsSummary` gains `signed_by_peer: PeerPubkey` + `signature: [u8; 64]` fields.
- `HeadsRequest` gains `signed_by_peer: PeerPubkey` + `signature: [u8; 64]` fields.
- New `HeadsSummarySignedPayload { authors, kernel_fuel_table_version, topic }` struct.
- New `HeadsRequestSignedPayload { requests, topic }` struct.
- Both new structs `pub`-exported from `myrhiza_types`.

**Behavior changes in `myrhiza_kernel::runtime`**:
- `publish_heads_summary` now signs the payload before publishing.
- `request_author_chain_gap` + the inline HeadsRequest publish in `handle_heads_summary` now sign via new helper `build_signed_heads_request`.
- `handle_message` dispatches via new verify-then-handle wrapping for HeadsSummary + HeadsRequest.
- Two new private fn-on-Runtime: `verify_heads_summary`, `verify_heads_request`.
- New `PeerWarning::SignatureInvalid { peer: Option<PeerPubkey> }` variant.

**Behavior change in `myrhiza_network::iroh_transport`**:
- `IrohNetwork::unsubscribe` returns `Ok(())` instead of `Err(NetError::Unimplemented { ... })`. Rustdoc updated to document that load-bearing cleanup happens via subscription drop, not via the method body.

**Wire-freeze regenerated** (`crates/types/tests/wire_freeze.rs`):
- `heads_summary_wire_layout`: size 100 → 212.
- `heads_request_wire_layout`: size 8 → 120.
- 4 new layout/prefix-property tests for the new `*SignedPayload` structs.
- `sample_heads_summary` + `sample_heads_request` helpers updated to include the new fields (zero-bytes default suffices for variant-tag tests).
- Variant-tag tests stay green (variant *order* in `GossipMessage` does NOT change).

**Modified existing files**:
- `crates/types/src/dag.rs` — type definitions + inline round-trip tests.
- `crates/types/src/lib.rs` — re-export new struct names.
- `crates/types/tests/wire_freeze.rs` — regenerated sizes + new prefix tests.
- `crates/network/src/iroh_transport.rs` — `unsubscribe` body + rustdoc.
- `crates/kernel/src/runtime.rs` — sign/verify + new PeerWarning variant.

**New files**:
- `crates/kernel/tests/attribution.rs` — sign/verify acceptance tests.

## 8. Non-goals (explicit)

- **No HeadsRequest direct-streams.** Point-to-point delivery via a new ALPN + Router protocol-handler is deferred to B-4.3 alongside cross-process tests (they share the Router infrastructure).
- **No halt detection on persistent `ApiError` mid-stream.** B-4.1 spec §6 spin behavior preserved.
- **No lag-count fidelity.** Iroh-gossip 0.99.0 drops the count internally; sentinel-0 preserved.
- **No NeighborUp/Down observability.** Membership events silently consumed.
- **No new ALPN registration / Router protocol-handler dispatch.** B-4.3.
- **No publish-side topic caching.** Re-subscribes per publish per B-4.1.
- **No bootstrap-discovery plumbing through `Runtime::start`.** Continues to pass `vec![]`.
- **No wire-version envelope.** Pre-launch wire-freeze break, no version bump.
- **No cross-process tests.** B-4.3.
- **No Event-level changes.** `Event::signature` already carries author-attestation; nothing in B-4.2 touches the event substrate.

## 9. Prior-art consultation

Consulted via the `using-prior-art` skill, 2026-05-20:

- **`prior-art/iroh/identity.md` §"NodeID = Ed25519 public key" (lines 5-19)** — confirms iroh's `EndpointId` and Myrhiza's `PeerPubkey` are the same primitive (raw 32-byte Ed25519 pubkey). The `signed_by_peer: PeerPubkey` field B-4.2 adds to `HeadsSummary` + `HeadsRequest` reuses this primitive end-to-end; no translation layer at the attribution site. Also `§"Discovery — DNS, pkarr, and (opt-in) mainline DHT" (lines 39-47)` — "All three publish-paths are *signed* — a discovery record is only valid if its signature matches the EndpointID it claims to describe." Same property B-4.2 establishes for HeadsSummary / HeadsRequest: claimed `signed_by_peer` MUST match the verifying key.
- **`prior-art/willow/identity.md` §"Ed25519 as identity root" (lines 10-41)** — Willow's `pack(payload, identity) / unpack(bytes) -> (T, EndpointId)` (lines 35-37) IS the signed-envelope wire form B-4.2 mirrors. Willow's `pack_profile / unpack_profile` (lines 38-41) "add a `peer_id` cross-check to defeat profile spoofing — a profile claiming to be Alice signed by Mallory's key returns `IdentityError::PeerMismatch`" — that property is exactly what `verify_heads_summary` / `verify_heads_request` enforce in B-4.2: the claimed `signed_by_peer` must match the verifying key on the signature. Also Willow's `Identity::verify()` "is wired through `iroh-base`'s `ed25519_dalek::VerifyingKey::verify_strict` (RFC 8032 strict mode), closing Ed25519 signature-malleability vectors" (lines 30-34) — `myrhiza_manifest::verify_signature` uses the same underlying primitive via the workspace's `ed25519-dalek`; the malleability-closure property is inherited.
- **`prior-art/willow/crypto.md` §"DMs deferred to MLS-over-Willow" (lines 92-118)** — captures the lesson that "deniability claims were structurally false (the real Ed25519 signature non-repudiably binds the author once the rumor plaintext is recovered)" (lines 111-113). Same property here: a peer signing a `HeadsSummary` non-repudiably binds itself to the (authors, kernel_fuel_table_version, topic) triple under its `peer_key`. No attempt at deniability for sync-protocol attribution; the kernel does not need it and pretending otherwise would mislead future spec authors.
- **`prior-art/mls/protocol.md` §"Wire format" (lines 96-103)** — MLS uses TLS-style binary presentation; transcript hashes commit to canonical wire bytes. Myrhiza uses canonical bincode for the same reason: byte-stable signing target. The two `*SignedPayload` structs in B-4.2 follow the same discipline — the signed bytes are exactly canonical bincode of the payload struct, with no transformation between sign-side and verify-side. Direct analog of MLS's transcript-hash discipline at a single-message scope.
- **`prior-art/mls/protocol.md` §"MLS application messages" (lines 105-117)** — `PrivateMessage` has an `authenticated_data` field "(AAD — visible to DS, integrity-protected)". Distinguish: `topic` in our `HeadsSummarySignedPayload` is integrity-protected (covered by signature) but NOT carried on the wire (recipient reconstructs it). The MLS AAD pattern is on-wire-and-integrity-protected; our topic-binding is integrity-protected-only. The Myrhiza choice trades one byte-pattern-on-the-wire for one fewer reconstruction-divergence risk — wire and signed bytes can't ever disagree about topic because wire doesn't carry it. Documented as a deliberate divergence from MLS shape.

**Runner-up paradigms rejected:**

- **Global `Signed<T>` envelope wrapping every `GossipMessage` variant** (per §2 row "Signed-envelope shape"). Rejected because (a) `Event::signature` already exists at the inner-Event level — wrapping `GossipMessage::Event(Event)` in a global `Signed` would double-sign; (b) the wire-freeze pattern would change for every variant, blowing up the diff size of B-4.2; (c) the per-variant pattern matches the existing `DriftMessage` precedent and the runtime's existing verify-then-dispatch shape. Direct precedent for the per-variant choice is `dag.rs:150-193`.
- **Outer-message `topic` field (signed AND on wire)** (per §2 row "Topic-binding location"). Rejected because the wire `topic` would be redundant — the recipient already knows it from the subscription that delivered the message — and worse, it would invite a class of bugs where wire-`topic` and signed-`topic` could end up disagreeing after a refactor (signature would still verify since the signed bytes are self-consistent; the message would then be processed under the wrong topic). Putting `topic` only in the signed payload eliminates this divergence by construction.
- **Direct peer-to-peer streams for HeadsSummary / HeadsRequest** (per B-4.1 spec §9 "Runner-up paradigms rejected"). Rejected for HeadsSummary (broadcast semantics — every subscriber should see). Rejected for HeadsRequest in B-4.2 ONLY because the new-ALPN + Router-protocol-handler infrastructure colocates better with B-4.3's cross-process tests; HeadsRequest IS a natural fit for point-to-point semantically, just not blocked-on for attribution.
- **Halt the runtime on any signature failure** (per §2 row "PeerWarning::SignatureInvalid"). Rejected because one malicious peer could halt every honest peer's runtime by spamming bad-sig messages on a public topic. Sig failures are routine in adversarial conditions; non-fatal-with-observability is the load-bearing pattern.
- **`pub(crate)` visibility on `*SignedPayload` structs.** Rejected because cross-crate test forging (mirroring `perf_carryovers.rs:732`'s pattern) needs `pub`.

**Remaining gaps in the prior-art corpus** (candidate triggers for future research):

- **Topic-binding pattern formalization.** None of the consulted prior-art (iroh, willow, mls) describes a "signed-but-not-wire" field convention. The Myrhiza choice to put `topic` in the signed payload but not the wire is novel against the surveyed corpus. May be worth promoting as a Myrhiza-conventions doc once a third use-case lands (currently: just HeadsSummary + HeadsRequest, both in B-4.2).
- **MLS sender-data privacy patterns.** RFC 9420's `PrivateMessage` hides sender identity from the Delivery Service via per-message ephemeral keys (separate from the sender's long-term identity). Myrhiza's HeadsSummary attribution makes `signed_by_peer` plaintext on the wire — anyone observing gossip traffic sees who signed what. This is acceptable for v1 (sync-metadata privacy is not a stated goal), but if future work demands metadata-hiding for sync-protocol messages, MLS's sender-data approach (`prior-art/mls/protocol.md §"MLS application messages"`) is the model.
- **Cross-topic-replay precedent.** The audit didn't find a precedent in the corpus for explicit signed-topic-field. The Bitcoin-payment-channel `chan_id` field embedded in HTLCs is structurally similar (each signed message identifies its channel) but is on-wire + signed, not signed-only. Worth a flag for any future researcher: Myrhiza's topic-binding choice is intentional and load-bearing, not boilerplate.

## 10. Future work — explicit deferrals

- **B-4.3** — Real cross-process / multi-process acceptance tests + HeadsRequest direct-streams (new ALPN, Router protocol-handler dispatch) + halt detection on persistent `ApiError` + the lag-count-fidelity test (deferred from B-4.1 §4 prose).
- **B-4.4+** — Discovery / pkarr / DHT integration. Bootstrap currently caller-provided; plumb `Vec<PeerPubkey>` from `RuntimeCfg` through `Runtime::start` once a discovery primitive lands.
- **HeadsSummary / HeadsRequest rate-limiting at the kernel layer.** B-4.2 adds no rate-limit on signed messages; a peer with a valid signing key could still flood. Drift has an explicit rate limit (`runtime.rs RateLimit::try_emit`); a parallel rate-limit for HeadsSummary + HeadsRequest may become necessary if observed gossip volume becomes a load-bearing concern.
- **Sender-data privacy** for sync-protocol messages (per §9 gap). MLS-shape ephemeral-sender-key pattern if a metadata-hiding requirement emerges.
- **`NeighborUp` / `NeighborDown` observability** through `RuntimeHandle`.
- **Wire-version envelope** — first post-1.0 wire change becomes the trigger to land this.

## 11. Sources

- `crates/types/src/dag.rs:150-193` — `DriftMessage` + `DriftSignedPayload` precedent.
- `crates/kernel/src/runtime.rs:1448-1466` — `process_drift_message` verify-flow (model for B-4.2's HeadsSummary / HeadsRequest verify).
- `crates/kernel/src/runtime.rs:1414-1432` — `maybe_emit_drift` sign-flow (model for B-4.2's publish-side signing).
- `crates/kernel/src/runtime.rs:548-557` — `publish_heads_summary` site (modified in §3.1).
- `crates/kernel/src/runtime.rs:840-863` — `request_author_chain_gap` site (modified in §3.1).
- `crates/kernel/src/runtime.rs:872-933` — `handle_heads_summary` site (HeadsRequest publish at line 928).
- `crates/kernel/src/runtime.rs:614-622` — `handle_message` dispatch (modified in §3.2).
- `crates/kernel/src/runtime.rs:155-223` — `PeerWarning` enum (new variant added).
- `crates/kernel/src/identity/mod.rs:69-71` — `PeerKeypair::sign`.
- `crates/manifest/src/signature.rs:33` — `verify_signature` API.
- `crates/network/src/iroh_transport.rs:156-167` — current `unsubscribe` body (modified in §3.3).
- `crates/types/tests/wire_freeze.rs:128-162` — wire-freeze layout for HeadsSummary + HeadsRequest (modified in §3.4).
- `iroh-gossip-0.99.0/src/api.rs:207` — `GossipTopic` rustdoc: drop = leave swarm; no explicit leave API.
- `iroh-gossip-0.99.0/src/api.rs` lines 433-447 — `Event` enum.
- `iroh-gossip-0.99.0/src/api.rs` lines 449-462 — `Message` struct with `delivered_from`.
- `iroh-gossip-0.99.0/src/api.rs` lines 395-410 — `GossipReceiver::joined`.
- [`prior-art/iroh/identity.md`](../prior-art/iroh/identity.md) §"NodeID = Ed25519 public key" + §"Discovery — DNS, pkarr, and (opt-in) mainline DHT".
- [`prior-art/willow/identity.md`](../prior-art/willow/identity.md) §"Ed25519 as identity root".
- [`prior-art/willow/crypto.md`](../prior-art/willow/crypto.md) §"DMs deferred to MLS-over-Willow".
- [`prior-art/mls/protocol.md`](../prior-art/mls/protocol.md) §"Wire format" + §"MLS application messages".
- [`docs/specs/2026-05-09-myrhiza-master-design/convergence.md`](2026-05-09-myrhiza-master-design/convergence.md) §4.2 + §4.6 + §4.7.
- [`docs/specs/2026-05-10-plan-b-1-dag-memnet-design.md`](2026-05-10-plan-b-1-dag-memnet-design.md) §8.1 — DriftMessage spec.
- [`docs/specs/2026-05-20-plan-b-4-1-iroh-gossip-design.md`](2026-05-20-plan-b-4-1-iroh-gossip-design.md) §11 — Q-4 deferral pointing here + §6 spin-on-error invariant preserved.
- [`docs/specs/2026-05-20-plan-b-4-0-iroh-skeleton-design.md`](2026-05-20-plan-b-4-0-iroh-skeleton-design.md) — baseline iroh integration.