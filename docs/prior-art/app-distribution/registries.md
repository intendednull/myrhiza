**Date:** 2026-05-22
**Status:** active
**Subject:** OCI registry landscape, the dead `warg` protocol, CNCF TAG Runtime media types, and what a "Myrhiza registry" would look like.

# Registries

Where artifacts actually live. OCI's federation-by-convention model means there's no central registry — everything is a `Distribution-spec` HTTP API behind some hostname. The choice of which hostname matters more for trust, free-tier limits, and the network's politics than for technical capability.

## The hosted-registry landscape (verified 2026-05)

### Public free tiers (best for OSS WASM components)

| Registry | Provider | Free tier | OCI 1.1 + Referrers | Notes |
|---|---|---|---|---|
| **GitHub Container Registry (`ghcr.io`)** | GitHub / Microsoft | Unlimited public artifacts; storage costs only for private | Yes | Default for Bytecode Alliance demos; integrates with GitHub Actions; supports anonymous pull |
| **Docker Hub** | Docker, Inc. | Limited (rate-limited anon pulls, public free for individuals) | Yes | Original; rate-limit policy unpopular; still default in many CI configs |
| **Quay.io** | Red Hat | Free public repos | Yes | Solid OCI 1.1 conformance; CVE scanning built in |
| **`ttl.sh`** | (community-run) | Anonymous, ephemeral (1h–24h TTL); no account needed | Yes | Best for CI / demo / one-off testing. Cannot persist artifacts beyond TTL |

### Cloud-hosted paid tiers

| Registry | Provider | Pricing | OCI 1.1 + Referrers | Notes |
|---|---|---|---|---|
| **AWS Elastic Container Registry (ECR)** | AWS | Per GB-month + per-request | Yes (since 2023-11) | Multi-region replication; tight IAM integration |
| **Azure Container Registry (ACR)** | Microsoft | Per registry-day, tiered | Yes | Trust Signing service tightly Notation-integrated |
| **Google Artifact Registry** | Google Cloud | Per GB-month | Yes | Multi-format (Docker + Maven + npm + WASM) |
| **JFrog Artifactory** | JFrog | Commercial subscription | Partial (1.0 OK; 1.1 Referrers patchy) | Common in enterprises; worst OCI conformance of the major commercial registries as of 2026 |

### Self-hosted (Apache-2.0 open-source)

