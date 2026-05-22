**Date:** 2026-05-22
**Status:** active
**Subject:** SBOM (SPDX/CycloneDX), in-toto attestations, SLSA provenance, and how OCI 1.1 + Sigstore wire them together.

# Supply chain

Signing answers "did this come from a claimed identity?" Supply-chain attestations answer "how was it built?" Both attach to OCI artifacts via the same Referrers mechanism (see [`signing.md`](./signing.md)); the distinction is in what's signed.

## SBOM (Software Bill of Materials)

An SBOM is a machine-readable inventory of an artifact's components and dependencies. Two competing formats; both production-grade:

### SPDX

- **Org:** Linux Foundation
- **Current spec:** SPDX 3.0.1 (2024-09-12 RC; 3.0 GA 2024-04-15)
- **Format:** JSON-LD, RDF, or tag-value text
- **Strength:** mature; license-tracking-first; ISO/IEC 5962:2021 standard
- **OCI media type:** `application/spdx+json`

### CycloneDX

- **Org:** OWASP
- **Current spec:** CycloneDX 1.6 (2024-04-12)
- **Format:** JSON or XML
- **Strength:** vulnerability-tracking-first; cleaner spec than SPDX for non-license use cases; broad tool support
- **OCI media type:** `application/vnd.cyclonedx+json`

### How SBOMs attach to OCI artifacts

```bash
oras attach --artifact-type application/spdx+json ghcr.io/foo/bar:0.1.0 sbom.spdx.json
# or
cosign attest --predicate sbom.cdx.json --type cyclonedx ghcr.io/foo/bar:0.1.0
```

Both produce a new OCI manifest with `subject` pointing at the original artifact's digest. `oras discover ghcr.io/foo/bar:0.1.0` lists them.

### Generators

