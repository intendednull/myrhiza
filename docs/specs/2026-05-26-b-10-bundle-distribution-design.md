**Date:** 2026-05-26
**Status:** draft
**Parent:** [docs/specs/2026-05-09-myrhiza-master-design/README.md](2026-05-09-myrhiza-master-design/README.md)
**Subject:** Plan B-10 — Bundle distribution polish + iroh-blobs fetch path

# Plan B-10 design — Bundle distribution + iroh-blobs

## 1. Goal

Close the last remaining v1-blocking gap in [`mvp.md`](2026-05-09-myrhiza-master-design/mvp.md) §15.1 criterion #1 — *"the kernel loads and instantiates a WASM state component from a bundle **fetched via iroh-blobs**"* — and finish [`implementation.md`](2026-05-09-myrhiza-master-design/implementation.md) §20 item 14 (currently 🟡: install/load from disk works, no revocation topic, no per-author publishing, no iroh-blobs fetch). Wire `iroh-blobs` 0.101.0 as the production fetch transport, evolve `BundleAddress` into an enum so the existing disk-loaded test/dev paths keep working, ship the [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.7 per-author revocation topic, and land the per-author publishing flow (publish blobs + emit availability event). After B-10 lands, all five v1 criteria are met against the wire shape the master spec actually commits to, not against a disk-only proxy.

## 2. Scope

### In v1 (this slice)

