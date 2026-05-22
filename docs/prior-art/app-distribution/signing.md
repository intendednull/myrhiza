**Date:** 2026-05-22
**Status:** active
**Subject:** Signed artifact lineage — Sigstore (Cosign + Fulcio + Rekor) and Notary Project (Notation v1/v2). Two converging approaches to OCI-registry-attached signatures.

# Signing artifacts

The WASM ecosystem inherits container-world signing primitives almost wholesale. Two projects matter; both layer signatures over OCI 1.1's Referrers API.

## Sigstore — keyless-by-default OIDC signing

**[Sigstore](https://www.sigstore.dev)** is an OpenSSF-hosted project (Incubating maturity level as of 2026-05) that pioneered "keyless" software signing — using a short-lived certificate bound to the signer's OIDC identity (Google account, GitHub account, etc.) instead of a long-lived private key the signer has to manage.

### Components

| Component | What | Repo |
|---|---|---|
| **Cosign** | Client tool: sign + verify artifacts | [`sigstore/cosign`](https://github.com/sigstore/cosign), Apache-2.0 |
| **Fulcio** | Free, public CA: issues short-lived (~10 min) X.509 certs tied to OIDC identities | [`sigstore/fulcio`](https://github.com/sigstore/fulcio), Apache-2.0 |
| **Rekor** | Public, append-only transparency log: records every signature so the world can audit | [`sigstore/rekor`](https://github.com/sigstore/rekor), Apache-2.0; v2 GA October 2024 |
| **Gitsign** | git commit signing via the same flow | [`sigstore/gitsign`](https://github.com/sigstore/gitsign), Apache-2.0 |
| **Policy Controller** | Kubernetes admission controller enforcing signature policies | [`sigstore/policy-controller`](https://github.com/sigstore/policy-controller), Apache-2.0 |
| **Model Signing** | ML/AI model signing (2025-ish addition) | [`sigstore/model-transparency`](https://github.com/sigstore/model-transparency) |

### Cosign — versions verified 2026-05

- **v3.0.6** (2025-04-06) — current main line. Cosign 3 was the major rewrite that made Referrers the default discovery mode and removed legacy tag-based signature discovery.
- **v2.6.3** (2025-04-06) — current LTS for environments still on Cosign 2.

License: Apache-2.0.

### Sigstore foundation history

- **2021-03-09:** Sigstore announced (Linux Foundation post by Luke Hinds + Red Hat + Google + Chainguard origin team).
- **2021-09-15:** Sigstore moved under the OpenSSF umbrella.
- **2022-10-26:** Cosign + Rekor + Fulcio declared GA at SigstoreCon (Detroit), production readiness milestone. Quoting the CNCF blog announcement: *"the Sigstore APIs are now ready for enterprise use."*
- **2023+:** Adopters multiply. Kubernetes releases (1.24+), Helm charts, npm provenance (May 2023), Homebrew (May 2024), PyPI (November 2024), Maven Central (January 2025).
- **2024-10:** Rekor v2 GA.
- **Maturity level as of 2026-05:** OpenSSF **Incubating** (per OpenSSF project status table); has not yet reached "Graduated" status though discussion is active.

**Anti-pattern to flag:** "Sigstore is OpenSSF-graduated" — common mistake. As of writing it is Incubating. Verify before specifying.

### How Cosign signing flow works (keyless)

```
1. cosign sign ghcr.io/foo/bar:0.1.0
2. Cosign opens a browser → user authenticates with OIDC IdP (GitHub, Google, etc.)
3. Cosign sends an ephemeral keypair's pubkey + OIDC ID-token to Fulcio
4. Fulcio verifies the ID-token, issues a short-lived (10 min) X.509 cert binding
   the pubkey to the OIDC subject
5. Cosign signs the artifact's digest with the ephemeral private key
6. Cosign uploads (signature + cert) as a new manifest in the same OCI registry,
   with `subject` pointing at the artifact's digest
7. Cosign records (signature + cert + log entry) in Rekor's transparency log
8. The ephemeral private key is discarded
```

Verification (`cosign verify`) inverts the chain: fetch the signature manifest via Referrers → verify the signature against the artifact digest → verify the cert chains to Fulcio's CA → verify the cert's OIDC subject matches the expected identity (`--certificate-identity foo@example.com --certificate-oidc-issuer https://accounts.google.com`) → verify Rekor has a log entry corroborating the signature was created in-window.

**Crucial property:** no long-lived private key. The signer cannot lose / leak a signing key, because the signing key didn't outlive the act of signing. The trust root is the OIDC identity provider plus Fulcio's CA root plus Rekor's transparency log.

### Cosign signing flow (keyed)

For air-gapped environments or when OIDC isn't viable:

```
cosign generate-key-pair                  # creates cosign.key + cosign.pub
cosign sign --key cosign.key ghcr.io/foo/bar:0.1.0
cosign verify --key cosign.pub ghcr.io/foo/bar:0.1.0
```

This is structurally the same X.509-meets-OCI shape, just with a long-lived ECDSA/Ed25519 keypair instead of Fulcio-minted ephemeral certs. The signature is still attached via Referrers.

## Notary Project — Notation v1/v2

The **[Notary Project](https://notaryproject.dev)** is the CNCF Incubating successor to the original Docker Notary (v1, deprecated). It standardizes around **Notation** — a CLI and signature-format spec built X.509-PKI-first rather than OIDC-first.

### Versions verified 2026-05

- **Notation v1.3.2** (2025-04-27) — current stable on the v1 line.
- **Notation v2.0.0-alpha.1** (2025-03-13) — preview of v2, which adds *blob signing* (signing arbitrary files, not just registry artifacts) and OCI v1.1 referrers-by-default. v2 is **not yet GA**; v1 is the production target.

License: Apache-2.0. CNCF Incubating maturity (accepted to Incubating 2023).

### Notation vs Cosign — design split

| Axis | Cosign | Notation |
|---|---|---|
| **Default identity model** | OIDC keyless (Fulcio) | X.509 cert / trusted-CA-bundle |
| **Transparency log default** | Required (Rekor) | Optional (no equivalent default) |
| **Signature format** | Sigstore's bundle format (envelope of DSSE + cert chain + log entry) | JWS / COSE Sign1 (RFC 8152) |
| **Plugin model** | First-class; HSM signers, KMS plugins, etc. | First-class; vendor signing plugins (AWS, Azure, Notary plugin protocol) |
| **Trust policy** | CLI flags or env vars | Declarative `trustpolicy.json` (per-registry-pattern rules) |
| **OCI attach mechanism** | Referrers API (since Cosign 3) | Referrers API (since Notation v0.13) |
| **Enterprise on-ramp** | Mixed; Chainguard / Sigstore.dev hosted | Strong; AWS Signer, Azure Trust Signing, vendor-CA-friendly |

Practically: **Cosign for the "ship to open-source registries, trust GitHub OIDC" path. Notation for the "ship inside a regulated enterprise, trust the corporate CA" path.** Both produce signatures attached to the same OCI artifact via the same Referrers mechanism, so a single artifact can carry both signatures — and verifiers pick whichever they trust.

### Notation signing flow

```bash
notation cert generate-test --default "myorg.example"
notation sign ghcr.io/foo/bar:0.1.0
notation verify ghcr.io/foo/bar:0.1.0
```

`notation sign` reads `~/.config/notation/signingkeys.json` for the active key, produces a JWS-formatted signature, and uploads it as an OCI manifest with `subject` pointing at the artifact. `notation verify` reads `trustpolicy.json` — a declarative document like:

```json
{
  "trustPolicies": [
    {
      "name": "default",
      "registryScopes": ["ghcr.io/foo/*"],
      "signatureVerification": {"level": "strict"},
      "trustStores": ["ca:myorg-ca"],
      "trustedIdentities": ["x509.subject:CN=Foo Inc,O=Foo"]
    }
  ]
}
```

The declarative-policy approach is the biggest pragmatic difference from Cosign. Notation refuses to verify without a policy file matching the registry — this is intentional, to prevent "verify-against-anything" misconfigurations.

## What gets signed

Three orthogonal things can be signed, attached to the same artifact via Referrers:

1. **The artifact itself** — "I, identity X, attest this `.wasm` is what I built / endorsed."
2. **A Software Bill of Materials (SBOM)** — "I, identity X, attest this is the SBOM of artifact Y." See [`supply-chain.md`](./supply-chain.md).
3. **An attestation** (in-toto, SLSA-provenance) — "I, identity X, attest the build process for artifact Y was {build-system, inputs, parameters}." See [`supply-chain.md`](./supply-chain.md).

A "fully signed" component artifact may have 3+ Referrers attached, each from a different signer at a different step in the supply chain.

## Threat model — what signatures do and don't defend against

**Defends:**
- Tampered binary in transit or in registry. The signature catches the digest mismatch.
- Compromised registry account pushing a malicious artifact under a known tag. The signature won't verify against the expected signer identity.
- Supply-chain mole replacing a published build. Rekor's transparency log makes "silent re-sign" detectable.

**Doesn't defend:**
- Compromised signer at the time of signing. If the signer's GitHub account is owned, the signed artifact can be malicious-but-signed-by-the-real-owner. (Mitigated by limiting signer scope + monitoring identity provider audit logs.)
- Sigstore root-of-trust compromise. If Fulcio's root CA private key leaks, all keyless signatures become unverifiable / spoofable. (Mitigated by Fulcio's HSM-backed root + log-witness redundancy.)
- Registry availability. If the registry is offline, you can't fetch the signature to verify. (Mitigated by mirroring or by `cosign verify --offline` against a local copy.)
- Determinism / behavior of the signed code. Signing says "this is the build that was attested." It says nothing about whether the build is correct, safe, or deterministic.

## Implications for Myrhiza

**The right primary path:** Cosign-keyless against GHCR. Free, public, transparent, no key custody burden, OIDC against GitHub / Google / GitLab. This is what Myrhiza apps should target as the default sign-on-publish UX.

**The regulated-deployment path:** Notation. If a corporate Myrhiza deployment needs vendor-CA-anchored trust, Notation is the answer. Both signatures can coexist on the same artifact via Referrers; a Myrhiza host can verify whichever matches its configured trust roots.

**The P2P-mismatch:** Both Sigstore and Notation assume an OCI registry sits between signer and verifier. Myrhiza's P2P story doesn't. If two Myrhiza peers exchange an app over iroh blob transport, the signature needs to travel too — packaged in the blob, or fetched via a separate OCI lookup. This is unsolved ecosystem work; see [`open-problems.md`](./open-problems.md) §p2p-signature-transport.

**Don't roll our own:** the temptation to invent a Myrhiza-native bundle-signing scheme (e.g. Holochain-style author-key signing baked into a YAML manifest) should be resisted. The Sigstore + Notation lineage has more eyeballs, more rotation primitives, more revocation thinking. Compose with it; don't replace it.

**Verify-policy is load-bearing:** even with great signing tools, the *verification* side is where most production deployments mess up — `cosign verify` without `--certificate-identity` proves nothing. Myrhiza's app-install flow needs an opinionated "what does this signature mean" model from day one. See [`lessons.md`](./lessons.md) §borrow-notation-trustpolicy.

**Trust-root bootstrapping is the hard part:** for the OIDC-keyless flow to work, peers need a shared notion of "what's a trusted OIDC issuer." For the X.509 flow, they need a shared CA bundle. Myrhiza is a P2P network — there's no central admin to push a `trustpolicy.json`. This is the same problem Holochain's "Verified" badge tried and failed to solve manually. See [`open-problems.md`](./open-problems.md) §trust-roots.

## Sources

- Sigstore project: <https://www.sigstore.dev>, <https://docs.sigstore.dev/about/overview/>
- Sigstore GA announcement (2022-10-26): CNCF blog and SigstoreCon proceedings
- Cosign: <https://github.com/sigstore/cosign>
- Fulcio: <https://github.com/sigstore/fulcio>
- Rekor: <https://github.com/sigstore/rekor>
- Notary Project: <https://notaryproject.dev>
- Notation: <https://github.com/notaryproject/notation>
- Notation spec: <https://github.com/notaryproject/specifications>
- in-toto: <https://in-toto.io>
- SLSA: <https://slsa.dev>
- OpenSSF project maturity: <https://openssf.org/projects/>