| Tool | SBOM format | Source language |
|---|---|---|
| **Syft** ([anchore/syft](https://github.com/anchore/syft)) | SPDX + CycloneDX | binary / container scanning (multi-language) |
| **cargo-sbom** | SPDX | Rust (Cargo metadata → SPDX) |
| **cargo-cyclonedx** | CycloneDX | Rust |
| **CycloneDX npm-bom** | CycloneDX | npm |
| **`syft <oci-ref>`** | SPDX + CycloneDX | OCI artifact (peeks inside layers) |

For WASM components: Syft as of mid-2025 has experimental WASM support but does not parse CM custom sections; SBOM generation for components is unsolved beyond "list the source-language deps." This is an ecosystem gap.

## in-toto attestations

[in-toto](https://in-toto.io) (CNCF Graduated 2023-09-12) is the framework for **attestations** — signed JSON statements about software artifacts. The attestation format is:

```json
{
  "_type": "https://in-toto.io/Statement/v1",
  "subject": [
    { "name": "ghcr.io/foo/bar", "digest": { "sha256": "abc..." } }
  ],
  "predicateType": "https://slsa.dev/provenance/v1",
  "predicate": { ... }  // free-form, predicateType-specific
}
```

The `predicate` slot carries any structured claim — provenance, SBOM, test result, audit log entry, vulnerability scan, code review record. Each predicate type has its own schema URL.

**Attestation envelope:** wrapped in [DSSE](https://github.com/secure-systems-lab/dsse) (Dead Simple Signing Envelope) for signing. DSSE provides authenticated payload + signature + envelope-binding without the JWT pitfalls.

### How attestations attach to OCI artifacts

```bash
cosign attest --predicate provenance.json --type slsaprovenance ghcr.io/foo/bar:0.1.0
```

Same Referrers shape as everything else. The attestation manifest has `subject` set to the original artifact's digest.

## SLSA (Supply-chain Levels for Software Artifacts)

[SLSA](https://slsa.dev) is the framework defining provenance levels for build pipelines. **SLSA v1.0 GA 2023-04-19**. Current spec: SLSA v1.1 (2024-08-12).

### Levels

| Level | Requirements | Practical signal |
|---|---|---|
| **0** | None | "I have an artifact" |
| **1** | Build process documented; provenance produced | "I know where it came from" |
| **2** | Hosted build service; signed provenance | "A third-party hosted system attests the build" |
| **3** | Hardened build platform; non-falsifiable provenance | "The build platform can't be tricked" |
| **(4)** | Removed in v1.0 — was "two-party review" | (rolled into v1.0 broader requirements) |

### What SLSA provenance contains

```json
{
  "buildDefinition": {
    "buildType": "https://github.com/actions/runner",
    "externalParameters": { ... },     // inputs
    "internalParameters": { ... },     // controlled inputs
    "resolvedDependencies": [ ... ]     // pinned deps with digests
  },
  "runDetails": {
    "builder": { "id": "https://github.com/actions/runner" },
    "metadata": {
      "invocationId": "...",
      "startedOn": "...",
      "finishedOn": "..."
    },
    "byproducts": [ ... ]
  }
}
```

The provenance is signed (typically Cosign-keyless against the build platform's OIDC identity, e.g. `https://token.actions.githubusercontent.com`). The verifier checks: signer identity (was this signed by GitHub Actions, not a random user?), claimed-build-platform identity (matches expected workflow path?), build inputs (do they match what was expected?).

### How SLSA provenance attaches to OCI

Most modern CI emits SLSA provenance as a side effect of building. GitHub Actions has the [slsa-github-generator](https://github.com/slsa-framework/slsa-github-generator) action that produces L3 provenance for any artifact. The provenance is then attached as a Cosign attestation.

### What "SLSA L3 wasm component" looks like in practice

```yaml
# .github/workflows/build.yml
- uses: slsa-framework/slsa-github-generator/.github/workflows/generator_generic_slsa3.yml@v1.10.0
  with:
    base64-subjects: ${{ needs.build.outputs.hashes }}
- uses: actions/upload-artifact@v4
  with:
    path: my-component.wasm
- run: |
    cosign attest --predicate provenance.intoto.jsonl \
                  --type slsaprovenance \
                  ghcr.io/foo/bar:0.1.0
```

The result: anyone fetching `ghcr.io/foo/bar:0.1.0` can `cosign verify-attestation --type slsaprovenance --certificate-identity-regexp '^https://github.com/foo/' --certificate-oidc-issuer https://token.actions.githubusercontent.com` and get a cryptographic proof that GitHub Actions built that exact `.wasm` from a specific commit + workflow.

## The full "signed + attested" artifact

A production-grade WASM component artifact in 2026 might carry:

```
ghcr.io/foo/bar:0.1.0  (digest sha256:abc...)
├── Cosign author signature       (subject=abc, type=cosign-bundle)
├── Notation enterprise signature (subject=abc, type=jws)
├── SPDX SBOM                     (subject=abc, type=application/spdx+json)
├── CycloneDX SBOM                (subject=abc, type=application/vnd.cyclonedx+json)
├── SLSA L3 provenance            (subject=abc, type=in-toto+slsaprovenance)
├── Trivy vuln scan attestation   (subject=abc, type=in-toto+vuln)
└── Two-party-review attestation  (subject=abc, type=in-toto+review)
```

All discoverable via one `oras discover ghcr.io/foo/bar:0.1.0` call. **This is the standard the WASM ecosystem is converging on.** Spin Cloud, wasmCloud commercial offerings, and Bytecode Alliance reference flows already produce this shape.

## Implications for Myrhiza

**Don't invent supply-chain primitives.** SBOM + in-toto + SLSA + DSSE are mature and broadly adopted. Compose with them.

**The SLSA L3 floor is achievable for free in GitHub Actions.** Any Myrhiza spec recommending "apps SHOULD be signed" should default to "with SLSA L3 GitHub Actions provenance." The infra cost is zero; the trust dividend is large.

**SBOM for WASM components is an ecosystem gap.** Syft's WASM support is shallow as of mid-2025. Myrhiza spec authors writing about SBOM should reach an "ecosystem isn't there yet for parsing WASM-internal dependencies; SBOM lists source-language deps only" caveat. This is fixable upstream but not Myrhiza's problem to solve.

**Two-signer model is borrowable:** the Sigstore "author signature" + Notation "enterprise signature" pattern (one artifact, two signatures, different trust roots) maps cleanly onto Myrhiza's "user installed an app" + "Myrhiza network admin endorsed it" trust composition.

**The verify-burden is real:** each attestation type needs its own verifier logic. A Myrhiza-app-installer that "checks all the things" easily ends up running 5+ verification calls. Practical advice: bundle verification policy into a single declarative file (Notation's `trustpolicy.json` is a good model) and run it once.

**Air-gapped / offline-first matters.** Some Myrhiza networks may not be able to reach `rekor.sigstore.dev` to verify a Sigstore signature. The signed bundle for an "offline install" needs to embed the Rekor log entry alongside the signature — Cosign 2+ supports this via `cosign sign --bundle bundle.json`. Use that path for Myrhiza-flavored peer-to-peer signature delivery.

## Sources

- SPDX: <https://spdx.dev>
- CycloneDX: <https://cyclonedx.org>
- in-toto: <https://in-toto.io>
- SLSA: <https://slsa.dev>
- slsa-github-generator: <https://github.com/slsa-framework/slsa-github-generator>
- DSSE: <https://github.com/secure-systems-lab/dsse>
- Syft: <https://github.com/anchore/syft>
- Cosign attestations: <https://docs.sigstore.dev/cosign/signing/attestations/>