- **iroh-blobs fetch path** — given the manifest's BLAKE3 iroh-blobs hash, pull bytes from a peer, decode the manifest, pull each declared component's bytes by their manifest-declared hashes, verify each component's hash matches the manifest's `bundle_content_hash` recipe, hand the result to `InstallFlow::load`. Feature-gated under `network-iroh` alongside the existing iroh transport.
- **iroh-blobs publish path** — given a built bundle (manifest + N component bytes), import each artifact into a local `iroh-blobs` store, return the manifest hash as the `BundleAddress::IrohBlob` identifier the author shares out of band.
- **Per-author revocation topic** — derive `BLAKE3("myrhiza/revocations/v1" || author_pubkey)` per [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.7, auto-subscribe on install, gossip canonical-bincode-encoded `RevocationEvent { revoked_bundle_hash, reason, revoked_at_event_hash, revocation_seq }` envelopes, apply seq-monotonicity + per-window cap on apply.
- **Per-author publishing announcement** — derive `BLAKE3("myrhiza/publications/v1" || author_pubkey)` (new topic, mirror of revocation shape) and gossip `PublicationEvent { manifest_hash, version, signed_target }` so peers already subscribed to an author can discover new releases without out-of-band re-share. Per-bundle subscription happens at install time.
- **`BundleAddress` enum** — `Disk { bundle_dir, manifest_path }` (existing shape, kept for tests + offline dev) and `IrohBlob { manifest_hash: BlobHash }` (new). `InstallFlow::load` accepts both via the same entry point; new `BundleFetcher` materializes `IrohBlob → Disk` semantics into a tempdir under the hood so the post-fetch path is unchanged.
- **`crates/distribution/`** — new workspace member owning iroh-blobs fetch + publish + revocation/publication topic schema. Behind the `network-iroh` feature.
- **Test plan** — state-tier unit tests for revocation log apply (pure-function purity), kernel-tier acceptance test exercising publish-on-A + fetch-on-B over real iroh-blobs through `IrohHarness`, retain all existing disk-bundle tests untouched.

### Explicitly deferred

- **OCI layering** (`wkg` / Spin / wasmCloud convention) — runner-up paradigm, see §10. Not v1.
- **Bundle discovery without out-of-band share** — a new peer who has never installed an app from author A still gets the manifest hash through a link / QR / in-app share, exactly as [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.3 already commits. In-band catalog gossip is a future-direction child spec.
- **Cross-process iroh-blobs tests** — same deferral as E2E-2 covers for iroh-gossip; B-10 stays in-process (still real iroh-blobs through real QUIC over loopback via `IrohHarness`).
- **Revocation post-rotation / key recovery** — explicit single-key compromise risk acknowledged in [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.7 ("Author key compromise"). Future direction.
- **Sigstore / OIDC / X.509 signing overlays** — runner-up, see §10. Author-pubkey-signed-manifest is the v1 trust binding.
- **Lazy / partial component fetch** — fetch all declared component slots together; range-streaming a subset is a follow-up. (BLAKE3 + Bao tree supports it natively — see [`prior-art/iroh/blobs.md`](../prior-art/iroh/blobs.md).)
- **GUI / CLI publishing tooling** — B-10 ships the library + a `myrhiza-cli publish` subcommand smoke-test; rich publishing UX is post-v1.
- **Bundle tag / GC policy** — uses a single retention tag per installed bundle. LRU / quota / refcount is future work per [`prior-art/iroh/blobs.md`](../prior-art/iroh/blobs.md) §Tagging-and-GC ("Tags are the only retention primitive").
- **Subscription enumeration mitigations** — relay rotation / topic-subscription cover. Out of scope per [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.7 closing paragraph.

## 3. Scope decisions (locked during brainstorming + prior-art consultation, 2026-05-26)

These are the seven critical ambiguities the slice handoff named. Each has a decision, a runner-up, and the prior-art citations grounding the call.

### 3.1 Wire format: two blobs (manifest + components), not a combined tarball, not OCI

**Decision (a)**: the manifest is one iroh-blob; each declared component file is a separate iroh-blob; the manifest references components by their content hashes. Two-blob shape at minimum (manifest + state-apply); up to five if the bundle declares state-propose + interaction + behavior. Optional UI assets live in additional blobs referenced from the manifest.

**Runner-up (b) — combined tarball / `.happ`-shape blob**. Rejected because [`prior-art/holochain/distribution.md`](../prior-art/holochain/distribution.md) "Bundles are not seekable or streamable" makes the failure mode explicit: a gzip+MessagePack blob forces the entire bundle into RAM at install time and offers no native cross-bundle component deduplication. [`prior-art/app-distribution/lessons.md`](../prior-art/app-distribution/lessons.md) §Avoid row "Custom bundle format" calls this out directly ("Holochain's `.happ` is custom; no native signing; no registry ecosystem").

**Runner-up (c) — OCI artifact layered via `wkg`**. Rejected for v1 because [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.8 commits "No central registry. No sigstore dependency. No reliance on any centralized service for app distribution." OCI is the WASM-ecosystem mainstream answer ([`prior-art/app-distribution/lessons.md`](../prior-art/app-distribution/lessons.md) §Validates row 1), but OCI distribution assumes an HTTPS registry. [`prior-art/app-distribution/open-problems.md`](../prior-art/app-distribution/open-problems.md) §1 names "P2P distribution without a registry" as the open problem that requires exactly the iroh-blobs answer we're picking. A future plan may add an OCI publish/pull side-channel for cross-ecosystem interop — that's interop, not the v1 fetch path.

**Why two blobs and not a `HashSeq`**: [`prior-art/iroh/blobs.md`](../prior-art/iroh/blobs.md) describes `HashSeq` as a blob whose contents are a sequence of 32-byte hashes — perfect for an opaque collection. We instead carry component hashes inside the *typed manifest*, signed alongside everything else. The signed manifest already commits to `bundle_content_hash` (a recipe over up-to-four component slot hashes — see `crates/manifest/src/canonical.rs::bundle_content_hash`). Layering a `HashSeq` on top would be a second hash structure committing to the same component set; the signature already provides authenticated multi-blob binding. One indirection beats two.

### 3.2 Bundle hash semantics: BLAKE3 of canonical-bincode manifest bytes

**Decision (b)**: `BundleAddress::IrohBlob { manifest_hash: BlobHash }` carries the iroh-blobs hash of the **canonical-bincode-encoded manifest bytes**. The manifest itself commits to each component's BLAKE3 hash through the existing `signing_target_bytes` framing (the signature covers `length_prefix(DOMAIN_SEP) | length_prefix(manifest_canonical_hash) | length_prefix(content_hash) | length_prefix(version) | length_prefix(author_pubkey)` per [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.2 / §10.4).

Authentication chain on fetch:

1. Caller hands kernel a `BlobHash` (out-of-band share).
2. Fetcher pulls bytes from peer(s) — BLAKE3 + Bao verified streaming ([`prior-art/iroh/blobs.md`](../prior-art/iroh/blobs.md)) guarantees the fetched bytes hash to the expected `BlobHash`. Bytes are the canonical-bincode manifest.
3. Decode manifest via `canonical_bincode().deserialize`.
4. For each declared component slot in `Manifest::components`, fetch that component blob by the hash recorded in the manifest. iroh-blobs verifies each component's BLAKE3 hash matches the request hash.
5. Materialize the bundle into a tempdir mirroring the disk-bundle layout (`manifest.bincode` + `components/state-apply.wasm` etc.).
6. Hand the tempdir's `BundleAddress::Disk` to existing `InstallFlow::load`, which re-derives `bundle_content_hash` over the component bytes and verifies it matches the manifest's signed claim — that step is unchanged from the disk path.

**Why not (a) — hash of the combined blob**: there is no combined blob (we chose two-blob shape in §3.1). If we glued the blobs together for the sole purpose of hashing them, we'd lose component deduplication.

**Why not (c) — canonical Bech32m-encoded `(manifest, component_bytes)` tuple**: bech32m is the human-readable encoding ([`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.2 `wpub-` HRP). The iroh-blobs hash is the wire identifier; bech32m is the display encoding. Wrapping the raw hash in a bech32m bundle-share string at the share-the-link layer is fine — that's a UX surface, not the kernel identifier. Future plan may add a `BundleShareUri::Iroh { manifest_hash, bootstrap_peers }` formatter (akin to iroh's `BlobTicket`) that bech32m-encodes the same hash plus optional bootstrap peer hints; nothing in B-10 needs that.

**Determinism**: hashed bytes are canonical-bincode of the typed manifest struct (per [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.2 — "the manifest signature is computed NOT over the TOML text itself but over a **canonical bincode 1.3.x encoding** of the parsed manifest's typed structure"). The same struct on two peers produces the same bytes produces the same blob hash. No peer-local variation.

### 3.3 Revocation topic: per-author append-only log via gossip

**Decision (a)**: derive `topic_id = BLAKE3("myrhiza/revocations/v1" || author_pubkey)` exactly as [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.7 already pins. Every peer that has installed at least one bundle by author A auto-subscribes to A's revocation topic at install time and stays subscribed for the lifetime of the install. Revocations are gossiped as signed `RevocationEvent` envelopes.

The shape is the master spec's pinned commitment; this slice realizes it. The decision worth recording here is **why the topic is per-author and not per-bundle**.

**Runner-up (b) — per-bundle revocation on the bundle's own application topic**. Rejected: peers do not subscribe to bundles they don't have, so a peer who deferred installing version 1.4 has no subscription on which to receive the "1.4 is revoked, do not install" message. The point of revocation is to warn peers *before* they install — per-author beats per-bundle structurally.

**Runner-up (c) — global revocation registry**. Rejected per [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.8 "No central registry" commitment and [`prior-art/app-distribution/lessons.md`](../prior-art/app-distribution/lessons.md) §Avoid row "Tying Myrhiza to a single registry vendor."

**Why not Sigstore-shape transparency log**: [`prior-art/app-distribution/signing.md`](../prior-art/app-distribution/signing.md) "How Cosign signing flow works" shows the Rekor model — append-only, signed, third-party-witnessed. The pattern is sound. The Sigstore-as-deployed answer requires a third-party log operator (Rekor.sigstore.dev), which violates the no-central-service commitment. The *primitive* (signed append-only log) is exactly what `revocation-seq: u64` monotonicity per author already buys us per [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.7 — gossip + Ed25519-signed-by-author + monotonic seq is structurally a Rekor-shape log distributed across the author's followers. Future plan may add a third-party witness ([`prior-art/holochain/distribution.md`](../prior-art/holochain/distribution.md) DPKI is the comparable peer-symmetric attempt, also not central). Not v1.

**Concurrent-revoke conflict**: master spec §10.7 already commits monotonic `revocation-seq` per author plus `MAX_REVOCATION_JUMP = 1024 per 24h window`. Two concurrent revocations under the same seq from a compromised key collide deterministically (last-write-wins is well-defined because both events are append-only signed by the same key). One published per (author, seq); the kernel rejects the second. This is fine: the seq is monotonic at the author's choosing, so a careful author always increments. A compromised key publishing concurrent (author, seq=N) events is the single-key-compromise threat already acknowledged by §10.7 ("the kernel cannot distinguish; users must out-of-band verify").

### 3.4 Per-author publishing flow: separate publication topic carrying the manifest hash

**Decision (a)** with a refinement: author signs manifest locally → imports manifest + each component into local iroh-blobs store → emits a signed `PublicationEvent` on a per-author publication topic. New-peer discovery is out-of-band link/QR/in-app share for v1 (existing master commitment §10.3); but **existing followers of the author** — peers who already installed any bundle from author A — receive announcements via the publication topic automatically without separate share.

**Publication topic derivation**: `BLAKE3("myrhiza/publications/v1" || author_pubkey)`. Same shape as revocation topic.

**Publication event shape**:

```rust
struct PublicationEvent {
    /// iroh-blobs hash of the canonical-bincode manifest.
    manifest_hash: BlobHash,
    /// Same value embedded in the manifest's [app] section; carried
    /// here so peers can render a release notification before fetching
    /// the manifest bytes.
    version: String,
    /// Signature by the author's pubkey over the canonical-bincode
    /// encoding of (manifest_hash || version || sequence).
    publication_seq: u64,
    signature: Signature,
}
```

The `publication_seq` field mirrors `revocation-seq` semantics — monotonic per author, gates against replay + flood. Kernel rejects out-of-order or duplicate seqs.

**Why a separate topic and not piggyback on the revocation topic**: separation of concerns at the wire level. A peer scanning revocations gets exactly that; a peer scanning publications gets exactly that. Topic-ID structure surfaces semantics in the metadata trail a relay sees (which is already privileged per [`prior-art/iroh/lessons.md`](../prior-art/iroh/lessons.md) §Avoid "Relay metadata is privileged"). Future plan could merge if topic-count proliferation becomes a problem; one extra topic per installed author is well within iroh-gossip's per-peer subscription budget.

**Runner-up (b) — push blob to iroh-blobs network + emit availability event separately**. Same as the decision in different words — iroh-blobs publication and topic announcement are different transports. The decision is to do both: import into the local iroh-blobs store (which becomes the seed source), gossip the announcement (which is how peers learn about it). The author's iroh-blobs store advertises through iroh's normal connection establishment (pkarr-on-Mainline-DHT discovery for the author's `EndpointAddr` per [`prior-art/iroh/identity.md`](../prior-art/iroh/identity.md)).

**Runner-up (c) — out-of-band only (no in-band announcement)**. Rejected because peers who already follow the author benefit from automatic update notification, exactly the use case the per-app-update consent flow in [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.5 step 7 ("Per-update consent") is built for. Without a publication topic, every update is yet another out-of-band link share.

### 3.5 Backwards compatibility with disk-loaded bundles: `BundleAddress` becomes an enum

**Decision (a)**: `BundleAddress` becomes a two-variant enum:

```rust
pub enum BundleAddress {
    /// Existing on-disk layout. Used by tests, by the dev workflow,
    /// and as the materialization target for the iroh-blobs fetch path.
    Disk {
        bundle_dir: PathBuf,
        manifest_path: PathBuf,
    },
    /// Production fetch path. Carries the iroh-blobs hash of the
    /// canonical-bincode manifest. Resolution happens via
    /// `BundleFetcher::fetch` which materializes a `Disk` variant
    /// in a tempdir, then hands that to `InstallFlow::load`.
    IrohBlob {
        manifest_hash: BlobHash,
    },
}
```

`InstallFlow::load(&BundleAddress)` dispatches on the variant. For `Disk`, the existing implementation runs unchanged (zero behavior delta for every current test). For `IrohBlob`, the kernel calls into the new `BundleFetcher` (held by the runtime; see §4.3) which:

1. Fetches the manifest blob.
2. Decodes the canonical-bincode manifest.
3. Fetches each declared component blob.
4. Writes the materialized layout into a `tempfile::TempDir`.
5. Returns a `Disk` `BundleAddress` pointing at the tempdir.

Then `InstallFlow::load` runs against the materialized `Disk` address — same code path as today.

**Why not (b) — iroh-blobs only, retire disk loading**: disk loading is load-bearing for two test tiers (state-apply unit tests + kernel-tier acceptance tests that don't need iroh) and for the dev workflow (an app author building locally should not need a running iroh-blobs node to instantiate their own bundle). Disk path stays.

**Why not (c) — both available with feature flag / runtime config**: a feature flag on `BundleAddress` itself would force every consumer to know which path they're on. Enum variants are the natural shape — the runtime decides at call time, no global switch.

**Backwards-compat for current call sites**:

```rust
// before B-10:
let addr = BundleAddress { bundle_dir, manifest_path };

// after B-10:
let addr = BundleAddress::Disk { bundle_dir, manifest_path };
```

This is a breaking change to `BundleAddress`'s constructor shape but a mechanical one. `crates/test-utils/src/bundle.rs` (the central fixture builder used by every kernel test) updates in one place — `build_signed_counter_bundle()` constructs a `Disk` variant. Test bodies that pattern-match `BundleAddress` need updating — `grep -rn "BundleAddress {" crates/ tests/` returns the call sites; the migration is local. No semantic change to the loaded behavior.

`crates/kernel/tests/acceptance.rs` gains one new test exercising the `IrohBlob` path (see §6).

### 3.6 Test strategy: mix — unit tests use disk, kernel-tier acceptance uses real iroh-blobs

**Decision (c) — mix**:

- **State-tier**: revocation log apply tests (pure-function purity, `RevocationLog::apply(event) -> RevocationLog`). No iroh, no kernel. Tests cover seq-monotonicity, MAX_REVOCATION_JUMP gate, double-revoke idempotence, signature verification.
- **Kernel-tier integration**: one acceptance test in `crates/kernel/tests/iroh_acceptance.rs` exercising the full publish-then-fetch loop via `IrohHarness`. Peer A imports a bundle into its iroh-blobs store, emits the `PublicationEvent` on the per-author topic, peer B (already subscribed) fetches by the announced manifest hash through real `iroh-blobs` over loopback QUIC, runs `InstallFlow::load`, instantiates the WASM, applies an event, asserts state. This is the test that closes `mvp.md §15.1 #1` against the iroh-blobs wire shape, not against the disk-only proxy.
- **Kernel-tier integration**: one revocation propagation test — author publishes, peer A installs, author publishes revocation, peer B (subscribed to author's revocation topic via A's prior install) receives + applies revocation, asserts kernel surface flags the install for uninstall prompt.
- **Existing acceptance tests** under `crates/kernel/tests/acceptance.rs`: untouched. They continue to use `Disk` bundles.

**Why not (a) — real iroh-blobs everywhere**: existing state-tier unit tests don't need iroh and shouldn't pay its setup cost. The kernel-tier convergence/coexistence tests already use `IrohHarness` selectively; B-10 extends that pattern.

**Why not (b) — mock blob store**: mocks would prove the wire-shape contract, which `wire_freeze.rs` style tests already cover for canonical-bincode envelopes. The interesting risk is iroh-blobs API churn (the crate is pre-1.0 with a self-declared "not yet production quality" warning on the rewrite line — see [`prior-art/iroh/blobs.md`](../prior-art/iroh/blobs.md) §Versions). Real iroh-blobs through real QUIC catches integration regressions a mock cannot.

**Flake risk**: same as `IrohHarness` carries today (one local UDP socket per peer, `MemoryLookup` for address resolution, drop-order discipline). The pattern shipped in E2E-1 (B-7) and has been stable across iroh_convergence + iroh_coexistence; B-10's test extends the same harness. iroh-blobs adds the BlobsProtocol ALPN registration on the router — one additional ALPN per peer — which is uncontroversial alongside the existing gossip + heads-request ALPN registrations.

### 3.7 Crate boundary: new `crates/distribution/`

**Decision**: introduce `crates/distribution/` as a new workspace member owning iroh-blobs publish + fetch + the revocation/publication topic schema. Feature-gated under `network-iroh` (matches the network crate's iroh gate).

```
crates/
├── backend/              (existing)
├── kernel/               (existing)
├── manifest/             (existing)
├── network/              (existing)
├── distribution/         NEW — iroh-blobs fetch + publish + revocation topic
├── test-utils/           (existing)
├── types/                (existing)
└── wasmtime-backend/     (existing)
```

**Why not extend `crates/network/`**: the network crate owns the `Network` trait, gossip pub/sub, and direct-streams. Adding iroh-blobs (a separate ALPN with a separate request/response shape) would conflate "wire transport" with "content distribution." [`prior-art/iroh/blobs.md`](../prior-art/iroh/blobs.md) frames iroh-blobs as the *data plane* of an iroh deployment, distinct from iroh-gossip; mirroring that boundary in our crate layout matches the layering.

**Why not extend `crates/manifest/`**: `myrhiza-manifest` owns parse / canonicalize / sign / verify. It is a pure-data crate (no networking). Adding iroh-blobs would force every manifest consumer to drag in iroh.

**Why not extend `crates/kernel/`**: kernel hosts the orchestration (runtime, install-flow); the distribution mechanism is a peer-level subsystem the kernel composes. Splitting it out matches the workspace shape committed in [`mvp.md`](2026-05-09-myrhiza-master-design/mvp.md) §15.4 (which lists `crates/storage/`, `crates/crypto/`, `crates/network/` as separate crates).

**Public surface of `crates/distribution/`** (sketch):

```rust
pub struct BundleFetcher { /* iroh-blobs handle, store, etc. */ }
pub struct BundlePublisher { /* same handle, publish topic */ }

pub enum FetchError { /* ... */ }
pub enum PublishError { /* ... */ }

pub struct RevocationEvent { /* see §4.4 */ }
pub struct PublicationEvent { /* see §3.4 */ }
pub struct RevocationLog { /* pure-function apply */ }

pub fn derive_revocation_topic(author_pubkey: AuthorPubkey) -> Topic;
pub fn derive_publication_topic(author_pubkey: AuthorPubkey) -> Topic;
```

The kernel imports `myrhiza-distribution` and wires it into `Runtime::start` behind the existing `network-iroh` feature gate.

## 4. Design

### 4.1 Wire format detail

Each blob in the bundle is one of:

| Blob | Bytes | Hash |
|---|---|---|
| **Manifest** | canonical-bincode-encoded `Manifest` struct | `manifest_blob_hash = iroh-blobs::Hash(BLAKE3(canonical_bincode(manifest)))` |
| **state-apply** | raw `.wasm` component bytes | hash recorded in `Manifest::components::state_apply_hash` (new field, see below) |
| **state-propose** (optional) | raw `.wasm` | `Manifest::components::state_propose_hash` |
| **interaction** (optional) | raw `.wasm` | `Manifest::components::interaction_hash` |
| **behavior** (optional) | raw `.wasm` | `Manifest::components::behavior_hash` |

**Manifest schema delta**: the existing `Manifest::components` carries file paths (`state_apply: Option<PathBuf>`). For the iroh-blobs path, the manifest must carry the *hash* of each component. Two encoding options:

1. **Replace** `state_apply: Option<PathBuf>` with `state_apply_hash: Option<BlobHash>`.
2. **Add** parallel hash fields alongside path fields: keep `state_apply: Option<PathBuf>` for disk-bundle layout + add `state_apply_hash: Option<BlobHash>` for iroh-blobs addressing.

**Pick option 2.** Reasoning: disk bundles + iroh-blobs bundles coexist (per §3.5). Disk-bundle authoring populates path; publish-side `BundlePublisher::publish` computes the hash from the file at the path and fills in the hash field. The manifest then carries both — paths informative for disk-side filenames, hashes load-bearing for iroh-blobs fetch. Signature target absorbs both fields automatically (canonical-bincode includes all fields). Manifest schema becomes:

```rust
pub struct ComponentsSection {
    pub state_apply: Option<PathBuf>,           // existing
    pub state_apply_hash: Option<BlobHash>,     // NEW
    pub state_propose: Option<PathBuf>,         // existing
    pub state_propose_hash: Option<BlobHash>,   // NEW
    pub interaction: Option<PathBuf>,           // existing
    pub interaction_hash: Option<BlobHash>,     // NEW
    pub behavior: Option<PathBuf>,              // existing
    pub behavior_hash: Option<BlobHash>,        // NEW
}
```

This is a manifest schema additive change. Per [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.2 ABI versioning: *"adding a capability or module field is additive (new kernel minor version)"*. B-10 bumps `kernel-minor-min` for bundles publishing through the iroh path; disk-only bundles can still set hashes to `None` for back-compat (the install flow does not require them when the variant is `Disk`).

**Install-side cross-check** (defense-in-depth): when fetching, the kernel verifies that each fetched component's actual BLAKE3 hash matches the `*_hash` field in the manifest (iroh-blobs verifies this against the *request* hash via BLAKE3+Bao; the kernel additionally cross-checks that the request hash matches what the manifest declares). For disk bundles, the existing `bundle_content_hash` recipe already covers tamper detection.

### 4.2 Hash semantics formula

```
manifest_blob_hash = iroh-blobs Hash type wrapping
                     BLAKE3(canonical_bincode(Manifest))

Manifest::components::state_apply_hash =
    BLAKE3(state_apply.wasm raw bytes)

Manifest::components::state_propose_hash =
    BLAKE3(state_propose.wasm raw bytes)         // if declared
... etc for interaction + behavior ...

Manifest::signature =
    Ed25519_sign(author_secret,
                 length_prefix("myrhiza/manifest/v1")
                 | length_prefix(BLAKE3(canonical_bincode(Manifest_without_signature)))
                 | length_prefix(bundle_content_hash(component_bytes...))
                 | length_prefix(app.version)
                 | length_prefix(app.author_pubkey))
```

The signing target is unchanged from [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.2. The `*_hash` fields in `ComponentsSection` participate in the canonical-bincode encoding, so they're authenticated by the signature without any new signing-target framing.

The `BlobHash` type is a thin newtype wrapping `iroh_blobs::Hash` (32-byte BLAKE3) — needed because `iroh_blobs::Hash` is foreign to `myrhiza-manifest`. Conversion functions in `crates/distribution/src/lib.rs`:

```rust
pub fn blob_hash_from_iroh(h: iroh_blobs::Hash) -> BlobHash { ... }
pub fn iroh_hash_from_blob_hash(h: BlobHash) -> iroh_blobs::Hash { ... }
```

(Same orphan-rule pattern as B-4.0's `peer_pubkey_from_iroh`.)

### 4.3 `BundleFetcher` and `BundlePublisher` API

```rust
/// Holds a handle to a local iroh-blobs store + the iroh::Endpoint
/// already constructed by the kernel embedder. Constructed once at
/// kernel boot, shared across all fetch + publish operations.
pub struct BundleDistribution {
    store: iroh_blobs::store::MemStore,             // or FsStore in prod
    blobs_protocol: iroh_blobs::BlobsProtocol,
    endpoint: iroh::Endpoint,
}

impl BundleDistribution {
    /// Wire BlobsProtocol::ALPN against the router. Called once at
    /// kernel boot alongside iroh_gossip::ALPN + HEADS_REQUEST_ALPN.
    pub fn protocol_handler(&self) -> &iroh_blobs::BlobsProtocol { ... }

    /// Publish: import all blobs into local store. Returns the
    /// manifest hash (the `BundleAddress::IrohBlob` identifier).
    pub async fn publish(
        &self,
        manifest: &Manifest,
        manifest_bytes: &[u8],                      // already canonical-bincode-encoded
        state_apply_bytes: &[u8],
        state_propose_bytes: Option<&[u8]>,
        interaction_bytes: Option<&[u8]>,
        behavior_bytes: Option<&[u8]>,
    ) -> Result<BlobHash, PublishError>;

    /// Fetch: pull manifest + all declared components from peers,
    /// materialize into a tempdir, return BundleAddress::Disk ready
    /// for InstallFlow::load.
    pub async fn fetch(
        &self,
        manifest_hash: BlobHash,
        peers: &[PeerPubkey],                       // bootstrap peers known to host the bundle
    ) -> Result<MaterializedBundle, FetchError>;
}

pub struct MaterializedBundle {
    pub _tempdir: tempfile::TempDir,                // RAII; bundle bytes live here
    pub address: BundleAddress,                     // Disk variant pointing into _tempdir
}
```

**Tag lifecycle**: each successful `publish` adds a tag to the local store named `bundle/<manifest_hash>`; the kernel keeps the tag alive for as long as the bundle is installed and drops it on uninstall. [`prior-art/iroh/blobs.md`](../prior-art/iroh/blobs.md) §Tagging-and-GC: "every component that wants persistent data must own its tags and clean them up." Future plan addresses LRU / quota / refcount discipline ([`prior-art/iroh/blobs.md`](../prior-art/iroh/blobs.md) — "Tag-based GC is fine for local cache management but it is *not* a content-availability guarantee").

### 4.4 Revocation topic schema + state machine

```rust
pub struct RevocationEvent {
    /// The bundle hash being revoked. Must match a previously-installed
    /// bundle from this author for any peer to act on it.
    pub revoked_bundle_hash: BlobHash,
    /// Human-readable reason. Truncated at MAX_REASON_LEN = 256 bytes.
    pub reason: String,
    /// Author-asserted timestamp (informational). The kernel does NOT
    /// trust this for ordering — `revocation_seq` is the authority.
    pub revoked_at: u64,
    /// Monotonic-per-author. Kernel rejects out-of-order or duplicate.
    pub revocation_seq: u64,
    /// Signature by the author's pubkey over canonical-bincode of
    /// (revoked_bundle_hash, reason, revoked_at, revocation_seq).
    pub signature: Signature,
}
```

**State machine** (`RevocationLog::apply`):

```
State: per-author { last_observed_seq: u64, revoked_bundles: BTreeSet<BlobHash> }

apply(event):
    verify signature against author pubkey       — reject on fail
    if event.revocation_seq <= last_observed_seq — reject (out of order / duplicate)
    if event.revocation_seq > last_observed_seq + MAX_REVOCATION_JUMP
                                                 — reject (jump cap from §10.7)
    insert event.revoked_bundle_hash into revoked_bundles
    last_observed_seq = event.revocation_seq
    emit RevocationApplied(author, revoked_bundle_hash) to kernel UI surface
```

The kernel UI surface receives the `RevocationApplied` and renders the uninstall prompt per [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.5 step 7 (kernel-controlled UI surface).

**Pure-function property**: `RevocationLog::apply` is a pure function of `(prior log state, signed event)`. No timestamps consulted from the system clock. No network calls. State-tier unit tests cover it directly without any kernel or iroh dependency. This matches the determinism discipline in CLAUDE.md: *"State-apply components must be pure functions of `(prior state, event)` plus the deterministic helper set."* Revocation log is a kernel-resident analog with the same purity contract.

### 4.5 Per-author publishing sequence

```
Author side (publishing v1.4 of "counter"):

    1. Build counter-state-apply.wasm + counter-state-propose.wasm
       + counter-interaction.wasm
    2. Compute hash of each: state_apply_hash = BLAKE3(bytes), etc.
    3. Build Manifest with components.{state_apply, state_apply_hash, ...}
       all populated, plus existing app/abi/capabilities sections.
    4. Canonicalize: m.canonicalize()
    5. Sign: signature = Ed25519_sign(secret, signing_target_bytes(&m, &content_hash))
    6. Attach signature: m.signature = Some(sig)
    7. Canonical-bincode-encode the signed manifest → manifest_bytes
    8. BundleDistribution::publish(m, manifest_bytes, state_apply_bytes, ...)
       → imports all 4 blobs into local iroh-blobs store
       → returns manifest_hash
    9. Build PublicationEvent { manifest_hash, version: "1.4",
                                publication_seq: prior_seq + 1,
                                signature: Ed25519_sign(secret, ...) }
   10. IrohNetwork::publish(publication_topic, PublicationEvent envelope)


Subscribed peer side (peer B, already installed v1.3 of "counter"):

    1. Already subscribed to publication_topic = BLAKE3("myrhiza/publications/v1"
                                                       || author_pubkey)
    2. IrohSubscription::recv() yields PublicationEvent envelope
    3. Verify signature against known author_pubkey
    4. Check publication_seq monotonicity
    5. Surface to user (kernel-controlled UI): "Author X published v1.4 of counter
       (manifest hash <truncated>). Update from v1.3?"
    6. On user approval, BundleDistribution::fetch(manifest_hash, &[author_peer])
       → pulls manifest + components from author's iroh-blobs store
       → materializes into tempdir
    7. InstallFlow::load(BundleAddress::Disk { tempdir, manifest_path })
       → existing install path, no changes
    8. Kernel runs the capability summary + per-update consent flow per §10.5


New peer side (peer C, never installed any bundle from author X):

    1. Receives manifest_hash out-of-band (link / QR / in-app share)
    2. BundleDistribution::fetch(manifest_hash, &[some_peer_known_to_host_it])
       — the bootstrap-peer hint may come from the share format itself
       (a bech32m bundle-share URI; see §3.2)
    3. Steps 7+8 identical to subscribed-peer path
    4. After successful install, kernel auto-subscribes peer C to author X's
       publication_topic + revocation_topic per [distribution.md] §10.7
```

### 4.6 Crate dependency direction

```
crates/distribution/
    depends-on: crates/types, crates/manifest, crates/network
    feature-gate: network-iroh (pulls in iroh + iroh-blobs)

crates/kernel/
    depends-on: crates/distribution (new dep)

crates/test-utils/
    depends-on: crates/distribution (gated under network-iroh) for
    publish-side helpers in kernel-tier acceptance tests
```

No circular deps. `crates/manifest` does NOT depend on `crates/distribution` — the `BlobHash` newtype lives in `crates/types` (it's a pure 32-byte hash, like `EventHash`), so the manifest can carry the hash type without taking on iroh.

### 4.7 Cargo.toml changes

Workspace `[workspace.dependencies]`:

```toml
# iroh-blobs is the data-plane sibling of iroh-gossip. Pinned to
# 0.101.0 (latest as of 2026-05-08); aware that this is the
# "rewrite line" the iroh team flags as "not yet production
# quality" — see prior-art/iroh/blobs.md §"What's actually
# shipping right now". Bump deliberately.
iroh-blobs = "=0.101.0"
```

`crates/distribution/Cargo.toml`:

```toml
[dependencies]
myrhiza-types = { path = "../types" }
myrhiza-manifest = { path = "../manifest" }
myrhiza-network = { path = "../network" }
async-trait.workspace = true
tokio = { workspace = true, features = ["sync", "rt", "macros", "time"] }
serde.workspace = true
thiserror.workspace = true
bincode.workspace = true
tempfile = { workspace = true }
blake3 = { workspace = true }
iroh = { workspace = true, optional = true }
iroh-blobs = { workspace = true, optional = true }
ed25519-dalek = { workspace = true }

[features]
default = []
# Iroh-blobs publish + fetch + revocation/publication topic dispatch.
# Default-off to keep `cargo test --workspace` fast; the iroh-acceptance
# tests in crates/kernel/tests opt in via `network-iroh`.
network-iroh = ["dep:iroh", "dep:iroh-blobs", "myrhiza-network/network-iroh"]
```

`crates/kernel/Cargo.toml`: add `myrhiza-distribution = { path = "../distribution" }` and propagate the `network-iroh` feature.

## 5. Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| **iroh-blobs API churn** (0.101.0 on the rewrite line — the team's own README warns "not yet considered production quality") | High | Spec rewrite in a future minor; behavior regression on bump | Exact-version pin (`=0.101.0`) per [`prior-art/iroh/lessons.md`](../prior-art/iroh/lessons.md) §Avoid row 1; isolate iroh-blobs imports to `crates/distribution/` so a bump touches one crate; track iroh-blobs releases in the iroh prior-art folder's date frontmatter. Anchor spec against *concepts* (content-addressed blob fetch, BLAKE3 verified streaming) and import iroh-blobs current names as terminology. |
| **Fetch flakes under network jitter** | Medium | Test flake in CI; user-visible install failures in production | iroh-blobs BLAKE3 + Bao verified streaming is resumable per [`prior-art/iroh/blobs.md`](../prior-art/iroh/blobs.md) §Wire-protocol ("re-asking for the missing tail of a partially-received blob just costs the missing range"); kernel-tier acceptance test wraps fetch in `tokio::time::timeout(30s)`. Production-side retry policy is a follow-up (no automatic retry in B-10 — user re-tries the install on transient failure). |
| **Revocation log conflict under concurrent revoke** | Low | Last-write-wins ambiguity from compromised key | Already covered by `revocation-seq` monotonicity + MAX_REVOCATION_JUMP from [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.7. A non-compromised author increments seq carefully; a compromised key publishing concurrent (author, seq=N) collides deterministically and one is rejected. This is the single-key-compromise threat already named. |
| **Publication topic enumeration** | Medium | Relay can correlate which peers follow which authors | Same as the revocation-topic enumeration risk already accepted per [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.7 closing paragraph + [`risks.md`](2026-05-09-myrhiza-master-design/risks.md) §19. Mitigation (relay rotation, topic-subscription cover) deferred. |
| **BundleAddress enum migration breaks call sites** | Low | Compile failures across kernel/test-utils on the B-10 PR | Mechanical migration — `grep` returns ~5-10 call sites all in `crates/kernel/tests/`, `crates/test-utils/`, and `crates/myrhiza-cli/`. One PR cleanup. |
| **Manifest schema gains 4 new fields; old bundles without hash fields** | Low | Disk bundles still need to work; iroh bundles need hashes populated | Hash fields are `Option<BlobHash>`. Disk bundles set them to `None` (unchanged behavior). Publish-side fills them in. Install-time check is conditional on the `BundleAddress` variant — `Disk` variant ignores them, `IrohBlob` variant requires them present and rejects on absence. Manifest signature canonical-bincode covers them automatically. |
| **iroh-blobs MemStore vs FsStore choice** | Medium | Production needs persistent store; tests want fast in-memory | Use `MemStore` in tests + dev. `FsStore` for production wired in by the kernel embedder (config-driven). Per `iroh-blobs` 0.101.0, both implement the same `Store` trait. B-10 lands MemStore wired in; FsStore wiring is a small follow-up tied to the storage layer (B-9). |
| **iroh-blobs publish requires the author's iroh::Endpoint to be reachable** | Medium | An author publishing offline has nothing to seed | Acceptable for v1 — same model as iroh-gossip (peers serve what they have, replicas spread organically). Future direction: dedicated seed/relay peers per author. |
| **Bundle hash share format collisions with future `BlobTicket`-shaped URIs** | Low | If we later adopt iroh's `BlobTicket` format, current `BlobHash`-only sharing might need migration | `BlobHash` is the kernel identifier; the share URI is a UX surface. Adopt `BlobTicket` later (it adds bootstrap peer hints embedded in the encoded form, per [`prior-art/iroh/blobs.md`](../prior-art/iroh/blobs.md) §Discovery) — that's additive. |

## 6. Test plan

### 6.1 State-tier (revocation log purity)

`crates/distribution/src/revocation.rs::tests`:

- `applies_first_revocation` — empty log + one signed event → log contains the bundle hash, seq=1.
- `rejects_signature_mismatch` — event signed by wrong key → `RevocationError::SignatureInvalid`.
- `rejects_out_of_order_seq` — seq=3 followed by seq=2 → second rejected.
- `rejects_duplicate_seq` — seq=2 followed by seq=2 → second rejected.
- `rejects_jump_exceeds_max` — seq=N followed by seq=N+MAX_REVOCATION_JUMP+1 → second rejected.
- `accepts_jump_at_max` — seq=N followed by seq=N+MAX_REVOCATION_JUMP → accepted (boundary).
- `idempotent_double_revoke_same_bundle` — two events revoking the same bundle hash under different seqs are both accepted (semantic idempotence: bundle is already in revoked set).

These tests are pure-function tests with no kernel and no iroh. Run on every `cargo test`.

### 6.2 State-tier (publication event verification)

Similar shape under `crates/distribution/src/publication.rs::tests`. Covers signature verification, seq monotonicity, version-string truncation policy.

### 6.3 Kernel-tier (iroh-blobs fetch acceptance test)

`crates/kernel/tests/iroh_bundle_distribution.rs` (new file, feature-gated `#![cfg(feature = "network-iroh")]`):

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn b10_fetch_via_iroh_blobs_closes_mvp_15_1_criterion_1() {
    let harness = IrohHarness::new("b10-test").await;
    let author_peer = harness.spawn_peer_with_blobs(None).await;
    let installer_peer = harness.spawn_peer_with_blobs(None).await;

    // Author publishes the counter bundle.
    let (manifest, manifest_bytes, apply_bytes, propose_bytes, interaction_bytes) =
        build_signed_counter_bundle_three_components_in_memory();
    let manifest_hash = author_peer.distribution
        .publish(&manifest, &manifest_bytes,
                 &apply_bytes, Some(&propose_bytes),
                 Some(&interaction_bytes), None)
        .await
        .expect("publish ok");

    // Installer fetches by manifest hash, naming the author as bootstrap.
    let materialized = installer_peer.distribution
        .fetch(manifest_hash, &[author_peer.network.peer_pubkey()])
        .await
        .expect("fetch ok");

    // The materialized address feeds the existing install flow.
    let loaded = InstallFlow::new()
        .load(&materialized.address)
        .expect("install + verify");

    // Round-trip the kernel-tier criterion: instantiate + apply.
    let backend = WasmtimeBackend::new().expect("backend");
    let handle = backend.instantiate_state_apply(&loaded.component_bytes, ...)
        .expect("instantiate");
    let outcome = handle.apply(&genesis_event(), ...).expect("apply");
    assert_eq!(outcome.new_state_bytes(), 0_i64.to_be_bytes());
}
```

This test exercises real `iroh-blobs::BlobsProtocol` over real iroh QUIC over loopback. The `IrohHarness` already wires the router for gossip + heads-request ALPNs; B-10 extends it to also accept `iroh_blobs::ALPN`.

### 6.4 Kernel-tier (revocation propagation)

`crates/kernel/tests/iroh_revocation.rs`:

- Peer A installs a bundle from author X (auto-subscribes to X's revocation topic).
- Author X (in test, a separate `IrohHarness` peer) publishes a `RevocationEvent` revoking the just-installed bundle.
- Peer A receives the revocation via gossip; `RevocationLog::apply` accepts; kernel surfaces the install for uninstall prompt.
- Assert: peer A's installed-bundle list shows the bundle as `flagged-for-revocation`.

### 6.5 What is NOT tested in B-10

- **Cross-process** iroh-blobs (deferred per E2E-2 shape, same as iroh-gossip cross-process).
- **Real-world relay-traversed iroh-blobs** (loopback only).
- **Storage persistence across restart** (B-9 territory).
- **Mass revocation under flood** beyond the unit-test boundary check.

## 7. Surface change summary

### New public surface

- `crates/distribution/` crate (new workspace member).
- `myrhiza_distribution::BundleDistribution` (the unified handle).
- `myrhiza_distribution::{RevocationEvent, RevocationLog, RevocationError}`.
- `myrhiza_distribution::{PublicationEvent, PublicationLog, PublicationError}`.
- `myrhiza_distribution::{derive_revocation_topic, derive_publication_topic}`.
- `myrhiza_distribution::{blob_hash_from_iroh, iroh_hash_from_blob_hash}` (orphan-rule conversion free fns).
- `myrhiza_types::BlobHash` (new 32-byte newtype; same shape as `EventHash`).

### Modified public surface

- `myrhiza_kernel::BundleAddress` becomes an enum (`Disk` + `IrohBlob`). Existing struct-initializer call sites migrate to `Disk { ... }`.
- `myrhiza_manifest::schema::ComponentsSection` gains four `Option<BlobHash>` fields parallel to the existing `Option<PathBuf>` fields.

### Unchanged public surface

- `InstallFlow::load` signature.
- `WasmtimeBackend::instantiate_state_apply` signature.
- `Runtime` / `RuntimeCfg` public shape.
- All existing kernel tests.

### Cargo feature

- `myrhiza-distribution::network-iroh` (default-off, mirrors the network crate's feature).
- `myrhiza-kernel::network-iroh` (propagates to distribution).

## 8. Cross-references

Within master design:

- [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.1 (bundle shape), §10.2 (manifest schema), §10.3 (iroh-blobs distribution commitment), §10.4 (signing), §10.5 (install flow), §10.7 (revocation topic), §10.8 (no central registry).
- [`mvp.md`](2026-05-09-myrhiza-master-design/mvp.md) §15.1 #1 (the requirement); §15.4 (workspace shape — confirms `crates/distribution/` style as a separate crate).
- [`implementation.md`](2026-05-09-myrhiza-master-design/implementation.md) §20 item 14.
- [`networking.md`](2026-05-09-myrhiza-master-design/networking.md) (iroh-gossip topology that the revocation/publication topics ride on).
- [`risks.md`](2026-05-09-myrhiza-master-design/risks.md) §19 (subscription enumeration risk explicitly out of scope).

Within sibling specs:

- [Plan B-4.0 — Iroh transport skeleton](2026-05-20-plan-b-4-0-iroh-skeleton-design.md) — for the iroh dependency pinning + orphan-rule pattern + caller-owned-endpoint discipline that B-10 mirrors for iroh-blobs.
- [Plan B-4.1 — Real iroh-gossip subscribe + publish](2026-05-20-plan-b-4-1-iroh-gossip-design.md) — the revocation + publication topics use the same `IrohNetwork::publish` + `subscribe` surface this slice landed.
- [Plan B-4.4 — Direct-stream heads request](2026-05-21-plan-b-4-4-direct-streams-design.md) — for the ALPN-router wiring pattern (`router.accept(ALPN, handler)`) that B-10 extends for iroh-blobs.
- [docs/reports/2026-05-21-mvp-gap-analysis.md](../reports/2026-05-21-mvp-gap-analysis.md) — item 14 + open question #2 ("Iroh-blobs for bundle fetch") explicitly name this slice.
- [docs/specs/2026-05-22-e2e-test-coverage-design.md](2026-05-22-e2e-test-coverage-design.md) — the `IrohHarness` pattern this slice extends.

## 9. Prior-art consulted

Per the `using-prior-art` skill, consulted (folder + section):

- **[`prior-art/iroh/blobs.md`](../prior-art/iroh/blobs.md)** — the load-bearing single-doc reference. Validates the entire wire shape: BLAKE3 verified streaming for tamper-evident pull, `HashSeq` shape for collections (we chose typed manifest over `HashSeq` — see §3.1), tag-based GC (informs §4.3 tag lifecycle), discovery gap ("iroh-blobs has no DHT, no gossip, no built-in announce" — directly motivates the publication topic in §3.4), 0.35-vs-rewrite version split + "not yet production quality" warning on main (informs §5 risk register).
- **[`prior-art/iroh/lessons.md`](../prior-art/iroh/lessons.md)** §Validates row "Content-addressing as the bundle-distribution primitive" + §Borrow row 4 "BLAKE3 + Bao verified streaming for app-bundle distribution" — confirms the entire approach. §Avoid row 1 "Every minor is breaking" — drives the exact-version pin `=0.101.0`. §Avoid row "Relay metadata is privileged" — informs §5 publication topic enumeration risk.
- **[`prior-art/iroh/identity.md`](../prior-art/iroh/identity.md)** — for NodeID = Ed25519 32-byte pubkey + pkarr-on-Mainline-DHT discovery model (informs §4.5 new-peer side).
- **[`prior-art/app-distribution/README.md`](../prior-art/app-distribution/README.md)** — framing of OCI / wkg / Sigstore as the WASM ecosystem mainstream answer.
- **[`prior-art/app-distribution/bundle-comparisons.md`](../prior-art/app-distribution/bundle-comparisons.md)** — the comparison matrix between CM canonical, Spin, wasmCloud, Holochain, Endo, Pears, ESM+import-maps. "Implications for Myrhiza" identifies "hash-canonical" as the P2P-native answer Myrhiza commits to (§3.2).
- **[`prior-art/app-distribution/lessons.md`](../prior-art/app-distribution/lessons.md)** §Avoid row "Custom bundle format instead of OCI" — invoked in §3.1 against the combined-tarball runner-up. §Avoid row "Tying Myrhiza to a single registry vendor" — invoked in §3.3 against the global-registry runner-up. §Validates row "Content addressing at the artifact layer is the right primitive" — confirms §3.2.
- **[`prior-art/app-distribution/signing.md`](../prior-art/app-distribution/signing.md)** — Sigstore Cosign keyless + Notary X.509 design split. The Sigstore-as-primitive (signed append-only log) inspired the revocation-log shape; the Sigstore-as-deployed (Rekor + Fulcio third parties) is the OIDC dependency we reject. Runner-up paradigm called out in §3.3.
- **[`prior-art/app-distribution/open-problems.md`](../prior-art/app-distribution/open-problems.md)** §1 "P2P distribution without a registry" + §2 "Signature verification under peer-author identity (not OIDC)" + §3 "Component-bundle revocation" — these are the three open problems B-10 closes for v1. §3 explicitly cites our exact approach: "In-band negative events (an event in the app's own log says 'revoked')" — modulo our choice of per-author topic instead of per-app log.
- **[`prior-art/holochain/distribution.md`](../prior-art/holochain/distribution.md)** §"Bundle binary format" + §"Bundle signing & source verification" — the closest peer-symmetric WASM-runtime precedent. The Holochain `.happ` shape (gzip+MessagePack tarball) is the runner-up paradigm explicitly rejected in §3.1 for being non-streamable + non-deduplicating. The "no native bundle signing" gap ("a real gap") in Holochain validates our author-pubkey-signed manifest from day one.
- **[`prior-art/wasmcloud/README.md`](../prior-art/wasmcloud/README.md)** §"Where wasmCloud sits" — wasmCloud's per-component OCI artifact pattern is what we'd land if we picked OCI. We don't; §3.1 names this as runner-up (c). wasmCloud v1's pre-orchestrator era ("capability providers as bundles" pattern) informs the per-bundle vs per-author distinction (we picked per-author for revocation/publication, §3.3 + §3.4).
- **[`prior-art/spin/README.md`](../prior-art/spin/README.md)** §"Why this folder exists" point 3 ("OCI distribution + componentize-* build paths — Spin pioneered the WASM-component-as-OCI-artifact distribution model") — same as wasmCloud, an explicit OCI-canonical alternative we benchmark against.

### Runner-up paradigms named

Per CLAUDE.md "Surface tradeoffs explicit. Name the runner-up paradigm if a choice was made":

| Choice | Picked | Runner-up | Cited in |
|---|---|---|---|
| Wire format | two iroh-blobs (manifest + per-component) | combined tarball ([`prior-art/holochain/distribution.md`](../prior-art/holochain/distribution.md)); OCI artifact ([`prior-art/wasmcloud/`](../prior-art/wasmcloud/), [`prior-art/spin/`](../prior-art/spin/)) | §3.1 |
| Hash semantics | iroh-blobs BLAKE3 of canonical-bincode manifest | hash of combined blob (no combined blob); bech32m-encoded share string (UX layer, not kernel) | §3.2 |
| Revocation distribution | per-author append-only via gossip | per-bundle topic (doesn't reach uninstalled peers); global registry ([`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.8); Sigstore/Rekor third-party log ([`prior-art/app-distribution/signing.md`](../prior-art/app-distribution/signing.md)) | §3.3 |
| Per-author publishing | signed manifest + import into local store + gossip announcement on per-author publication topic | OCI registry push ([`prior-art/spin/`](../prior-art/spin/)); out-of-band only (loses subscribed-peer notification) | §3.4 |
| Backwards compat | `BundleAddress` enum (`Disk` + `IrohBlob`) | retire disk; feature-flag both paths | §3.5 |
| Test strategy | mix (disk for unit, real iroh-blobs for kernel-tier) | mock store; everywhere real iroh-blobs | §3.6 |
| Crate boundary | new `crates/distribution/` | extend `crates/network/`; extend `crates/manifest/` | §3.7 |

### Remaining gaps in the corpus

- **iroh-blobs 0.101.0 exact API surface** — the prior-art folder is dated 2026-05-08 and the version it documents (`iroh-blobs = "0.101"` per [`prior-art/iroh/blobs.md`](../prior-art/iroh/blobs.md)) matches the pin. The implementer should still verify exact method signatures (`BlobsProtocol::new`, `MemStore::add_bytes`, the downloader API, tag lifecycle methods) at impl time — pre-1.0 churn risk per [`prior-art/iroh/lessons.md`](../prior-art/iroh/lessons.md) §Avoid row 1.
- **Endo bundle-hash for the cross-runtime comparison** — [`prior-art/agoric-endo/`](../prior-art/agoric-endo/) is referenced by app-distribution/bundle-comparisons.md (the "cleanest existence-proof of 'ship a code artifact by its hash, not by a name in a registry'") but is not load-bearing for this slice's decisions; Endo's chain-stored bundles assume a consensus layer Myrhiza explicitly does not have.
- **Pears / Hypercore for the P2P-versioned-app comparison** — [`prior-art/pears/`](../prior-art/pears/) "the closest existing-art for Myrhiza's 'ship apps over a P2P transport' story." Not consulted for this slice's wire-format decisions because Hypercore is fundamentally a different shape (append-only signed log, not content-addressed blob store). Future direction: a Hypercore-shape version log layered on top of iroh-blobs hashes could replace the publication topic — that's a v1.5+ design space.

## 10. Out-of-scope future work — explicit deferrals

- **OCI artifact path as a side-channel** for cross-WASM-ecosystem interop. A future plan may add `BundleAddress::Oci { reference: String }` alongside `Disk` + `IrohBlob`, using `wkg` to pull from a registry then materializing into a tempdir same as the iroh path. Out of v1 (no central registry commitment).
- **Sigstore / Cosign signature attachments** as an *optional* second signature layer beyond the author-pubkey-signed manifest. Provides supply-chain provenance for authors who want OIDC-keyless attestation alongside their own signature. Compatible with the manifest layout; would attach as a separate blob referenced from the manifest.
- **Bundle-share URI format** — bech32m-encoded `(manifest_hash, [bootstrap_peers])` for sharing in chat / QR / in-app. Modeled on iroh's `BlobTicket`. Out of B-10 — the kernel uses raw `BlobHash` + optional `Vec<PeerPubkey>` directly; UX surface is a follow-up.
- **Tag GC + LRU / quota** — iroh-blobs tags hold blobs alive indefinitely. Future storage spec (B-9 territory) defines retention policy.
- **`FsStore` for production persistence** — B-10 uses `MemStore` in tests and dev; production storage of blob bytes across restart is B-9-adjacent and lands behind the same `network-iroh` feature gate.
- **Lazy / streaming component fetch** — fetch only the components needed for the current profile (e.g. skip behavior.wasm if the peer is not the designated behavior host). BLAKE3 + Bao supports it natively via range requests.
- **Bundle-availability gossip** — a "I have bundle X" beacon so new peers can find seeders without out-of-band peer hints. Currently the caller-provided `&[PeerPubkey]` bootstrap list handles this for known peers; new-peer discovery is the open `prior-art/app-distribution/open-problems.md §1` we partially address but don't fully close.
- **Module-dep recursive fetch** — manifest's `[[modules.dep]]` array carries content-hash-pinned module deps per [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.2. The install flow's recursive resolution (step 4 of §10.5) needs to fetch each module bundle by hash. B-10 ships the primitive (fetch by hash) but not the recursive resolver — the kernel's `InstallFlow` does not yet do recursive module-dep resolution (that's marked "plan B" in `crates/kernel/src/install.rs`). Follow-up slice.
- **Revocation surfaces to UI** — the kernel surfaces `RevocationApplied` to a UI sink; rendering the uninstall prompt + handling pin/uninstall is the kernel-controlled UI work that mvp.md §15.1 #5 hangs off (already met for v1 via `crates/myrhiza-cli/`). B-10 wires the surfacing; the CLI / UI flow is a follow-up polish slice.

## 11. Estimate

**5-7 days** for one focused engineer, matching the gap-analysis estimate. Breakdown:

- Day 1: `crates/distribution/` scaffold + `BlobHash` newtype + manifest schema delta + `BundleAddress` enum migration (mechanical, touches many call sites).
- Day 2: `BundleDistribution::publish` (in-memory store, single-author) + state-tier publication tests.
- Day 3: `BundleDistribution::fetch` + materialization tempdir + integration with `InstallFlow::load`.
- Day 4: `RevocationLog` + `PublicationLog` + state-tier revocation tests.
- Day 5: `IrohHarness` extension for iroh-blobs ALPN + kernel-tier acceptance test (the load-bearing §6.3 test).
- Day 6: kernel-tier revocation propagation test + edge-case shake-out (drop-order, fetch timeout, concurrent revoke).
- Day 7: docs polish + lint zero-warning pass + integration with `crates/myrhiza-cli/` for publish smoke-test.

Risk-adjusted upper bound: 7 days. Likely-case middle: 5-6 days.

## 12. Open questions for the plan writer

These are decisions intentionally deferred to the plan that the spec does not constrain:

1. **`MemStore` vs `FsStore` in tests** — both implement the same `Store` trait per iroh-blobs 0.101.0. The kernel-tier acceptance test could go either way. `MemStore` is faster (no tempdir for blob bytes); `FsStore` is closer to production. Plan picks one and documents.
2. **`BlobsProtocol` events: send None or Some(EventSender)** — `BlobsProtocol::new(&store, None)` per the API. The `events` parameter exists for progress notifications; B-10 may want it `None` for tests and `Some` in production for install-progress UI. Plan-writer chooses.
3. **`tokio::time::timeout` duration for fetch in tests** — 30s suggested in §5; the plan-writer may tune based on iroh-blobs benchmark numbers observed during implementation.
4. **Exact bench numbers for fetch latency over loopback** — informational; the gap analysis didn't bench iroh-blobs and we shouldn't speculate. Plan captures observed numbers in PR body.
5. **Whether to add `BundleShareUri` bech32m format in B-10 or defer** — leaning defer (out of scope per §10) but the plan-writer may add it if it falls out cleanly while wiring `BundleAddress::IrohBlob`.

## Sources

- iroh-blobs 0.101.0 crate: <https://docs.rs/iroh-blobs/0.101.0/iroh_blobs/>
- iroh-blobs protocol docs: <https://docs.iroh.computer/protocols/blobs>
- iroh-blobs repository: <https://github.com/n0-computer/iroh-blobs>
- BLAKE3 specification: <https://github.com/BLAKE3-team/BLAKE3-specs/blob/master/blake3.pdf>
- Bao verified streaming: <https://github.com/oconnor663/bao>
- [`prior-art/iroh/blobs.md`](../prior-art/iroh/blobs.md) (the primary load-bearing reference)
- [`prior-art/iroh/lessons.md`](../prior-art/iroh/lessons.md) §Validates row "Content-addressing" + §Borrow row 4 + §Avoid row 1
- [`prior-art/app-distribution/bundle-comparisons.md`](../prior-art/app-distribution/bundle-comparisons.md), [`signing.md`](../prior-art/app-distribution/signing.md), [`lessons.md`](../prior-art/app-distribution/lessons.md), [`open-problems.md`](../prior-art/app-distribution/open-problems.md)
- [`prior-art/holochain/distribution.md`](../prior-art/holochain/distribution.md) §"Bundle binary format" + §"Bundle signing & source verification"
- [`prior-art/wasmcloud/README.md`](../prior-art/wasmcloud/README.md) §"Where wasmCloud sits"
- [`prior-art/spin/README.md`](../prior-art/spin/README.md)
- [`distribution.md`](2026-05-09-myrhiza-master-design/distribution.md) §10.1–§10.10 (entire chapter)
- [`mvp.md`](2026-05-09-myrhiza-master-design/mvp.md) §15.1 #1 + §15.4
- [`implementation.md`](2026-05-09-myrhiza-master-design/implementation.md) §20 item 14
- [docs/reports/2026-05-21-mvp-gap-analysis.md](../reports/2026-05-21-mvp-gap-analysis.md) item 14 + open question #2
- [Plan B-4.0 — Iroh transport skeleton](2026-05-20-plan-b-4-0-iroh-skeleton-design.md)
- [Plan B-4.1 — Iroh-gossip subscribe + publish](2026-05-20-plan-b-4-1-iroh-gossip-design.md)
