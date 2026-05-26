**Date:** 2026-05-22
**Status:** active
**Subject:** The decision file — what app-distribution prior art validates, what Myrhiza should avoid, what to borrow when designing component install / signing / bundle-format specs.

# Lessons for Myrhiza — app distribution

Synthesis across [`oci-artifacts.md`](oci-artifacts.md), [`wkg.md`](wkg.md), [`wac.md`](wac.md), [`signing.md`](signing.md), [`registries.md`](registries.md), [`bundle-comparisons.md`](bundle-comparisons.md), [`supply-chain.md`](supply-chain.md), [`browser-distribution.md`](browser-distribution.md). Format: validates / avoid / borrow.

## Validates

1. **OCI as the distribution substrate, not a container substrate.** OCI image-spec 1.1 (2024-02-15) made `artifactType` + `subject` + the Referrers API first-class, finally separating "thing in a registry" from "OCI container image." This is the substrate Spin, wasmCloud, and the BA toolchain have all converged on for Wasm components. Myrhiza inheriting OCI for component distribution is mainstream, not exotic. *Source: [`oci-artifacts.md`](oci-artifacts.md).*

2. **Content addressing at the artifact layer is the right primitive.** OCI digests are `sha256:...` of the manifest; manifest references layer blobs by digest. The Endo bundle-hash story ([`prior-art/agoric-endo/distribution.md`](../agoric-endo/distribution.md)), the Holochain DNA hash, and Spin's component digests all reduce to the same shape. Myrhiza's "component-by-hash" addressing is the standard way to do this. *Source: [`bundle-comparisons.md`](bundle-comparisons.md).*

3. **Two-tier composition (component + manifest) is universal.** Spin (`spin.toml`), wasmCloud (`wadm.yaml`), Holochain (`happ` bundle), Endo (`compartment-mapper`), and Wasm core (`wac` for build-time composition) all express "this app = these components + this wiring." A Myrhiza app manifest sitting next to its component blobs is well-precedented. *Source: [`bundle-comparisons.md`](bundle-comparisons.md), [`wac.md`](wac.md).*

4. **Sigstore as the default signing primitive.** Cosign GA 2022-10; OpenSSF graduating; broad adoption across container ecosystem and increasingly Wasm. Keyless / OIDC-bound signing fits a P2P kernel that delegates trust to identity providers (or, for Myrhiza, to peer-author keypairs). *Source: [`signing.md`](signing.md).*

5. **Wasm component as the bundle unit.** Spin uses single-component bundles; wasmCloud uses per-component artifacts plus a manifest; the BA `wac` toolchain composes components at build time into a single `.wasm`. Myrhiza's "app = component(s)" framing is the path of least friction with the rest of the ecosystem. *Source: [`wac.md`](wac.md), [`bundle-comparisons.md`](bundle-comparisons.md).*

## Avoid

