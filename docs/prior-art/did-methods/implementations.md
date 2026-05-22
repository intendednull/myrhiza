**Date:** 2026-05-22
**Status:** active
**Subject:** Implementation libraries — Spruce `ssi-rs` + `didkit` (archived), Veramo (JS), DIF universal-resolver, libp2p PeerID (related but not a DID method).

# DID implementations — libraries + tooling

Four implementation stacks matter for evaluating "could Myrhiza use this":

1. **Spruce `ssi`** (Rust) — the lowest-level, broadest-DID-method, actively-maintained Rust crate.
2. **Veramo** (JS/TS) — the modular framework for Node + browser + React Native.
3. **DIF universal-resolver** — the polyglot Docker-based "resolve any DID method" service.
4. **libp2p PeerID** — not a DID method, but a related identity primitive Myrhiza already uses.

## Spruce — `ssi` crate (Rust)

**Repository:** [spruceid/ssi](https://github.com/spruceid/ssi). **Current version:** v0.16.0 (published 2026-04-16). **License:** Apache-2.0. **Downloads:** 160k+ total, 18.6k recent. **Status:** actively maintained.

The `ssi` crate is a workspace covering DID resolution, Verifiable Credentials, JWS/JWT/JWE/COSE, and key types. Workspace crates include `ssi-dids`, `ssi-jwk`, `ssi-claims`, `ssi-jws`, `ssi-crypto`, `ssi-caips` (chain-agnostic standards), `ssi-bbs` (BBS signatures), and feature-gated method drivers.

**DID method drivers** (feature-flagged):

- `did:key` (always available)
- `did:web` (`http-did` feature)
- `did:pkh` (public-key-hash, like crypto wallet addresses)
- `did:ethr` (`ethereum` feature)
- `did:jwk` (JSON Web Key embedded in identifier)
- Plus support for `did:tezos`, `did:tz`, and various Aleo / blockchain methods.

**Key types supported:** Ed25519, secp256k1, P-256, P-384 ("secp256r1"), RSA, BLS12-381 / BBS, ed25519-dalek for native Rust. Feature-gated so a minimal build is small.

**Architecture:** Modular — pull in only the workspace crates you need. The `ssi` umbrella crate re-exports common types. Used in production by Spruce's enterprise identity products (SpruceID Wallet, Sign-In With Ethereum, etc.).

**Pros for Myrhiza:**

- Rust-native. Compiles to WASM-component-friendly targets.
- Apache-2.0, no copyleft drama.
- Actively maintained (`v0.16.0` released the month before this corpus was written).
- Workspace structure means we can vendor `ssi-jwk` + `ssi-dids` without dragging in the full Verifiable Credentials machinery.

**Cons for Myrhiza:**

- The crate is *not* `no_std`. Some sub-crates may work; the umbrella does not.
- Heavy dependency surface (~50+ transitive crates with all features on). The `default` feature pulls in `w3c`, `rsa`, `ed25519`, `secp256k1`, `secp256r1`, `ripemd-160`, `eip712`.
- The version number (`0.16.0`) signals pre-1.0 instability. Breaking changes have happened on minor-version bumps.
- Originally consumed via `didkit` (archived July 2025) — see below.

**Don't use `didkit`.** Spruce archived [`spruceid/didkit`](https://github.com/spruceid/didkit) on 2025-07-10. The repository is read-only. Spruce's [statement](https://github.com/spruceid/didkit): "As we do not use the DIDKit bindings internally anymore, we have decided to archive their respective repositories." They redirect users to:

- The `ssi` crate directly (Rust users).
- The new `sprucekit-mobile` library (iOS/Android).

The `didkit` crate on crates.io shows last release v0.6.0 (2023-06-30), confirming the archival. **If anything cites "didkit" as the recommended Spruce entry point, that citation is out of date.**

## Veramo — JS/TS framework

**Repository:** [decentralized-identity/veramo](https://github.com/decentralized-identity/veramo). **Current version:** `@veramo/core` v7.0.0 (published 2026-02-11). **License:** Apache-2.0. **Status:** actively maintained. **Maintainer:** Consensys Mesh R&D (Mircea Nistor + team); previously uPort.

Veramo is a plugin-host framework: you build an `Agent` from a set of plugins, each plugin exposing operations on a `IPluginMethodMap`. Core plugins:

- `@veramo/did-manager` — DID lifecycle.
- `@veramo/did-resolver` — uses `did-resolver` (TypeScript) under the hood.
- `@veramo/did-provider-ethr`, `did-provider-web`, `did-provider-key`, `did-provider-peer` — per-method providers.
- `@veramo/credential-w3c` — VC issuance + verification.
- `@veramo/key-manager`, `@veramo/kms-local`, `@veramo/kms-web3` — key custody backends.
- `@veramo/data-store`, `@veramo/data-store-json` — persistence.

**Platform support:** Node.js, browser (via webpack/rollup), React Native (with shims).

**DID method coverage:** `did:ethr` is the flagship (Mircea Nistor also maintains `ethr-did-resolver`). `did:web`, `did:key`, `did:peer` are first-class. `did:pkh`, `did:jwk` via plugins. `did:plc` resolution exists via community plugin.

**Pros for Myrhiza (if Myrhiza needs a JS-side surface — e.g. for jco browser embedding):**

- Apache-2.0.
- Modular: pull in only the plugins needed.
- Active (v7.0.0 released February 2026, three months before this corpus).
- The original uPort team's accumulated DID experience is in this codebase.

**Cons:**

- TS/JS only — needs a host with Node or a browser. For a Rust/WASM-component runtime like Myrhiza, this is a JS-side library at best.
- Plugin system is heavy — a minimal agent still pulls in `@veramo/core` + `@veramo/key-manager` + a KMS + a method provider. Tree-shaking helps but doesn't eliminate.
- Lots of transitive npm dependencies (typical JS ecosystem).

**For Myrhiza:** Veramo is the right reference if Myrhiza ever wants to ship a `did:web` representation of a peer-author readable by an external JS client. Not a primary identity library.

## DIF universal-resolver

**Repository:** [decentralized-identity/universal-resolver](https://github.com/decentralized-identity/universal-resolver). **License:** Apache-2.0. **Last tagged release:** v0.5.0 (2022-01-07). **Commits ongoing:** 2,085 total. **Status:** actively maintained despite stale tag.

The universal-resolver is a Docker-Compose-orchestrated set of **~70+ DID method drivers**. Each driver is a separate container exposing a uniform HTTP API (`GET /1.0/identifiers/<did>` → DID document + metadata). Drivers include all methods covered in [`methods.md`](methods.md) plus dozens of long-tail methods (did:btcr, did:sol, did:near, did:hedera, did:dns, did:webplus, did:ebsi, did:prism, did:iden3, did:kilt, did:dock, etc.).

**Operational model:** DIF (Decentralized Identity Foundation) hosts a public reference instance at [dev.uniresolver.io](https://dev.uniresolver.io); organizations can also self-host.

**Pros:**

- The de-facto "any DID" resolution surface for the ecosystem.
- Mature: ~8 years of contributions.
- Docker-native, polyglot drivers (Java, Node, Go, Rust, Python all present).

**Cons:**

- Operational complexity — ~70 containers to keep healthy.
- Performance: cold-start latency on a Docker driver is multi-second.
- The public reference instance has historical reliability issues — DIF operates it best-effort, not as production infrastructure.
- The v0.5.0 tag from 2022 is misleading; drivers update independently with no aggregate version coordination.

**For Myrhiza:** Universal-resolver is what an *external service* would use to resolve a Myrhiza-issued `did:web` (or future `did:myrhiza`). Myrhiza itself shouldn't run a universal-resolver — too much operational overhead for a peer process. The takeaway is that *if* Myrhiza ever publishes its identity in DID format, building a universal-resolver driver is a known, well-trodden path.

## libp2p PeerID — related but not a DID method

**Spec:** [libp2p/specs/peer-ids](https://github.com/libp2p/specs/blob/master/peer-ids/peer-ids.md). **NOT** a DID method.

A libp2p PeerID is a multihash of an encoded public key:

- For keys ≤ 42 bytes (i.e. Ed25519 keys, secp256k1 keys), the multihash uses the `identity` codec — the PeerID literally contains the public key bytes.
- For longer keys (RSA), the PeerID is the SHA-256 of the encoded key.

Mandatory key support: **Ed25519**. Recommended: RSA (for DHT interop). Optional: secp256k1, ECDSA.

Text representation: legacy base58btc multihash, modern multibase-encoded CIDv1 with `libp2p-key` multicodec.

**Relationship to DIDs:**

- A PeerID *could* be wrapped as a `did:key` — for Ed25519/secp256k1 the encoding is structurally similar (multibase + multicodec + raw public key).
- iroh's NodeID is functionally a PeerID-shaped Ed25519 identifier ([`prior-art/iroh/identity.md`](../iroh/identity.md) covers this in depth).
- The libp2p PeerID is NOT registered in the DID Methods registry. There is no `did:libp2p` or `did:peerid`.

**Relevance to Myrhiza:** Myrhiza's `PeerKeypair` ([`crates/kernel/src/identity.rs`](../../../crates/kernel/src/identity.rs)) is Ed25519, exactly the same primitive a libp2p PeerID wraps. Myrhiza's bech32m encoding of the public key (`wpeer1...` HRP per master spec) is *another* encoding of the same primitive — interconvertible with libp2p PeerID, with `did:key`, and with raw 32-byte bytes. Choosing between them is a UX/wire-format decision, not a cryptographic one.

## Implementation choice matrix for Myrhiza

| Need | Library |
|---|---|
| Native Rust DID method resolution (e.g. resolve a `did:web` for an external entity) | `ssi` crate (specifically `ssi-dids` + `ssi-jwk`) |
| Emit a `did:key` or `did:web` representation of a Myrhiza author key | `ssi` crate (or hand-roll the multicodec encoding — it's ~30 lines) |
| Resolve any of the long-tail DIDs | DIF universal-resolver via HTTP (don't vendor it) |
| JS-side interop in a jco-browser deployment | Veramo (specifically `@veramo/did-resolver` + the `did-resolver` underlying TS package) |
| Pure peer identity (no DID conformance) | Myrhiza's own bech32m + Ed25519 (already in `crates/kernel/src/identity.rs`); equivalent to libp2p PeerID |

**The honest answer:** for Plan B-2, Myrhiza doesn't *need* any of these libraries. The cryptography is `ed25519-dalek` (already in use). The encoding is bech32m (already in use). External interop via `did:key`/`did:web` is a *future* feature that adds <100 lines of multicodec encoding when needed. **The DID library ecosystem is a thing to know about, not a thing to integrate.**

## Sources

- Spruce `ssi` crate — <https://crates.io/crates/ssi> (v0.16.0, 2026-04-16).
- Spruce `ssi` repository — <https://github.com/spruceid/ssi>.
- Spruce `didkit` (ARCHIVED 2025-07-10) — <https://github.com/spruceid/didkit>.
- Veramo repository — <https://github.com/decentralized-identity/veramo>.
- `@veramo/core` on npm — <https://registry.npmjs.org/@veramo/core/latest> (v7.0.0, 2026-02-11).
- DIF universal-resolver — <https://github.com/decentralized-identity/universal-resolver>.
- DIF universal-resolver hosted instance — <https://dev.uniresolver.io>.
- libp2p PeerID spec — <https://github.com/libp2p/specs/blob/master/peer-ids/peer-ids.md>.
- iroh NodeID in-tree — [`prior-art/iroh/identity.md`](../iroh/identity.md).
- Myrhiza identity module — [`crates/kernel/src/identity.rs`](../../../crates/kernel/src/identity.rs).
