**Date:** 2026-05-29
**Status:** active
**Subject:** System-specific terms used across this folder — TUF roles, attack classes, threshold-signing, and transparency-log vocabulary.

# Glossary

Terms specific to update-framework trust models. General Myrhiza terms (state-apply, capability, iroh-gossip) live in the master spec.

## TUF roles and metadata

- **Root role / `root.json`** — the offline trust anchor; signs which public keys are authoritative for every role (including itself). The one document a client must obtain out-of-band once. ([`tuf-roles-and-metadata.md`](./tuf-roles-and-metadata.md))
- **Targets role / `targets.json`** — authoritative list of installable files with their hashes and sizes; may delegate subtrees of the namespace.
- **Delegated targets** — a subordinate role signing a glob-scoped subset of the target namespace (e.g. one project's packages). Basis of PyPI PEP 480 developer signing.
- **Snapshot role / `snapshot.json`** — pins the version numbers of all targets-metadata into one coherent set; defeats mix-and-match.
- **Timestamp role / `timestamp.json`** — short-lived, online-key-signed pointer to the current snapshot; the freshness beacon that defeats freeze.
- **Root chaining** — accepting a new `root.json` because it is signed by the *previous* root's key threshold, enabling in-band root rotation with no re-bootstrap.
- **Online key / offline key** — online keys re-sign frequently (timestamp) and are most exposed but assert the least; offline keys (root, targets) are cold-stored and assert the most.

## Threshold signing

- **Threshold / M-of-N** — require M of N keys before metadata is trusted; survives loss of up to M−1 keys.
- **N-independent-signatures threshold** — TUF's form: M *separate* signatures on one document, all checked by the verifier.
- **FROST** — Flexible Round-Optimized Schnorr Threshold; M-of-N signers produce **one** ordinary Schnorr signature against one group key. RFC 9591. ([`frost-threshold-signing.md`](./frost-threshold-signing.md))
- **DKG (Distributed Key Generation)** — protocol by which N parties jointly create a shared key without any single party ever holding the whole secret.
- **Ciphersuite** — for FROST, the (group, hash) pair instantiating the scheme, e.g. FROST(Ed25519, SHA-512).
- **Standards-Track vs Informational RFC** — Standards-Track is IETF rough-consensus standardization; Informational (RFC 9591's status, via the IRTF CFRG research stream) documents without conferring standard status.

## Attack classes (the taxonomy)

- **Rollback** — serving an older validly-signed view to reintroduce a vuln or hide a fix. ([`tuf-attack-taxonomy.md`](./tuf-attack-taxonomy.md))
- **Fast-forward** — inflating a version number so far that later legitimate versions are rejected as rollbacks, bricking the channel.
- **Freeze / indefinite freeze** — withholding all newer views so the client is stuck at a stale version forever.
- **Endless data** — answering a download with an infinite byte stream to exhaust disk/memory.
- **Mix-and-match** — combining metadata/targets that never coexisted on the repository.
- **Arbitrary software installation** — passing off a malicious file as legitimate (the base case).
- **Malicious mirror** — a mirror inside the distribution fabric blocking/degrading to force a freeze or rollback.

## Uptane

- **Image Repository** — Uptane's slow, offline-signed store: "what firmware exists and is authentic."
- **Director Repository** — Uptane's fast, online store: "what this specific vehicle should install now," constrained to images the Image Repo authenticated.
- **Primary / Secondary ECU** — network-connected ECU that serves constrained ECUs; secondaries do full or partial verification.
- **Full vs partial verification** — a capable verifier checks the complete metadata set; a constrained one checks a reduced set, trusting its primary for the rest.

## Provenance and transparency

- **in-toto attestation** — a signed statement binding an artifact (subject) to a typed claim (predicate). ([`in-toto-slsa-provenance.md`](./in-toto-slsa-provenance.md))
- **DSSE (Dead Simple Signing Envelope)** — the envelope wrapping attestations for signing.
- **SLSA** — Supply-chain Levels for Software Artifacts; build-track levels L0–L3 grading build-process trustworthiness.
- **Transparency log** — append-only, Merkle-backed, publicly auditable record (of certs, signatures, or hashes) where the operator cannot rewrite history undetected. ([`transparency-logs.md`](./transparency-logs.md))
- **Certificate Transparency (CT)** — the origin transparency-log deployment, for TLS certificate issuance.
- **Rekor** — Sigstore's transparency log for signatures.
- **sumdb** — the Go toolchain's tile-based transparency log for module checksums (`sum.golang.org`).
- **Trillian / Tessera / Sunlight** — verifiable-log backends; the latter two are the newer tile-based / static-file lines.
- **Tile-based log** — a Merkle tree split into tiles (many hashes each) served as static files, making the read path a filesystem + cache.
- **Witness** — an independent party that observes and cross-signs a log's head so clients can detect a forked/rewritten log.

## Reproducible builds

- **Reproducible build** — same source under same declared conditions yields **bit-for-bit identical** output. ([`reproducible-builds.md`](./reproducible-builds.md))
- **`SOURCE_DATE_EPOCH`** — env var pinning embedded timestamps so builds are timestamp-deterministic.
- **rebuilderd / reproduce.debian.net** — infrastructure that independently rebuilds packages and compares hashes.
- **diffoscope** — tool that explains, byte region by byte region, why two builds differ.

## Sources

- Terms defined across sibling files; see each file's own Sources section.
- TUF spec: <https://theupdateframework.github.io/specification/latest/>
- RFC 9591: <https://www.rfc-editor.org/rfc/rfc9591.html>
- Uptane Standard: <https://uptane.org/papers/ieee-isto-6100.1.0.0.uptane-standard.html>