| Pitfall | Source | Mitigation |
|---|---|---|
| **Custom bundle format instead of OCI.** Holochain's `.happ` (gzip+MessagePack) is custom; no native signing; no registry ecosystem. Spin and wasmCloud both moved off custom formats onto OCI. | [`bundle-comparisons.md`](bundle-comparisons.md), [`registries.md`](registries.md) | Use OCI artifacts. The interop benefit (any OCI registry + ORAS + Cosign) is worth more than the customization options of a one-off format. |
| **`opencontainers/artifacts` repo as a citation.** Archived 2023-07-18; superseded by image-spec 1.1 + distribution-spec 1.1. Linking it dates the spec. | [`oci-artifacts.md`](oci-artifacts.md) | Cite image-spec 1.1+ directly for the `artifactType` story. |
| **`warg` (Wasm Registry protocol) as a current dependency.** Archived 2025-07-28; superseded by `wasm-pkg-tools`. | [`wkg.md`](wkg.md) | Use `wkg`. `warg` should appear only in history sections. |
| **JWT-as-component-signature.** JWT is signed *metadata about a thing*; cap-token semantics. Bundle signing wants *signature over the artifact bytes*, the Sigstore/Notation/OpenPGP shape. | [`signing.md`](signing.md), cross-ref [`capability-tokens/`](../capability-tokens/) | Sigstore (Cosign) or Notary v2 (Notation). |
| **Tying Myrhiza to a single registry vendor.** OCI Distribution Spec is the substrate; ghcr.io, Docker Hub, ECR, GAR, JFrog all implement it. Picking one provider as the "Myrhiza app store" cedes governance. | [`registries.md`](registries.md) | Treat registries as fungible; let users / operators choose. The kernel pulls by digest-and-URL; the URL is configurable. |
| **`wac` at build-time only.** `wac` composes components statically. A runtime that wants dynamic composition / hot-reload needs a runtime composition mechanism too. | [`wac.md`](wac.md) | Treat `wac` as the canonical build-time composition tool. For runtime composition, design the kernel's import-binding mechanism separately. |
| **HTML import maps as the Myrhiza app-distribution model.** Useful for JS-ecosystem comparison but irrelevant to Wasm-component distribution. | [`browser-distribution.md`](browser-distribution.md) | Cite only as JS-ecosystem comparison; do not derive bundle-format choices from it. |
| **Pre-1.0 version pins.** `wkg` 0.15.0, `wac` 0.10.0 — both pre-1.0 with infrequent releases. Pinning a version in a Myrhiza spec is fine; treating it as stable API is not. | [`wkg.md`](wkg.md), [`wac.md`](wac.md) | Pin loose-ranges in specs; track upstream release cadence. Expect breaking changes through 1.0. |

## Borrow

1. **OCI artifact distribution for component bundles.** Use `wkg push` / `wkg pull` semantics. Map `artifactType` to a Myrhiza-specific media type (e.g., `application/vnd.myrhiza.component.v1+wasm`). *See [`oci-artifacts.md`](oci-artifacts.md), [`wkg.md`](wkg.md).*

2. **Sigstore Cosign for bundle signature attachments.** Cosign's "attach signature to OCI artifact by digest" model fits a peer-keypair-signed bundle directly. The signature is itself an OCI manifest with `subject` pointing at the component digest. *See [`signing.md`](signing.md).*

3. **`wac` for build-time component composition.** Apps that ship multiple components glued into one bundle use `wac`. Myrhiza's app-shape (component-per-profile: state-apply / state-propose / interaction / behavior) naturally maps to `wac` composition. *See [`wac.md`](wac.md).*

4. **SLSA build provenance attestations.** OCI artifact + Sigstore + SLSA attestation = supply-chain-attested bundle. This is the GA path for "where did this component come from." *See [`supply-chain.md`](supply-chain.md).*

5. **Spin's manifest-static capability declaration.** `spin.toml` enumerates the capabilities each component requests; Myrhiza's WIT imports give the same property declaratively. The Spin model (per-component capability scope) is borrowable directly for the Myrhiza manifest. *See [`bundle-comparisons.md`](bundle-comparisons.md), cross-ref [`prior-art/spin/`](../spin/).*

6. **Holochain's content-addressed app-hash as identity.** A Myrhiza app's identity *is* its component-bundle hash. Two peers running "the same app" agree on the same hash; differing hashes = different apps. Holochain's hApp hash is the canonical precedent. *See [`bundle-comparisons.md`](bundle-comparisons.md), cross-ref [`prior-art/holochain/`](../holochain/).*

## Cross-references

- [`README.md`](README.md) — folder overview + reading order
- [`oci-artifacts.md`](oci-artifacts.md), [`wkg.md`](wkg.md), [`wac.md`](wac.md), [`signing.md`](signing.md), [`registries.md`](registries.md), [`bundle-comparisons.md`](bundle-comparisons.md), [`supply-chain.md`](supply-chain.md), [`browser-distribution.md`](browser-distribution.md) — evidence files
- [`prior-art/spin/`](../spin/) — `spin.toml` manifest model
- [`prior-art/wasmcloud/`](../wasmcloud/) — OCI distribution + `wadm`
- [`prior-art/holochain/`](../holochain/) — hApp bundle + DNA hash
- [`prior-art/agoric-endo/`](../agoric-endo/) — bundle-hash story
- [`prior-art/wasm-component-model/`](../wasm-component-model/) — `wac` parent ecosystem

## Sources

All sources in evidence files. This file is synthesis.