| Registry | Repo | OCI 1.1 + Referrers | Notes |
|---|---|---|---|
| **`registry:2` (Distribution reference)** | [`distribution/distribution`](https://github.com/distribution/distribution) | Yes (2.8+) | Apache-2.0; the reference impl from CNCF; the `registry` Docker image |
| **Harbor** | [`goharbor/harbor`](https://github.com/goharbor/harbor) | Yes (2.7+) | CNCF Graduated; SBOM scanning, signing, RBAC, replication |
| **Project Zot** | [`project-zot/zot`](https://github.com/project-zot/zot) | Yes | CNCF Sandbox; OCI-native (no Docker baggage); excellent for artifact-first deployments |
| **CNCF Distribution** | (rebrand of registry:2) | Yes | Same project, different name post-CNCF-graduation 2023 |

**Practical guidance for Myrhiza spec authors:** if a spec needs a registry to talk about, default to GHCR for examples (free, ubiquitous) and Project Zot for "the registry the Myrhiza community could self-host." Avoid Docker Hub examples — the rate-limit story makes them brittle in CI.

## CNCF TAG Runtime media types — the WASM-on-OCI standardization

The CNCF Technical Advisory Group for Runtime (TAG Runtime) maintains the [WASM OCI artifact layout document](https://tag-runtime.cncf.io/wgs/wasm/deliverables/wasm-oci-artifact/), which standardizes media types for WASM content in OCI registries:

| Media type | What | Used by |
|---|---|---|
| `application/wasm` | A raw `.wasm` blob — could be a core module or a component | (layer media-type, universal) |
| `application/vnd.bytecodealliance.component.v0+wasm` | An OCI artifact wrapping a CM component | `wkg`, `wasm-tools` |
| `application/vnd.wasm.config.v0+json` | Config blob describing a WASM artifact | (general WASM-OCI configs) |
| `application/vnd.bytecodealliance.wasm.wit.v0+wasm` | An OCI artifact wrapping a WIT package (interface only, no component code) | `wkg wit fetch/build` |
| `application/vnd.wasm.content.layer.v1+wasm` | Used by some Spin/wasmCloud variants | Spin SIP-008 |

The status of these media types is still **stabilizing** — the BA pattern (`application/vnd.bytecodealliance.*`) is more widely deployed than the bare `application/vnd.wasm.*`. Specs SHOULD treat them as informational rather than fixed; expect renames before any final standardization.

For Myrhiza: a Myrhiza-specific `artifactType` (e.g. `application/vnd.myrhiza.app.v0`) is the right call if Myrhiza apps carry semantics beyond "this is a CM component" (e.g. manifest metadata, multi-profile component layout, expected kernel version). Register it with IANA per [RFC 6838](https://www.rfc-editor.org/rfc/rfc6838.html) once the format stabilizes.

## The `warg` post-mortem

`warg` was Bytecode Alliance's attempt to build a **Wasm-native registry protocol** — a checkpoint-log-backed, signed-publication registry purpose-designed for WASM components, with first-class versioning, transparency-log semantics, and a publish-by-signed-record flow.

- **Repo:** [`bytecodealliance/registry`](https://github.com/bytecodealliance/registry)
- **License:** Apache-2.0
- **Archived:** 2025-07-28
- **Reason:** "OCI won." From the archived repo README: *"This repository is no longer being actively developed by Bytecode Alliance members. Work on an OCI-based registry system continues in the bytecodealliance/wasm-pkg-tools repository."*

### What `warg` got right (worth carrying forward as design lessons even though the implementation is dead)

- **Signed publication records.** Every publish was a cryptographically signed event. The full registry history was a verifiable log.
- **Per-package transparency.** Each package had its own checkpoint log; consumers could verify the publication history independently of trusting the central server.
- **Version-as-immutable.** Once a version was published, it was content-addressed and immutable. No "tag retag" attacks. (OCI offers this via digest pinning but not by default.)

### What killed it

- **Infrastructure cost.** Operating a registry of any scale is expensive; running one with the additional cryptographic-log overhead even more so. No funded operator emerged.
- **OCI registries existed already.** Every developer already had `docker login ghcr.io` working. Asking the world to authenticate against a new registry protocol was an uphill battle.
- **Signing landed elsewhere.** Sigstore + Notation provided the signed-publication story over OCI as an attached layer — the killer feature `warg` was building toward arrived via OCI 1.1 + Referrers + Cosign before `warg` could ship a stable v1.
- **Small team, big scope.** Building a registry protocol, reference server, client library, transparency log, key-rotation story, and policy engine with ~3 maintainers wasn't tractable.

### What's still alive of the `warg` vision

- The `wkg` config supports `type = "warg"` as a backend stub.
- The protocol spec is preserved in the archive for anyone who wants to revive.
- Some of the signed-publication thinking informed Sigstore Rekor v2 design.

**Lesson for Myrhiza:** "build our own registry protocol" looks tempting and is almost always wrong. OCI's federation model + the Sigstore/Notation overlay covers what `warg` was trying to cover, with two orders of magnitude more infrastructure already deployed. Pick OCI; don't pick warg-shaped fights.

## What a "Myrhiza registry" might look like

If Myrhiza adopts OCI as canonical transport, a "Myrhiza registry" is just an OCI registry (any of the above) hosting artifacts with a Myrhiza-specific `artifactType`. No custom infrastructure required.

The interesting Myrhiza-specific layers would be:

- **A P2P resolution overlay.** OCI gives "name → digest"; iroh gives "digest → bytes via peers." A Myrhiza-aware client resolves the name via OCI, then prefers peer transport over registry transport for the actual blob. Compare BitTorrent's `magnet:` links resolving to trackers + DHT, except via OCI.
- **A trust-policy distribution mechanism.** Cosign/Notation verify signatures against a `trustpolicy.json` — but in a P2P network there's no central admin. A spec for "how does a Myrhiza peer get its trust roots" is unwritten. See [`open-problems.md`](./open-problems.md) §trust-roots.
- **A reputation / discovery layer.** OCI registries don't recommend apps; they're flat namespaces. Myrhiza-aware discovery (which apps does my social graph use? which apps are signed by people I trust?) is application-layer ecosystem work.

**Don't:** stand up a Myrhiza-specific central registry. The cost > value calculation that killed `warg` will kill ours too. Federated against existing OCI registries is the only sustainable model.

## Cross-registry mobility

A nice OCI property: artifacts can be **copied between registries preserving their digest**. `oras copy ghcr.io/src/x:1 ghcr.io/dst/x:1` produces an identical digest at the destination because OCI manifests + layer blobs are content-addressed.

This means **signatures continue to verify across copies** — `cosign verify` is digest-anchored, so once an artifact is signed at GHCR, it remains validly signed when mirrored to a private Harbor. Compose this with Myrhiza's P2P story: the same signature accompanies the artifact whether it's pulled from a registry or replicated over iroh.

**Practical implication:** Myrhiza can rely on artifact + signature mobility without inventing portability machinery. A Myrhiza app published to GHCR by its author can be re-served by a Myrhiza peer to other peers without breaking the signature chain.

## Sources

- OCI Distribution-spec: <https://github.com/opencontainers/distribution-spec>
- CNCF TAG Runtime WASM OCI artifact layout: <https://tag-runtime.cncf.io/wgs/wasm/deliverables/wasm-oci-artifact/>
- Distribution reference impl (`registry:2`): <https://github.com/distribution/distribution>
- Harbor: <https://goharbor.io>
- Project Zot: <https://zotregistry.dev>
- `warg` archive: <https://github.com/bytecodealliance/registry>
- ttl.sh: <https://ttl.sh>
- GHCR docs: <https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry>
- IANA media type registration RFC 6838: <https://www.rfc-editor.org/rfc/rfc6838.html>
