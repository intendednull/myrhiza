**Date:** 2026-05-22
**Status:** active
**Subject:** What this corpus does not solve — gaps in the WASM-app-distribution story that Myrhiza will hit when writing concrete install / signing / install-UX specs.

# Open problems — app distribution

What the OCI + `wkg` + Sigstore stack does *not* answer for a peer-to-peer runtime. Each entry: short problem statement + why it matters for Myrhiza + canonical sources to consult when writing the answer.

## 1. P2P distribution without a registry

OCI assumes an HTTPS registry. `wkg pull` resolves a tag against a known registry URL. A peer-to-peer runtime has no registry — peers gossip components directly. The OCI artifact format is fine; the *distribution mechanism* is not.

**What's needed:** a peer-to-peer fetch path that, given a content-addressed artifact digest, retrieves the artifact from other peers. iroh-blobs is the candidate transport. The question is how to map OCI's `manifest → layer-digest` graph onto iroh-blobs' content-addressed blob storage.

**Canonical sources:** [`prior-art/iroh/`](../iroh/) (iroh-blobs), [`prior-art/pears/`](../pears/) (Hyperdrive blob distribution), [`prior-art/holochain/`](../holochain/) (DHT distribution).

## 2. Signature verification under peer-author identity (not OIDC)

Cosign's keyless / OIDC-bound signing assumes a CA-rooted identity authority. Myrhiza peers sign with their own keypairs (Ed25519). The trust chain is "the peer who signed this bundle is the peer the user trusts" — not "this signature was issued by GitHub OIDC."

**What's needed:** a Cosign-shaped signature mechanism where the signing identity is a peer pubkey (or an MLS group), not an OIDC token. The wire format can mirror Cosign's OCI attachment shape; the trust resolution is different.

**Canonical sources:** [`signing.md`](signing.md), [`prior-art/signal/identity.md`](../signal/identity.md), [`prior-art/mls/`](../mls/), [`prior-art/capability-tokens/`](../capability-tokens/).

## 3. Component-bundle revocation

OCI artifacts don't natively support revocation. Cosign supports timestamping (the signature is valid at a point in time) and Rekor (transparency log) but neither answers "this signed bundle is now revoked, do not run it." Sigstore's transparency-log model is closer to "verifiable history" than "active revocation."

**What's needed:** Myrhiza's revocation story. Out-of-band kernel policy (peer refuses to run app X)? In-band negative events (an event in the app's own log says "revoked")? Capability-rotation (the cap-token granting the right to run an app expires)?

**Canonical sources:** [`signing.md`](signing.md), [`prior-art/capability-tokens/`](../capability-tokens/), [`prior-art/mls/open-problems.md`](../mls/open-problems.md) (PCS analog).

## 4. Snapshot portability across component upgrades

This is Myrhiza's snapshot-portability open problem ([`prior-art/willow/open-problems.md`](../willow/open-problems.md)) seen from the distribution side. When `app-v1.0.wasm` ships an updated `app-v1.1.wasm`, the existing snapshot of v1.0 state may not be valid for v1.1.

**What's needed:** version metadata on bundles + a snapshot-migration contract. Either (a) the new component exports a migrator function from v(N) → v(N+1), or (b) the new component refuses to load a v(N) snapshot, forcing replay from genesis.

**Canonical sources:** [`prior-art/schema-evolution/`](../schema-evolution/), [`prior-art/willow/open-problems.md`](../willow/open-problems.md) §"Snapshot portability".

## 5. Install UX without a central app store

OCI registries are the analog of an app store. Without one, Myrhiza needs a different "install this app" flow. Options: peer-to-peer share-the-link; QR codes; out-of-band capability tokens. None of these are solved in the corpus.

**What's needed:** spec for "user receives an invitation to run app X with cap-token Y" — the install-flow data model. Includes both first-install and subsequent-update flows.

**Canonical sources:** [`prior-art/at-protocol/`](../at-protocol/) (no parallel — atproto apps install in-browser), [`prior-art/holochain/`](../holochain/) (manual `.happ` installation), [`prior-art/pears/`](../pears/) (Keet's invite flow as closest precedent).

## 6. Cross-runtime portability

A Myrhiza component is a `.wasm` against a Myrhiza WIT world. Could the same component run on Spin? On wasmCloud? In a browser via jco? Yes in principle (Component Model is a substrate) but the WIT worlds differ. "Build once, run anywhere" is aspirational.

**What's needed:** decide whether Myrhiza WIT worlds extend an existing standard (WASI 0.2.x?) or stay Myrhiza-specific. Latter is simpler; former preserves portability options.

**Canonical sources:** [`prior-art/wasm-component-model/`](../wasm-component-model/), [`prior-art/spin/`](../spin/), [`prior-art/wasmcloud/`](../wasmcloud/), [`prior-art/jco/`](../jco/).

## 7. Bundle size + lazy loading

A `wac`-composed app can be tens of MB. OCI registries handle this fine (Docker handles GB images). A *peer* serving a bundle to another peer over residential bandwidth does not. Lazy / streaming load of components is unsolved at the distribution layer.

**What's needed:** either (a) split apps into "core" + "lazy" components and load on demand, or (b) component-internal lazy loading (CM `lift` lazy semantics). Both have spec implications.

**Canonical sources:** [`bundle-comparisons.md`](bundle-comparisons.md), [`prior-art/wasm-component-model/`](../wasm-component-model/) (lazy-loading discussions).

## 8. Multi-architecture component artifacts

OCI image-spec supports multi-arch via image-index manifests (Docker buildx publishes one image-index pointing to per-arch manifests). Wasm is in principle arch-agnostic — but `wasm32-wasi` vs `wasm64-wasi` vs preview2 vs preview3 are *effectively* different "architectures."

**What's needed:** decide whether Myrhiza components are pinned to a single WIT/WASI version (simpler) or use OCI image-index to ship multi-target (more complex). Likely single-target through v1; multi-target later.

**Canonical sources:** [`oci-artifacts.md`](oci-artifacts.md), [`prior-art/wasm-component-model/open-problems.md`](../wasm-component-model/open-problems.md).

## Cross-references

- [`README.md`](README.md), [`lessons.md`](lessons.md)
- Per-system evidence files (see [`README.md`](README.md))
- [`prior-art/iroh/`](../iroh/), [`prior-art/holochain/`](../holochain/), [`prior-art/spin/`](../spin/), [`prior-art/wasmcloud/`](../wasmcloud/), [`prior-art/jco/`](../jco/), [`prior-art/wasm-component-model/`](../wasm-component-model/), [`prior-art/schema-evolution/`](../schema-evolution/), [`prior-art/capability-tokens/`](../capability-tokens/), [`prior-art/mls/`](../mls/)

## Sources

All sources in evidence files.
