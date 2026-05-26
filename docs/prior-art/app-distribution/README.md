**Date:** 2026-05-22
**Status:** active
**Subject:** WASM / component app-distribution lineage — OCI artifacts, `wkg`, `wac`, Sigstore-shaped signing, and the bundle-format cousins (Spin, wasmCloud, Holochain, Endo) Myrhiza has to compose with.

# App distribution prior art

How do we get a built component from a developer's laptop to a peer running Myrhiza? This folder consolidates the answer the WASM ecosystem has converged on (OCI artifacts + `wkg` + signature overlays) and benchmarks it against the bundle formats Myrhiza-adjacent runtimes have already chosen (Spin, wasmCloud, Holochain, Endo). It's the "ship an app" half of the runtime story — the other half is "load an app", which lives in [`wasm-component-model/`](../wasm-component-model/) and [`jco/`](../jco/).

This folder is a **multi-subject survey**, not a single-project deep-dive — closer in shape to [`crdts/`](../crdts/) (Automerge + Yjs + Loro packed into one folder) than to [`holochain/`](../holochain/) or [`iroh/`](../iroh/).

## How to use

Read [`README.md`](./README.md) (this file) → [`oci-artifacts.md`](./oci-artifacts.md) → [`wkg.md`](./wkg.md) → [`bundle-comparisons.md`](./bundle-comparisons.md) → [`signing.md`](./signing.md) → [`lessons.md`](./lessons.md). Consult [`open-problems.md`](./open-problems.md) when a spec runs into something the corpus doesn't solve.

**Framing disclosure.** These docs are written from a Component-Model-as-foundation + P2P-first stance — most "Implications for Myrhiza" sub-sections frame each system's choices through that lens. The corpus assumes Myrhiza will adopt OCI as the canonical transport and CM `.wasm` (or wac-composed `.wasm`) as the canonical bundle shape, and reads the alternatives (Holochain `mr_bundle`, Endo bundle hash, raw URL-fetch) through that commitment. Future readers auditing whether Component-Model-on-OCI is itself the right primitive should weigh the corpus accordingly: it's a learn-from-the-ecosystem-into-CM-on-OCI artifact, not a neutral catalog.

## Key facts (verified 2026-05-22)

| Component | Version | Date | License | Status |
|---|---|---|---|---|
| OCI Image-spec | v1.1.1 | 2025-03-03 | Apache-2.0 | stable (artifactType + subject + Referrers API since 1.1.0 2024-02-15) |
| OCI Distribution-spec | v1.1.1 | 2025-03-03 | Apache-2.0 | stable (Referrers API since 1.1.0 2024-02-15) |
| opencontainers/artifacts repo | n/a | archived 2023-07-18 | — | folded into image-spec + distribution-spec 1.1 |
| ORAS CLI | v1.3.2 | 2025-04-18 | Apache-2.0 | stable; OCI distribution-spec 1.1.1 compliant |
| `wkg` (Bytecode Alliance wasm-pkg-tools) | v0.15.0 | 2025-02-06 | Apache-2.0 WITH LLVM-exception | active; small-team BA-stewarded |
| `wac` (wac-cli) | v0.10.0 | 2024-04-17 | Apache-2.0 | active; Bytecode Alliance |
| `warg` (Wasm Registry protocol) | n/a | archived 2025-07-28 | Apache-2.0 | superseded by wasm-pkg-tools |
| Sigstore Cosign | v3.0.6 (latest), v2.6.3 (LTS) | 2026-04-06 | Apache-2.0 | GA since 2022-10-26 |
| Sigstore (umbrella) | — | — | Apache-2.0 | OpenSSF Incubating |
| Notation CLI (Notary Project) | v1.3.2 stable, v2.0.0-alpha.1 | 2025-04-27 / 2025-03-13 | Apache-2.0 | CNCF Incubating |
| HTML import maps | (HTML Standard) | Baseline since 2023-03 | — | Chrome 89, Firefox 108, Safari 16.4 |
| Spin OCI distribution | `spin registry push/pull` | since Spin 1.x | Apache-2.0 WITH LLVM-exception | wkg integration since Spin 2.6 |
| wasmCloud OCI | `wash oci push/pull` | since wasmCloud 0.x | Apache-2.0 | core distribution mechanism |
| Holochain `.happ` bundle | gzip+MessagePack | through 0.6 | CAL-1.0 | no native signing |

**Verify-before-lifting rules:** wkg and wac are pre-1.0 with infrequent releases — version pins decay quickly; reverify before specifying. ORAS, Cosign, Notation move faster; the lifecycle stage matters more than the exact version. The opencontainers/artifacts repo is dead — never link it as authoritative; cite image-spec 1.1+ instead.

## Files

