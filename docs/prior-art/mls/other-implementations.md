**Date:** 2026-05-09
**Status:** active
**Subject:** Non-OpenMLS implementations of RFC 9420 — survey of mlspp, mls-rs, Wire's stack, libcrux, and others

> Companion files: [`openmls.md`](./openmls.md) — Rust deep-dive (load-bearing if Myrhiza adopts MLS); [`protocol.md`](./protocol.md) — RFC 9420 walkthrough; [`group-lifecycle.md`](./group-lifecycle.md), [`production-users.md`](./production-users.md), [`comparisons.md`](./comparisons.md), [`lessons.md`](./lessons.md), [`glossary.md`](./glossary.md).

This file surveys the major MLS implementations that are *not* OpenMLS — what shipped, who maintains it, what license it uses, and which production deployments depend on it. The honest production-vs-research distinction matters: only a handful of these have shipped to end users.

## 1. mlspp — Cisco / Webex (C++ reference)

| Field | Value |
|---|---|
| Repository | [github.com/cisco/mlspp](https://github.com/cisco/mlspp) |
| Language | C++17 |
| License | **BSD-2-Clause** |
| Stars / forks | 140 / 50 |
| Created | 2018-02-16 (predates RFC by 5+ years; tracked the protocol from -draft-09 onward) |
| Last push | 2026-04-13 (active) |
| Crypto | OpenSSL 1.1.1+, OpenSSL 3.0, or BoringSSL |
| Driving authors | Richard Barnes (RFC 9420 lead author, Cisco), Tommy Pauly, J. Mattsson |

mlspp is the de-facto **C++ reference implementation** maintained by Cisco. It is the second leg of the IETF interop runner (OpenMLS pairs against mlspp in CI). It powers **Cisco Webex's Zero-Trust E2EE** for meetings and content (chat, files, whiteboards, annotations) — Cisco started shipping a draft-version MLS in production years before RFC 9420 ratification and migrated to the standardized version after publication. Cisco's white papers describe MLS as the per-meeting key-agreement layer underneath SFrame for media encryption.

For Myrhiza, mlspp is mostly relevant as the **interop counterpart**: any Rust-side MLS deployment that needs cross-implementation guarantees runs against mlspp. It would not be embedded directly in a WASM Component Model runtime — the C++ surface plus OpenSSL is the wrong shape for that.

## 2. mls-rs — AWS Labs (Rust, BYOC)

| Field | Value |
|---|---|
| Repository | [github.com/awslabs/mls-rs](https://github.com/awslabs/mls-rs) |
| Crate | `mls-rs 0.55.0` (May 2026) |
| Language | Rust (edition 2021, MSRV 1.82) |
| License | **Apache-2.0 OR MIT** |
| Stars / forks | 228 / 48 |
| Created | 2023-11-06 (formerly `aws-mls`) |
| Top contributors | `mulmarta` (249 commits), `stefunctional` (154), `mgeisler` (50, Google), `albe-rosado` (21), `beltram` (Wire) |
| Crypto providers | OpenSSL, AWS-LC, RustCrypto, Web Crypto, **Apple CryptoKit** |

A second serious Rust impl, AWS-stewarded, with a more aggressive multi-backend posture than OpenMLS — five crypto providers in-tree, including **CryptoKit** (Apple-platform native), **AWS-LC** (Amazon's BoringSSL fork), and **Web Crypto** (browser-native). `no_std` is a stated category. Wire reportedly contributes back to mls-rs alongside its own stack; Wire engineer `beltram` shows up in both repos.

WASM posture: README claims "Support for WASM builds." A community wrapper `@river-build/mls-rs-wasm` exists on npm. A pinned issue (#77, "Browser support roadmap", opened Feb 2024) tracks remaining browser-side compatibility — tests pass on Chrome but fail on Firefox 122 and Safari 17 at the time of opening. Like OpenMLS, no WIT/Component Model artifact ships in-tree.

For Myrhiza, mls-rs is the **plausible alternative** to OpenMLS. Trade-offs:

- *Pro:* dual MIT/Apache-2.0 makes downstream relicensing easier than OpenMLS's MIT-only; richer crypto-backend matrix.
- *Pro:* no_std support is a stated goal — possibly easier to fit a constrained WASM target.
- *Con:* less direct involvement from RFC 9420 authors (no Barnes / Robert / Beurdouche commits); single-org primary stewardship (AWS) vs. OpenMLS's two-org base.
- *Con:* fewer external production users publicly disclosed.

## 3. Phoenix R&D — Phoenix homeserver and OpenMLS

Phoenix R&D (Raphael Robert, RFC 9420 co-author; Konrad Kohbrok; Marcel Keller; Damian Poddebniak) **does not ship a competing MLS engine** — they ship OpenMLS. Their commercial / product layer is the **Phoenix homeserver**, a federated MLS-based messaging service that uses OpenMLS as its protocol core and contributes upstream as the primary protocol vehicle. Phoenix R&D co-initiated the IETF **MIMI** (More Instant Messaging Interoperability) working group; their architecture is MIMI-aligned (HTTPS+MLS transport).

Funding signal: Open Technology Fund supports "Project Phoenix" as targeted at at-risk users. Public blog posts on "Making MLS more decentralized" and the OpenMLS validation tracker indicate sustained engineering investment.

For Myrhiza, the relevance is: **the Rust MLS implementation Myrhiza is most likely to adopt is also the protocol-author's own implementation**. That is unusually good alignment.

## 4. Wire — production MLS (Kotlin / Rust core via mls-rs)

Wire began deploying **draft-MLS** to early users in **2023** — *before* RFC 9420 was ratified, using a near-final draft of the protocol — and reached **RFC 9420 GA in April 2025** (see [production-users.md](production-users.md) for the GA date primary source). Wire's server (Haskell, **AGPL-3.0**, [github.com/wireapp/wire-server](https://github.com/wireapp/wire-server), 2.7k stars) and clients embed MLS for group messaging. Wire engineers contribute to both OpenMLS (`beltram`, 17 commits) and mls-rs (`beltram` again).

Wire's deployment is significant for two reasons:

1. It was the **first end-user-shipped MLS deployment** of any meaningful scale (Webex predates it on a closed/SaaS path; Wire was the first open-client one).
2. The server license is **AGPL-3.0** — a meaningful licensing posture for any project that wants to study the architecture without entanglement.

The Wire client SDK historically used a Rust core (CoreCrypto) wrapping mls-rs; Wire has published their MLS migration retrospective (worth reading for any project planning their own MLS rollout — see `lessons.md`).

## 5. Apple — RCS Universal Profile 3.0 (announced 2025)

Apple does **not** use MLS for iMessage internally — iMessage uses Apple's own protocol stack (PQ3, post-quantum since iOS 17.4, Feb 2024). The MLS connection at Apple is via **RCS Universal Profile 3.0**, the GSMA standard announced March 2025 for cross-platform (Android↔iOS) RCS encryption, which mandates MLS as the group-key-agreement protocol. Apple announced support for UP 3.0 at announcement time. As of May 2026, the rollout to Apple Messages is in limited deployment, with both Apple and Google testing the cross-platform path.

Apple has not open-sourced its MLS implementation, but mls-rs's `mls-rs-crypto-cryptokit` provider is positioned squarely at Apple-platform consumers, and CryptoKit is the natural primitive layer for any Apple-side adopter.

## 6. Google — RCS / Google Messages

Google Messages is **adding MLS** for RCS as part of UP 3.0. Reverse-engineering of the Google Messages app from late 2024 surfaced MLS-related strings ahead of the formal GSMA announcement. As of early-to-mid 2026 the rollout is partial. Google has not publicly named a specific in-tree MLS implementation; given Google's history with [tink](https://github.com/google/tink) and Project Wycheproof, an internal C++/Java implementation is plausible but not confirmed.

This deployment, when complete, makes MLS the encryption layer for **hundreds of millions of mobile devices** ([IETF post: "MLS set be used on 100s of millions of mobile devices"](https://www.ietf.org/blog/rcs-adopts-mls/)).

## 7. libcrux + hax — formally-verified primitives (Cryspen)

[github.com/cryspen/libcrux](https://github.com/cryspen/libcrux) (Apache-2.0, 225 stars) is **not an MLS implementation** but is the formally-verified crypto library underneath OpenMLS's `libcrux-provider`. It is a hybrid: formally-verified Rust generated from the HACL\* project (Inria/Cryspen, originally F\*) plus additional Rust code verified directly via Cryspen's `hax` toolchain (see [governance.md](governance.md) for the canonical description). The MLS-relevant primitives — Ed25519, X25519, P-256, AES-GCM, ChaCha20-Poly1305, HKDF, HMAC, SHA-2 — are progressively being verified for memory safety, panic freedom, and cryptographic correctness.

Karthikeyan Bhargavan's group at Cryspen also published the **MLS\*** project — an F\*-formalized MLS stack used as a reference for the IETF spec analysis. Not a deployable library; a formal-methods artifact.

For Myrhiza: if the project ever wants the strongest possible "this crypto is correct" story under the runtime, the path is OpenMLS (or mls-rs) compiled against libcrux. There is no other formally-verified crypto provider in the MLS ecosystem.

## 8. Other implementations on the IETF list

From [mlswg/mls-implementations](https://github.com/mlswg/mls-implementations):

| Project | Language | Status | Notes |
|---|---|---|---|
| **mls-kotlin** (Wire) | Kotlin | Active | Wire client-side, JVM target |
| **BouncyCastle MLS** | Java | Active | Folded into BC's broader crypto bundle |
| **ts-mls** | TypeScript | Active | Browser/JS-native impl |
| **mls-go** / **go-mls** | Go | In-progress / partial | Multiple competing Go forks |
| **rmls** ([webrtc-rs/rmls](https://github.com/webrtc-rs/rmls)) | Rust | Research/abandoned-leaning | Third Rust impl; far less activity than OpenMLS or mls-rs |
| **MLS\*** | F\* | Formal verification | Not a deployment artifact |
| **mls_stuff** | Python | In-progress | Educational |
| **RingCentral proprietary** | C++ | In-progress | Private |

## 9. Comparison table

| Implementation | Language | License | Production-shipping | RFC 9420 conformance | Notable users / role |
|---|---|---|---|---|---|
| **OpenMLS** | Rust | MIT | Yes (Phoenix homeserver, Wire client core) | Yes; tracks unreleased extensions | Phoenix R&D, Cryspen, OTF |
| **mlspp** | C++17 | BSD-2-Clause | Yes (Cisco Webex) | Yes; reference impl with OpenMLS in IETF interop | Cisco Webex, Webex Meetings |
| **mls-rs** | Rust | Apache-2.0 / MIT | Yes (Wire client; AWS internal use) | Yes; broad crypto matrix | AWS, Wire (contributing) |
| **mls-kotlin** | Kotlin | Wire's mix | Yes (Wire client) | Yes | Wire Android client |
| **CoreCrypto / Wire client** | Rust (over mls-rs) | AGPL/proprietary mix | Yes (Wire) | Yes | Wire native clients |
| **BouncyCastle MLS** | Java | MIT-style (BC) | Niche / library | Yes | JVM ecosystem |
| **ts-mls** | TypeScript | MIT | Limited | Partial | Browser experiments |
| **rmls** (webrtc-rs) | Rust | Apache-2.0 | No | Partial | Research/incomplete |
| **MLS\*** | F\* | — | No (research) | Specification-level | Cryspen formal verification |
| **libcrux** | Rust | Apache-2.0 | Used as crypto provider only | n/a (not MLS) | Underlies OpenMLS libcrux-provider |
| **iMessage (Apple)** | Closed | Proprietary | Yes (PQ3, **not MLS**) | n/a | iMessage itself is *not* MLS |
| **Apple RCS UP 3.0** | Closed | Proprietary | In rollout | Yes | Apple Messages cross-platform RCS |
| **Google Messages RCS UP 3.0** | Closed | Proprietary | In rollout | Yes | Google Messages cross-platform RCS |

## 10. Implications for Myrhiza

- **Embeddable MLS implementations that target a Rust/WASM Component Model story are exactly two: OpenMLS and mls-rs.** Everything else is either C++ (mlspp), JVM (mls-kotlin, BouncyCastle), browser-only (ts-mls), proprietary (Apple/Google), or research/incomplete (rmls, MLS\*).
- **License posture is permissive across the Rust options.** OpenMLS = MIT only; mls-rs = Apache-2.0 OR MIT (more flexible for downstream relicensing or dual-license schemes).
- **Production validation is real but concentrated.** Webex (mlspp) and Wire (OpenMLS / mls-rs hybrid via CoreCrypto) are the meaningful long-running deployments; RCS UP 3.0 (Apple + Google) is the impending volume play. There is *no* P2P deployment in the public list — every shipping MLS user runs a centralized Delivery Service.
- **For a P2P WASM runtime, the alternative-implementation surface is decorative — the choice is OpenMLS vs. mls-rs**, and OpenMLS leads on stewardship overlap with the RFC authors and on formal-verification-via-libcrux. mls-rs leads on backend variety and license flexibility.

## 11. Sources

- [github.com/cisco/mlspp](https://github.com/cisco/mlspp) (BSD-2-Clause, 140 stars) — `gh api repos/cisco/mlspp`
- [github.com/awslabs/mls-rs](https://github.com/awslabs/mls-rs) (Apache-2.0/MIT, 228 stars) — `gh api repos/awslabs/mls-rs`, workspace `Cargo.toml`, README
- [github.com/wireapp/wire-server](https://github.com/wireapp/wire-server) (AGPL-3.0, 2.7k stars)
- [github.com/cryspen/libcrux](https://github.com/cryspen/libcrux) (Apache-2.0, 225 stars)
- [github.com/mlswg/mls-implementations](https://github.com/mlswg/mls-implementations) — implementation list
- [Phoenix R&D — phnx.im](https://phnx.im/technologies), [phnx.im/openmls](https://phnx.im/openmls), [blog.phnx.im](https://blog.phnx.im/)
- [Cryspen — cryspen.com](https://cryspen.com/openmls/)
- [Webex blog: How MLS Enables Scalable End-to-End Security in Webex](https://blog.webex.com/collaboration/hybrid-work/scalable-end-to-end-security-in-webex/)
- [Cisco Zero-Trust Security for Webex white paper](https://www.cisco.com/c/en/us/solutions/collateral/collaboration/white-paper-c11-744553.html)
- [Wire — Messaging Layer Security explained](https://wire.com/en/blog/messaging-layer-security-mls-explained)
- [IETF blog: Support for MLS (2023)](https://www.ietf.org/blog/support-for-mls-2023/), [IETF blog: RCS Adopts MLS](https://www.ietf.org/blog/rcs-adopts-mls/)
- [GSMA / RCS Universal Profile 3.0 announcement (March 2025)](https://thehackernews.com/2025/03/gsma-confirms-end-to-end-encryption-for.html)
- [Apple PQ3 protocol writeup (PQShield)](https://pqshield.com/post-quantum-messaging-examining-apples-new-pq3-protocol/)
- [mls-rs browser support roadmap, issue #77](https://github.com/awslabs/mls-rs/issues/77)
- [@river-build/mls-rs-wasm on npm](https://www.npmjs.com/package/@river-build/mls-rs-wasm)
- RFC 9420 (July 2023), RFC 9750 (MLS Architecture)
