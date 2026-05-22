**Date:** 2026-05-22
**Status:** active
**Subject:** OCI artifacts — the WASM ecosystem's transport substrate. Image-spec 1.1 + Distribution-spec 1.1 + ORAS.

# OCI artifacts

## What it is, in one sentence

An **OCI artifact** is any content addressable in an OCI registry — not just container images. Image-spec 1.1 made this a first-class concept by adding the `artifactType` field to the manifest and letting registries treat arbitrary blobs (WASM components, Helm charts, SBOMs, signatures, ML models) as a peer to traditional Docker images.

## The two-spec stack

OCI is governed by two specs that move in lockstep:

| Spec | Repo | What it defines |
|---|---|---|
| **Image spec** | [`opencontainers/image-spec`](https://github.com/opencontainers/image-spec) | On-the-wire JSON format of manifests, indexes, descriptors, media types |
| **Distribution spec** | [`opencontainers/distribution-spec`](https://github.com/opencontainers/distribution-spec) | HTTP API surface registries expose for pull, push, mount, discovery |

A third spec — `runtime-spec` — defines how an unpacked container image runs. **It is not relevant to WASM distribution** because WASM components are not unpacked and run by an OCI runtime; they're pulled and then handed to a Wasmtime / wasmCloud / Spin host. Image-spec + distribution-spec are the two that matter.

The **archived [`opencontainers/artifacts`](https://github.com/opencontainers/artifacts) repo** is a separate, historically distinct project that ran 2019–2023 as a working group exploring how to extend OCI beyond images. It was archived 2023-07-18 once the two main specs absorbed its conclusions into 1.1.0. **Never cite the archived `artifacts` repo as authoritative** — image-spec 1.1+ is the live spec for what `artifacts` was trying to standardize.

## The OCI image-spec 1.1.0 change (2024-02-15)

Three additions made WASM distribution clean:

### 1. `artifactType` field on the manifest

Before 1.1.0: every manifest's "type" was inferred from `config.mediaType`, which was load-bearing and ambiguous for non-image artifacts (people had been overloading it since 2019 via the "artifacts working group" guidance).

After 1.1.0: a top-level `artifactType` field on the manifest declares the content kind directly. WASM components ship as e.g. `application/vnd.bytecodealliance.component.v0+wasm` (or one of the [CNCF TAG Runtime WASM OCI artifact layout](https://tag-runtime.cncf.io/wgs/wasm/deliverables/wasm-oci-artifact/) media types).

This decoupled "is this an image?" from "what's the manifest format?" and let registries serve WASM artifacts without lying about being container images.

### 2. `subject` field + the Referrers API

The `subject` field on a manifest declares "this artifact is associated with `<digest>`." A signature manifest's `subject` points at the signed artifact. An SBOM manifest's `subject` points at the artifact it documents. An attestation's `subject` points at the artifact attested.

The distribution-spec 1.1.0 paired this with the **Referrers API**:

```
GET /v2/<name>/referrers/<digest>
```

Returns an OCI Index listing every manifest in the registry whose `subject` matches `<digest>`. This is how Sigstore Cosign 2+ and Notation v2 attach signatures: you push the signature as a sibling manifest with `subject` set, and the verifier discovers it via Referrers.

The Referrers API replaced Cosign's earlier tag-based "find a signature by signing convention" pattern (e.g. `sha256-abc.sig`), which was a hack. Tag-based discovery is now deprecated and Cosign 3 / Notation v2 default to Referrers.

### 3. Deprecation of non-distributable layers

OCI 1.1 formally deprecated the "non-distributable layer" concept (used historically for Windows base layers with licensing constraints). Irrelevant for WASM but worth knowing if you grep the spec.

## OCI image-spec 1.1.1 (2025-03-03)

Patch-level clarifications, JSON schema fixes, no new fields. Distribution-spec 1.1.1 shipped the same day with matching clarifications. **1.1.1 is the current target for compliance** — `oras` v1.3.0+ and `wkg` v0.12.0+ both claim 1.1.1 conformance.

## The data model in 4 layers

```
Index (optional, multi-arch / variant grouping)
  ↓ refers to
Manifest (the artifact itself — one per platform / variant)
  ↓ refers to
Descriptor (a digest + media type + size)
  ↓ points at
Blob (the actual bytes — config.json + N layers)
```

For a WASM component, the typical shape:

```
Manifest
├── artifactType: application/vnd.bytecodealliance.component.v0+wasm
├── config: Descriptor → {sha256:abc} (component config blob, small)
├── layers: [
│     Descriptor → {sha256:def} (the .wasm bytes, single blob)
│   ]
└── annotations: {org.opencontainers.image.created, ...}
```

For a wac-composed multi-component bundle, layers grow:

```
Manifest
├── artifactType: application/vnd.bytecodealliance.component.v0+wasm
├── config: Descriptor → {sha256:abc}
├── layers: [
│     Descriptor → {sha256:def} (composed.wasm)
│     Descriptor → {sha256:ghi} (state-apply.wasm — original component, also published)
│     Descriptor → {sha256:jkl} (state-propose.wasm)
│     ...
│   ]
```

The pattern Spin (SIP-008) and wasmCloud both use: **one manifest per app, one layer per component, plus a config blob naming the entry point.** Cross-component dependency resolution happens at compose time, not at registry time.

## ORAS — the artifact-aware CLI

[ORAS](https://oras.land) (OCI Registry As Storage) is the project that drove artifact-as-first-class through OCI:

- **`oras` CLI** — Go, Apache-2.0, current stable v1.3.2 (2025-04-18). Compliant with OCI distribution-spec 1.1.1.
- **`oras-go`** — Go library powering the CLI; embedded by many tools (Helm 3.8+, Notation, ORAS Artifacts).
- **`oras-py`** — Python port for SBOM / ML model use cases.

Core commands:

```
oras push     ghcr.io/foo/bar:0.1.0 file.wasm   # upload artifact + metadata
oras pull     ghcr.io/foo/bar:0.1.0              # download
oras attach   ghcr.io/foo/bar:0.1.0 sbom.json --artifact-type application/spdx+json
oras discover ghcr.io/foo/bar:0.1.0              # use Referrers API to list attachments
oras manifest fetch ghcr.io/foo/bar:0.1.0        # inspect raw manifest
oras blob     fetch ghcr.io/foo/bar@sha256:def   # fetch raw blob by digest
oras tag      ghcr.io/foo/bar:0.1.0 ghcr.io/foo/bar:latest
oras copy     ghcr.io/src/x:1 ghcr.io/dst/x:1    # cross-registry copy preserving digests
oras login    ghcr.io                            # docker-style cred storage
```

For Myrhiza spec authors: **`oras` is the right exploratory tool**. `wkg` does more (WIT-aware resolution) but `oras` is the lower-level "what's actually in the registry" view. `oras manifest fetch` is the first command to run when debugging anything.

## Registry compliance landscape (verified 2026)

| Registry | OCI 1.1.0 + Referrers | Notes |
|---|---|---|
| GitHub Container Registry (`ghcr.io`) | Yes | Free for public artifacts; primary OSS host |
| Docker Hub | Yes | Free tier rate-limited; pull-through proxy common |
| AWS Elastic Container Registry (ECR) | Yes (since 2023-11) | Pay per GB-month |
| Azure Container Registry (ACR) | Yes | Pay per registry-day |
| Google Artifact Registry | Yes | Multi-format (Docker + Maven + npm + ...) |
| Quay.io (Red Hat) | Yes | Free public tier |
| Harbor (self-hosted) | Yes (Harbor 2.7+) | Apache-2.0 CNCF Graduated; SBOM scanning built in |
| `registry:2` (Distribution reference) | Yes (registry:2.8+) | Apache-2.0; the reference impl |
| `zot` (Project Zot) | Yes | CNCF Sandbox; OCI-native, no Docker compat baggage |
| `ttl.sh` | Yes | Anonymous, ephemeral (1h–24h TTL); good for testing |
| JFrog Artifactory | Partial (1.0; 1.1 Referrers limited) | Commercial |

**The Referrers API is the practical compliance bar** — if a registry serves it, signatures and SBOMs work cleanly. Almost everything modern does. The exception list shrinks every quarter.

## Implications for Myrhiza

**The good news:** if Myrhiza adopts OCI as canonical transport, the entire registry ecosystem works for free. No new infrastructure. GHCR is free for public components; ttl.sh is free for ephemeral CI testing; self-hosted `registry:2` or `zot` are Apache-2.0 if a Myrhiza network ever wants a private registry.

**The mismatch:** OCI is **pull-by-name** over HTTPS, not **pull-by-hash** over a P2P transport. Myrhiza's P2P story (iroh) is dial-by-pubkey content-addressed. The bridge — "discover artifact by name in registry → resolve to digest → fetch from peers by digest" — is unwritten ecosystem work. See [`open-problems.md`](./open-problems.md) §1.

**The artifactType choice:** Myrhiza will need to pick (or coin) a canonical `artifactType` value. The CNCF TAG Runtime layout proposes `application/vnd.bytecodealliance.component.v0+wasm` for plain components. If Myrhiza apps are heterogeneous (multi-component + manifest + assets), a Myrhiza-specific artifactType `application/vnd.myrhiza.app.v0` is the right call — and we should publish the media-type registration to IANA per RFC 6838 if the format stabilizes.

**The Referrers contract is load-bearing for signing.** Whatever signing story Myrhiza adopts (see [`signing.md`](./signing.md)), it should attach via the Referrers API rather than tag conventions. Cosign 3 / Notation v2 have already converged on this; copying their lead avoids inventing.

## Sources

- OCI Image-spec: <https://github.com/opencontainers/image-spec>
- OCI Distribution-spec: <https://github.com/opencontainers/distribution-spec>
- OCI 1.1 announcement (2024-03-13): <https://opencontainers.org/posts/blog/2024-03-13-image-and-distribution-1-1/>
- OCI Artifacts archived repo (2023-07-18): <https://github.com/opencontainers/artifacts>
- ORAS: <https://oras.land>, <https://github.com/oras-project/oras>
- CNCF TAG Runtime WASM OCI artifact layout: <https://tag-runtime.cncf.io/wgs/wasm/deliverables/wasm-oci-artifact/>
- Microsoft "Distributing WebAssembly Components Using OCI Registries" (2024-09-25): <https://opensource.microsoft.com/blog/2024/09/25/distributing-webassembly-components-using-oci-registries/>
- Project Zot: <https://github.com/project-zot/zot>
- Harbor: <https://goharbor.io>