| File | Scope |
|---|---|
| [`README.md`](./README.md) | this file — overview, key facts, ToC, framing disclosure |
| [`oci-artifacts.md`](./oci-artifacts.md) | OCI image-spec 1.1 + distribution-spec 1.1 + ORAS + the artifact / image / referrers data model |
| [`wkg.md`](./wkg.md) | Bytecode Alliance `wkg` (wasm-pkg-tools) — design, commands, package-resolution model, bus-factor honest assessment |
| [`wac.md`](./wac.md) | WebAssembly Compositions ("whack") — composition language, build-time wiring, comparison to runtime composition |
| [`signing.md`](./signing.md) | Sigstore (Cosign + Fulcio + Rekor + Gitsign), Notary Project Notation v1/v2, threat models, OCI 1.1 referrers as signature carrier |
| [`bundle-comparisons.md`](./bundle-comparisons.md) | Spin `spin.toml` + wac, wasmCloud component+provider OCI shape, Holochain hApp bundle, Endo bundle hash, ES modules + import maps. What "bundle" means in each |
| [`registries.md`](./registries.md) | OCI registry landscape (GHCR / Docker Hub / ttl.sh / self-hosted Distribution-spec / OCI ecosystem) + the dead `warg` protocol + CNCF TAG Runtime WASM OCI layout |
| [`browser-distribution.md`](./browser-distribution.md) | ES modules + HTML import maps + the jco transpile path — the browser-side counterpart to OCI |
| [`supply-chain.md`](./supply-chain.md) | SBOM (SPDX/CycloneDX), attestations (in-toto, SLSA), provenance, the Sigstore-attached-artifact model, Notation policy enforcement |
| [`history.md`](./history.md) | timeline: docker registry (2014) → OCI image-spec 1.0 (2017) → OCI artifacts proposal (2019) → Sigstore launch (2021) → image-spec 1.1 (2024-02-15) → wkg replaces warg (2025-07) |
| [`open-problems.md`](./open-problems.md) | what the lineage doesn't solve: P2P distribution (OCI is HTTPS-pull only), trust roots without OIDC, key rotation, install UX, content-addressed P2P interop |
| [`lessons.md`](./lessons.md) | **the decision file:** validates / avoid / borrow — synthesis for Myrhiza spec authors |

## Quick orientation

**What the WASM ecosystem standardized on (2024–2025):** Components ship as OCI artifacts. The "OCI artifact" is a manifest pointing at a `.wasm` blob (or multiple, for wac-composed bundles + interface deps + UI assets), with `artifactType` declaring the content kind and an optional `subject` linking signature / SBOM / attestation manifests via the Referrers API. The actual transport is just standard `pull` / `push` against any compliant registry — GHCR, Docker Hub, ECR, ACR, Harbor, ttl.sh, or a self-hosted `registry:2`. ORAS is the CLI / Go library for treating non-image artifacts as first-class.

**Where wkg sits:** `wkg` is the Bytecode Alliance's wasm-aware client on top of OCI. It knows about WIT package dependencies, the canonical CM media types, and component metadata. Use `wkg get foo:bar@1.2.3` and it resolves to the right OCI ref. Practical equivalent: "what `cargo` is to crates.io, `wkg` is to OCI-as-component-registry."

**Where wac sits:** Build-time composition. Two components in, one component out. The composed artifact is still a CM `.wasm` and ships through the same OCI pipeline. Runtime composition (Spin's host-side `add_to_linker`, wasmCloud's wRPC) is unrelated.

**The bundle vs transport split:** OCI is the transport. The bundle is a CM `.wasm` (possibly wac-composed). **Never conflate the two.** Holochain's `.happ` and Endo's bundle-hash are alternative bundle shapes; both could ship over OCI without contradiction (they don't currently).

**The signing overlay:** Sigstore + Notation are *layered on top* of OCI 1.1's Referrers API. Both attach signatures as separate manifests whose `subject` field points back at the artifact being signed. Cosign is the OIDC-keyless-by-default flavor; Notation is the X.509-PKI-by-default flavor. Both work against any OCI 1.1 registry.

## Quick neighbours

| Neighbour | Relationship |
|---|---|
| [`wasm-component-model/`](../wasm-component-model/) | The bundle this folder transports. `wkg` + `wac` live in the CM tooling ecosystem. |
| [`spin/`](../spin/) | Reference adopter of `spin registry push/pull` over OCI + `wkg` package resolution + `wac` composition. |
| [`wasmcloud/`](../wasmcloud/) | Reference adopter of `wash oci push/pull` for components + capability providers. |
| [`holochain/`](../holochain/) | Alternative bundle shape (`mr_bundle`) on a fully different (DHT-based) transport. The "what if we don't use OCI" data point. |
| [`agoric-endo/`](../agoric-endo/) | The bundle-hash + Compartment-Map story for the ocap-runtime cousin. Endo doesn't use OCI; the bundle-hash discipline transfers. |
| [`jco/`](../jco/) | Browser-side delivery. ESM + import maps fill the role OCI fills for native. |
| [`iroh/`](../iroh/) | Content-addressed P2P blob transport. The "what if OCI were dial-by-hash instead of dial-by-name" alternative. |

## Sources

- OCI Image-spec: <https://github.com/opencontainers/image-spec>
- OCI Distribution-spec: <https://github.com/opencontainers/distribution-spec>
- OCI 1.1 announcement (2024-03-13): <https://opencontainers.org/posts/blog/2024-03-13-image-and-distribution-1-1/>
- OCI Artifacts (archived 2023-07-18): <https://github.com/opencontainers/artifacts>
- ORAS: <https://oras.land>, <https://github.com/oras-project/oras>
- wasm-pkg-tools (`wkg`): <https://github.com/bytecodealliance/wasm-pkg-tools>
- `wac`: <https://github.com/bytecodealliance/wac>
- `warg` (archived 2025-07-28): <https://github.com/bytecodealliance/registry>
- Sigstore: <https://www.sigstore.dev>, <https://docs.sigstore.dev>
- Cosign: <https://github.com/sigstore/cosign>
- Notary Project: <https://notaryproject.dev>
- Notation: <https://github.com/notaryproject/notation>
- CNCF TAG Runtime WASM OCI artifact layout: <https://tag-runtime.cncf.io/wgs/wasm/deliverables/wasm-oci-artifact/>
- HTML import maps (HTML Standard): <https://html.spec.whatwg.org/multipage/webappapis.html#import-maps>
- Spin OCI (SIP-008): <https://github.com/spinframework/spin/blob/main/docs/content/sips/008-using-oci-registries.md>
