**Date:** 2026-05-22
**Status:** active
**Subject:** Timeline. How OCI + WASM-on-OCI + Sigstore + Notation reached today's shape.

# History

How the WASM-app-distribution stack assembled. Dates verified 2026-05-22 from the corresponding repos' releases pages and project announcements.

## 2013–2017: Containers establish the registry pattern

- **2013-03-13:** `docker` v0.1 — the original "registry that serves layered tarballs" concept.
- **2013-09-09:** `docker registry` (renamed `distribution`) split out as a separate component.
- **2015-06-22:** **Open Container Initiative (OCI)** founded under Linux Foundation, primary aim to standardize Docker's image + runtime formats. Founding members: Docker, AWS, Google, Microsoft, Red Hat, IBM, others.
- **2017-07-19:** **OCI Image Spec v1.0** GA. Defines manifest, descriptor, layer, blob, media-type, content-addressing.
- **2017-07-19:** **OCI Distribution Spec v1.0** (slightly later — separate working group) defines the HTTP API surface.

## 2018–2020: ORAS, Artifacts, and the WASM-OCI crossover

- **2019-09:** Microsoft launches the [ORAS project](https://oras.land) — "OCI Registry As Storage" — proving you can push arbitrary blobs to Docker Hub. Originally a Microsoft / Azure ACR effort; later joined CNCF.
- **2019-12-11:** OCI Image-spec v1.0.1 — patch release.
- **2019:** **OCI Artifacts working group** (separate repo at [`opencontainers/artifacts`](https://github.com/opencontainers/artifacts)) kicks off, exploring how to extend OCI for non-image content. *(This repo will be archived in 2023 once its conclusions land in image-spec 1.1.)*
- **2020:** First WASM-on-OCI experiments. Solo.io's `wasme` tool, Krustlet (WASM-on-Kubernetes), and various Bytecode Alliance experiments demonstrate that WASM components can ship as OCI artifacts.

## 2021: Sigstore launches, Spin debuts

- **2021-03-09:** **Sigstore** announced — Luke Hinds (Red Hat) + Dan Lorenc (Google) + others. Initial repos: cosign, fulcio, rekor.
- **2021-09-15:** Sigstore moves under the OpenSSF umbrella as an Incubating project.
- **2021-10-27:** **Fermyon** founded (Matt Butcher, Radu Matei, Bailey Hayes, others — Microsoft DeisLabs alumni).
- **2021-10:** **CNCF Wasm Microsurvey** — early industry signal that WASM is becoming a first-class deployable artifact.

## 2022: Production sign-off

- **2022-03-21:** **Fermyon Spin** 0.1 released. First serious WASM-OCI distribution flow (`spin registry push/pull`) in the open.
- **2022-04-21:** [Bytecode Alliance announces](https://bytecodealliance.org/articles/component-model) the WebAssembly Component Model preview, including the WIT IDL.
- **2022-10-26:** **SigstoreCon (Detroit):** Sigstore GA. Cosign + Fulcio + Rekor declared production-ready. *"The Sigstore APIs are now ready for enterprise use."* (CNCF blog)
- **2022-12:** Initial drafts of the artifactType-on-manifest proposal merge into OCI Image-spec PR queue.

## 2023: Wargs, RFCs, transparency-log adoption

- **2023-01:** **`wkg` (`wasm-pkg-tools`)** initial commits at Bytecode Alliance.
- **2023-01:** **Bytecode Alliance `warg`** (registry) project announced — purpose-built signed Wasm registry protocol.
- **2023-04-19:** **SLSA v1.0** GA — supply-chain provenance levels formalized.
- **2023-04-21:** **TLA+ Foundation** launches under Linux Foundation (relevant adjacent context; TLA+ is upstream of the verification overlay).
- **2023-05-18:** **npm provenance** (Sigstore-backed) ships — first hyperscale signing adoption.
- **2023-07-18:** **`opencontainers/artifacts` repo archived.** Its work merges into image-spec / distribution-spec 1.1 RC.
- **2023-09-12:** **in-toto** graduates CNCF.
- **2023:** **OCI v1.1 release-candidate cycle.** 1.1-rc1 → 1.1-rc6 over the year.
- **2023:** **Notation v1.0.0** GA. CNCF Incubating maturity.

## 2024: OCI 1.1 ships, signing matures

- **2024-02-15:** **OCI Image-spec v1.1.0 GA** and **Distribution-spec v1.1.0 GA**, simultaneously. The `artifactType` field, the `subject` field, and the Referrers API all land. *This is the load-bearing date for the entire modern WASM-OCI distribution model.*
- **2024-03-13:** Official OCI blog post announces the 1.1 releases — including the deprecation of non-distributable layers and the formal artifact / referrers model.
- **2024-04-17:** **`wac` v0.10.0** released.
- **2024-05:** Homebrew adopts Sigstore signing.
- **2024-08-12:** **SLSA v1.1** patch release.
- **2024-09-25:** Microsoft OSS blog ["Distributing WebAssembly Components Using OCI Registries"](https://opensource.microsoft.com/blog/2024/09/25/distributing-webassembly-components-using-oci-registries/) — demonstrates the full WASM-OCI pipeline with `wkg`.
- **2024-10:** **Rekor v2** GA — significant transparency-log scalability improvements.
- **2024-11:** PyPI adopts Sigstore signing.

## 2025: warg dies, OCI wins

- **2025-01:** Maven Central adopts Sigstore.
- **2025-01-21:** Spin and SpinKube accepted to **CNCF Sandbox** (jointly, same day). `fermyon/spin` repo migrated to `spinframework/spin` for CNCF vendor-neutrality.
- **2025-02-06:** **`wkg` v0.15.0** released — OCI dependencies + time crate bumped, latest stable.
- **2025-02-26:** WICG **`import-maps`** repo archived. Spec moves to HTML Standard.
- **2025-03-03:** **OCI Image-spec v1.1.1** and **Distribution-spec v1.1.1** — patch-level clarifications. Current target for compliance.
- **2025-03-12:** **`oras` v1.3.1** released.
- **2025-03-13:** **Notation v2.0.0-alpha.1** released — preview of v2 with blob signing.
- **2025-04-06:** **`cosign` v3.0.6** released. Cosign 3 = Referrers-by-default, legacy tag-based discovery removed.
- **2025-04-18:** **`oras` v1.3.2** released — current stable, OCI distribution-spec 1.1.1 compliant.
- **2025-04-27:** **Notation v1.3.2** released — current stable on the v1 line.
- **2025-07-28:** **`bytecodealliance/registry` (warg) archived.** "Work on an OCI-based registry system continues in the bytecodealliance/wasm-pkg-tools repository." End of the warg-protocol vision; OCI wins outright.
- **2025-12-01:** **Akamai acquires Fermyon.** Spin + SpinKube continue as open-source CNCF projects.

## 2026: where we are

As of 2026-05-22:

- **OCI 1.1 is the canonical transport.** Every major registry (GHCR, ACR, ECR, Quay, Harbor, Zot) supports the artifactType + Referrers model.
- **Sigstore + Notation both ship signatures via Referrers.** Two trust roots, one attachment mechanism.
- **`wkg` is the WASM-aware client.** Small-team BA-stewarded, structurally less resilient than `oras`. v0.15.0 is current.
- **`wac` is the canonical composition tool.** v0.10.0; less actively developed than `wkg` since 2024-04 release.
- **WASM-on-OCI media types are stabilizing** under the CNCF TAG Runtime artifact layout but no formal IANA registration yet.
- **The browser-side story (ESM + import maps + SRI) is widely-available Baseline** but offers no first-class signing — the structural gap remains unfilled.
- **Holochain `mr_bundle`, Endo bundle-hash, Pears Hypercore** remain alternative not-OCI bundle paths, each with constituent design lessons but no convergence with the WASM-OCI consensus.

## The unfilled chapters

- **P2P-native OCI.** No registry protocol or client treats OCI artifacts as P2P-replicable content-addressed blobs. Closest is BitTorrent-style overlays atop existing registries.
- **Browser author signing.** SRI is hash-only; OCI-Cosign equivalent for browsers doesn't exist as a standard.
- **WASM-internal SBOMs.** Syft + similar tools don't parse CM custom sections. Source-language deps only.
- **Cross-registry trust composition.** A trust policy that says "signatures from registry A AND registry B trusted" isn't first-class in any tool.
- **Trust-root distribution in P2P networks.** Cosign / Notation both assume operators push trust policies to peers. P2P networks don't have operators.

The next 12–18 months in the ecosystem will probably narrow these gaps. Myrhiza spec authors should plan for them being plausibly-filled-by-2027 rather than permanently-broken.

## Sources

- OCI Image-spec releases: <https://github.com/opencontainers/image-spec/releases>
- OCI Distribution-spec releases: <https://github.com/opencontainers/distribution-spec/releases>
- OCI 1.1 announcement: <https://opencontainers.org/posts/blog/2024-03-13-image-and-distribution-1-1/>
- OCI Artifacts archive: <https://github.com/opencontainers/artifacts>
- Sigstore GA at SigstoreCon: CNCF blog 2022-10-26
- ORAS releases: <https://github.com/oras-project/oras/releases>
- `wkg` releases: <https://github.com/bytecodealliance/wasm-pkg-tools/releases>
- `wac` releases: <https://github.com/bytecodealliance/wac/releases>
- `warg` archive: <https://github.com/bytecodealliance/registry>
- Cosign releases: <https://github.com/sigstore/cosign/releases>
- Notation releases: <https://github.com/notaryproject/notation/releases>
- Fermyon-Akamai acquisition (2025-12-01): see [`spin/governance.md`](../spin/) and Akamai press release
- import-maps archive: <https://github.com/WICG/import-maps>
- SLSA v1.0 GA: <https://slsa.dev/blog/2023/04/slsa-v1.0-release>
- in-toto CNCF Graduation: <https://www.cncf.io/announcements/2023/09/12/cloud-native-computing-foundation-announces-in-toto-graduation/>
