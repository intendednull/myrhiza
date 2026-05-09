**Date:** 2026-05-09
**Status:** active
**Subject:** OpenMLS — the Rust implementation of RFC 9420; deep-dive on the surface that matters if Myrhiza adopts MLS

> Companion files: [`protocol.md`](./protocol.md) — RFC 9420 walkthrough; [`group-lifecycle.md`](./group-lifecycle.md) — KeyPackage/Welcome/Commit mechanics; [`other-implementations.md`](./other-implementations.md) — non-Rust alternatives (mlspp, mls-rs, Wire, Webex); [`production-users.md`](./production-users.md), [`comparisons.md`](./comparisons.md), [`lessons.md`](./lessons.md), [`glossary.md`](./glossary.md).

## 1. What it is

OpenMLS is the most widely used Rust implementation of MLS — it tracks RFC 9420, drives the IETF interop pipeline alongside `mlspp`, and is the implementation Myrhiza would most naturally adopt if it commits to MLS for group capabilities. It is jointly maintained by **Phoenix R&D** (Raphael Robert is RFC 9420 co-author) and **Cryspen** (Karthikeyan Bhargavan's formal-verification group), with sustained Mozilla involvement.

| Field | Value |
|---|---|
| Crate (current) | `openmls 0.8.1` (published 2026-02-13) |
| Crate (parallel maint.) | `openmls 0.7.4` (2026-02-17) — older API line |
| Repository | [github.com/openmls/openmls](https://github.com/openmls/openmls) |
| Stars / forks | 930 / 144 |
| Created | 2020-05-16 |
| License | **MIT** (entire workspace) |
| Homepage / book | [openmls.tech](https://openmls.tech), [book.openmls.tech](https://book.openmls.tech) |
| Min. rustc | 1.56+ (workspace targets edition 2021) |
| Recent crate downloads | ~72,499 |
| Stewardship | Phoenix R&D + Cryspen + community |

## 2. Workspace layout

The repository is a Cargo workspace; each member is a focused crate. From the top-level `Cargo.toml`:

| Crate | Purpose |
|---|---|
| `openmls` (`./openmls`) | The protocol library. RFC 9420 state machine, `MlsGroup`, `KeyPackage`, ratchet tree, key schedule. |
| `traits` → `openmls_traits` | The host-abstraction surface: `OpenMlsProvider`, `OpenMlsCrypto`, `OpenMlsRand`, `Signer`, `StorageProvider`, `PublicStorageProvider`. **This is the boundary.** |
| `openmls_rust_crypto` | Default crypto provider, built on `RustCrypto`/`hpke-rs` (Ed25519, P256, AES-GCM, ChaCha20Poly1305, SHA-256, HKDF, HPKE). |
| `libcrux_crypto` → `openmls_libcrux_crypto` | Alternate crypto provider on **libcrux** (Cryspen's formally-verified crypto library) — same trait set, formally verified primitives where possible. |
| `memory_storage` → `openmls_memory_storage` | In-process `HashMap`-backed `StorageProvider` impl. Default for tests. |
| `sqlite_storage` → `openmls_sqlite_storage` | SQLite-backed `StorageProvider` (rusqlite + refinery migrations). |
| `basic_credential` → `openmls_basic_credential` | Reference `BasicCredential` + `SignatureKeyPair`. |
| `openmls_test` | Test harness used in-tree. |
| `interop_client` | gRPC client (`tonic` + `tokio`) for the [mls-implementations](https://github.com/mlswg/mls-implementations) cross-impl test runner. |
| `delivery-service/{ds,ds-lib}` | Reference Delivery Service — illustrative, not a deployment artifact. |
| `cli` | Demo command-line client. |
| `openmls-wasm` | Experimental wasm-bindgen wrapper. **See §6.** |
| `fuzz` | Fuzzing harnesses. |

The crypto-trait abstraction is `openmls_traits::OpenMlsProvider`, an aggregate of `CryptoProvider`, `RandProvider`, and `StorageProvider` — apps pick the providers, the library never instantiates I/O directly.

## 3. Storage abstraction

OpenMLS persists group state through the `StorageProvider<const VERSION: u16>` trait in `traits/src/storage.rs`. The trait is a *typed key-value store* — getters return `Result<Option<T>, E>`; per-group writers/enqueuers cover the `MlsGroupJoinConfig`, `MlsGroupState`, `GroupContext`, `TreeSync`, queued proposals, the interim transcript hash, the confirmation tag, past-epoch secrets, encryption keypairs, PSKs, and the resumption-PSK store. `CURRENT_VERSION = 1`.

What OpenMLS deliberately does **not** do:

- **No I/O.** No filesystem, no network, no time source. The library never opens a socket or a file.
- **No Delivery Service logic.** Delivering Welcome/Commit/handshake messages to peers is the host's problem.
- **No identity / Authentication Service.** Credential validation is a credential-trait extension.
- **No async runtime.** All trait methods are synchronous; persistence is expected to be cheap-and-blocking from OpenMLS's perspective.

OpenMLS itself shadows the public trait with a convenience trait of the same name in `openmls/src/storage.rs` so the generic `VERSION` parameter doesn't pollute call sites. The `Entity` and `Key` marker traits are implemented on every concrete type that crosses the storage boundary (`QueuedProposal`, `TreeSync`, `GroupContext`, `GroupId`, `ProposalRef`, `EncryptionKey`, `KeyPackageBundle`, `PskBundle`, `MessageSecretsStore`, `GroupEpochSecrets`, …).

## 4. Crypto provider abstraction

`OpenMlsCrypto` is the per-call dispatch surface: HKDF, HPKE, AEAD seal/open, signature sign/verify, hash, supported-ciphersuite enumeration. Two implementations ship in-tree:

- **`openmls_rust_crypto` (0.5.1)** — RustCrypto stack: `sha2`, `aes-gcm`, `chacha20poly1305`, `hmac`, `ed25519-dalek`, `p256`, `hkdf`, `rand_chacha`, plus `hpke-rs` + `hpke-rs-rust-crypto` for HPKE. The default for most users.
- **`openmls_libcrux_crypto` (0.3.1)** — Cryspen's libcrux: `libcrux-aead`, `libcrux-ed25519`, `libcrux-hkdf`, `libcrux-sha2`, `libcrux-hmac`, `hpke-rs-libcrux`. Many of these primitives are **formally verified** — libcrux is a hybrid: HACL\*-extracted Rust primitives plus additional Rust code verified directly via Cryspen's `hax` toolchain (see [governance.md](governance.md) for the canonical description). 0.8.0 added AES-GCM here.

Both are gated behind feature flags on the `openmls` crate (`libcrux-provider`, `openmls_rust_crypto`); a third-party can implement `OpenMlsProvider` against any other crypto stack (BoringSSL, AWS-LC, Web Crypto, hardware TEE) without forking.

Supported ciphersuites: `MLS_128_DHKEMX25519_AES128GCM_SHA256_Ed25519` (MTI), `MLS_128_DHKEMP256_AES128GCM_SHA256_P256`, `MLS_128_DHKEMX25519_CHACHA20POLY1305_SHA256_Ed25519`. P-521 / Ed448 / hybrid PQ are not in the shipped matrix.

## 5. API surface

Two layers:

- **`MlsGroup`** — the high-level, persistence-aware group object. The expected entry point. `MlsGroup::new(provider, signer, MlsGroupCreateConfig, group_id)` creates; `MlsGroup::join_by_welcome` / `JoinBuilder` / `external_commit_builder` joins; `propose_*` / `commit_to_pending_proposals` / `merge_pending_commit` advance epochs; `process_message` ingests inbound. Persistence happens against the configured `StorageProvider` automatically.
- **`CoreGroup`** (less commonly imported) — the lower-level state machine without the storage convenience layer. Used by the `PublicGroup` validation path and by tests.

A typical Add/Update/Remove cycle, sketched:

```text
let provider = OpenMlsRustCrypto::default();          // crypto + storage + rand bundle
let group  = MlsGroup::new(&provider, &alice_signer,  // create
                           &MlsGroupCreateConfig::builder().ciphersuite(cs).build(),
                           group_id)?;
let (commit, welcome, _gi) = group.add_members(&provider, &alice_signer,
                                               &[bob_kp])?;        // propose+commit
group.merge_pending_commit(&provider)?;                              // advance epoch
// Bob joins from `welcome` via StagedWelcome::new_from_welcome / build_from_welcome.
// Inbound messages flow through group.process_message(&provider, mls_message).
```

The `MlsGroupBuilder` / `JoinBuilder` style was generalized in 0.7.x; `external_commit_builder` (0.7.1) deprecated `join_by_external_commit`. 0.8.0 added `swap_members()` and a GREASE-injection helper. Unreleased work adds time-based past-epoch secret deletion.

## 6. WASM / Component Model story — load-bearing for Myrhiza

This is the section that decides whether Myrhiza can drop OpenMLS into a `state-apply` component.

**`wasm32-unknown-unknown` is built on CI** but explicitly listed under "Unsupported, but built on CI" in the README — it compiles, it isn't tested for behavior on every PR. The `js` Cargo feature wires `getrandom`'s `wasm_js` backend and `web-time` for the time source; with `js` + `libcrux-provider`, libcrux is supported in wasm too (the `libcrux-provider-js` feature was folded into `libcrux-provider`+`js` in 0.8.0, see PR #1926).

**`openmls-wasm` exists** as a thin `wasm-bindgen` wrapper crate (`crate-type = ["cdylib", "rlib"]`) — its README calls itself "a step on the way to proper Wasm support" and "a test bed for measuring the size of the packed" output. It exposes a minimal slice of the API: `MlsGroup` create/join, `KeyPackage`/`RatchetTree` to/from bytes, message creation, pending-proposal storage. Not a polished SDK; an experiment with active CI size limits.

**Active wasm work in 2025–26.** Recent PRs/issues confirm wasm is a real (if rough) target: PR #1976 "feat(wasm): add KeyPackage/RatchetTree serialization bindings" (closed 2026-03-22), #1949 "Relax Wasm size limit" (Feb 2026), #1745 "bump wasm threshold to 1.7MiB", #1710 / #1709 / #1708 / #1316 building out wasm bindings. Open issues #1486 ("Run more tests on Wasm"), #1485 ("Revisit dependency on getrandom and fluvio-wasm-timer"), and the recent #1983 "external_commit_builder not working in WASM anymore" (2026-03) flag that wasm parity is real but not bulletproof.

**`wasm32-wasi` / WASI Preview 2 / Component Model.** No Cargo feature, no in-tree crate, no issue surface for the **Component Model**. Searching the repo for "component-model" / "wit" / "preview2" returns nothing. OpenMLS targets `wasm32-unknown-unknown` for browsers via `wasm-bindgen`; it does **not** today produce a `.wit`-typed component artifact. A Myrhiza-shaped integration would have to (a) author a WIT world wrapping the OpenMLS surface, (b) implement that world in Rust on top of `MlsGroup`, then (c) build with `wasm32-wasip2` (or `cargo-component`).

**Practical wasm size.** The wasm size threshold is currently 1.7 MiB (PR #1745). This is the full library after wasm-bindgen + LTO; serious binary diet would require feature stripping (drop `libcrux`, drop `serde_json`, drop one of the AEADs).

## 7. Async story

The OpenMLS core is **synchronous**. A search across the workspace for `async fn` outside `interop_client/`, `delivery-service/`, and `sqlx_storage/` returns no hits in `openmls/`, `traits/`, `openmls_rust_crypto/`, `libcrux_crypto/`, `memory_storage/`, or `sqlite_storage/`. The `StorageProvider` trait is sync; `OpenMlsCrypto` is sync; `Signer` is sync.

Where `tokio` shows up: only in `interop_client` (a gRPC client/server scaffold for the IETF interop test runner — `tonic 0.14`, `tokio 1.x`) and in the experimental `delivery-service/` and `sqlx_storage/` projects. The protocol surface itself does not have an async variant.

**Implication for Myrhiza.** A `state-apply` component is itself a synchronous WASM call (`(prior state, event) → new state`). OpenMLS's sync trait surface is a *good* fit for that shape — the entire group state transition can run inside one synchronous WASM invocation, with all I/O (load prior state, persist new state) hoisted out to the kernel via the host import table. The mismatch only appears if a host wants to perform async I/O *inside* a `StorageProvider` call, which the trait does not allow.

## 8. Test coverage and interop

- **IETF interop runner.** `interop_client/` exposes OpenMLS through the gRPC interface defined in `mls_interop_proto` (sourced directly from `github.com/mlswg/mls-implementations`). Docker-compose harness pairs OpenMLS with mlspp for cross-impl test scenarios (`commit.json`, `welcome_join.json`, etc.).
- **RFC 9420 test vectors.** Consumed via the upstream `test_vectors/` corpus (excluded from the published crate).
- **In-tree integration tests.** `openmls/tests/` covers `mls_group.rs`, `external_commit.rs`, `book_code*.rs`, `app_data_update.rs`, `app_ephemeral.rs`, `decryption_key_index.rs`, `grease.rs`, `interop_scenarios.rs`, `readd.rs`, `own_messages.rs`, `opaque_extension.rs`, `data_next_epoch.rs`, `book_code_fork_resolution.rs`, `book_code_past_epoch.rs`. Storage stability KAT in `openmls/src/storage/kat_storage_stability.rs`.
- **Validation tracker.** Phoenix R&D maintains [validation.openmls.tech](https://validation.openmls.tech) tagging concrete RFC sentences (e.g. `valn0311`) to source-line implementations.

## 9. Performance

A `criterion`-based suite lives at `openmls/benches/benchmark.rs`. It exercises:

- KeyPackage bundle creation per ciphersuite,
- group creation,
- Welcome creation,
- Add operations.

It is a smoke benchmark, not a published scaling study. RFC 9420 itself guarantees `O(log N)` handshake cost via TreeKEM, and Cryspen's writeups argue OpenMLS's TreeKEM matches that bound; there is no canonical N-vs-latency table in-tree at the time of writing. For numbers on N up to thousands of members, the literature reference is the Cryspen blog and Phoenix R&D's writeups (linked from `openmls.tech`), not a CI-generated artifact.

## 10. Stewardship reality

Top contributors by commit count (`gh api repos/openmls/openmls/contributors`):

| Login | Commits | Affiliation |
|---|---|---|
| `franziskuskiefer` | 538 | Cryspen (formerly Mozilla) |
| `raphaelrobert` | 469 | Phoenix R&D — RFC 9420 co-author |
| `kkohbrok` | 232 | Phoenix R&D |
| `duesee` | 190 | Phoenix R&D / community |
| `wysiwys` | 89 | community |
| `keks` | 79 | community / cryspen |
| `dependabot[bot]` | 36 | — |
| `beltram` | 17 | Wire (mls-rs / Wire MLS port history) |
| `nplasterer` | 7 | Phoenix R&D |

Distribution: clearly multi-org (Cryspen + Phoenix R&D dominate, with Mozilla, Wire, and community contributors in the long tail). Repo is not single-maintainer — bus factor ≥ 3 across the top of the list. Phoenix R&D (Raphael Robert) and Cryspen (Karthikeyan Bhargavan / Franziskus Kiefer) are independent companies, which is healthier than single-corporate stewardship.

OTF (Open Technology Fund) has supported MLS-adjacent work at Phoenix R&D ("Project Phoenix") — non-trivial public funding signal.

## 11. Versioning posture

The crate is `0.x` — semver-minor bumps are permitted breaking changes. Recent track record:

- **0.7.0 (2025-07)** — broad API additions (`member_at`, `not_before`/`not_after`, `unknown()`).
- **0.7.1 (2025-09)** — `external_commit_builder` introduced; `join_by_external_commit` deprecated; "Safe exporter" (extensions-draft-08) gated behind a feature.
- **0.7.2 / 0.7.3 / 0.7.4** — bugfix line.
- **0.8.0 (2026-02-04)** — `swap_members`, GREASE proposal/extension/credential variants, AppEphemeral, `MlsGroup` creation now refuses to overwrite an existing `GroupId` (must call `replace_old_group`), `getrandom 0.3.4`, `libcrux-provider-js` feature folded into `libcrux-provider`+`js`.
- **0.8.1 (2026-02-13)** — security advisory remediations in libcrux/hpke-rs deps (GHSA-435g-fcv3-8j26, GHSA-g433-pq76-6cmf) and additional accessors.
- **Unreleased** — time-based past-epoch secret deletion (#1972), `propose_self_update_with_new_signer` (#2010).

`StorageProvider<VERSION>` is itself versioned (`CURRENT_VERSION = 1`); a future schema migration will bump it. There is no public 1.0 timeline.

## 12. Implications for Myrhiza

- **Crate boundary is right-shaped.** OpenMLS already cleaves cleanly into "protocol logic" (`openmls`) and "host services" (`StorageProvider`, `OpenMlsCrypto`, `OpenMlsRand`). That split is exactly the shape Myrhiza wants to push into a WASM Component Model boundary — the host services map onto kernel-mediated capabilities, the protocol logic runs inside a guest component.
- **Sync trait surface fits `state-apply`.** Because `StorageProvider`, `OpenMlsCrypto`, and `Signer` are all synchronous, the OpenMLS state transition can run inside a single synchronous WASM invocation — no need to fight the async-vs-sync impedance that breaks plain `tokio`-based libraries inside a deterministic `state-apply`.
- **`StorageProvider` ↔ kernel I/O capability.** Myrhiza's plan is for apps to never touch I/O directly; a WIT world that wraps `StorageProvider` (typed K-V getters/setters/enqueuers, no streams, no async) is a near-1:1 import-list. Storage durability and atomicity move to the kernel.
- **Determinism caveat.** OpenMLS uses `rayon` (parallel iterator) inside the protocol crate. For a strictly-deterministic `state-apply`, that has to be feature-gated off, or the parallel paths audited to confirm they do not affect output ordering. Crypto RNG is consumed during `KeyPackage` creation and signing — those are intentionally non-deterministic, which is fine for `state-propose` but disqualifies the same code path from `state-apply`. Myrhiza should run OpenMLS in `state-propose` and re-derive the deterministic state from the published commit message in `state-apply`.
- **WASM Component Model gap is real.** OpenMLS today targets `wasm32-unknown-unknown` via `wasm-bindgen`; it does not produce a Component-Model artifact. Adopting it on Myrhiza requires authoring a WIT world over the `MlsGroup` surface and building with `cargo-component` / `wasm32-wasip2`. Size budget: plan for ~1.7 MiB+ before further trimming.
- **Crypto agility.** If the formally-verified path matters (it should), use `openmls_libcrux_crypto`. If the smaller binary matters, use `openmls_rust_crypto` and feature-strip what the kernel doesn't need.
- **Versioning risk.** 0.x semantics mean breaking changes per minor — Myrhiza either pins exactly or commits to keeping up. The 0.7→0.8 jump (Feb 2026) was substantial.

## 13. Sources

- [github.com/openmls/openmls](https://github.com/openmls/openmls) — repository (license MIT, 930 stars, 144 forks)
- Workspace `Cargo.toml`, `openmls/Cargo.toml`, `traits/Cargo.toml`, `openmls_rust_crypto/Cargo.toml`, `libcrux_crypto/Cargo.toml`, `sqlite_storage/Cargo.toml`, `memory_storage/Cargo.toml`, `openmls-wasm/Cargo.toml` (`gh api repos/openmls/openmls/contents/...`)
- `traits/src/storage.rs`, `traits/src/traits.rs`, `openmls/src/storage.rs`
- [openmls/openmls CHANGELOG](https://github.com/openmls/openmls/blob/main/CHANGELOG.md) — 0.7.x / 0.8.x entries
- `gh api repos/openmls/openmls/contributors` — commit distribution
- Issue/PR search `repo:openmls/openmls wasm`: #1316, #1483, #1485, #1486, #1708, #1709, #1710, #1737, #1745, #1921, #1949, #1976, #1983
- `openmls/benches/benchmark.rs`, `interop_client/README.md`
- [openmls.tech](https://openmls.tech), [book.openmls.tech](https://book.openmls.tech), [validation.openmls.tech](https://validation.openmls.tech)
- [Phoenix R&D — phnx.im](https://phnx.im/openmls), [Cryspen — cryspen.com/openmls/](https://cryspen.com/openmls/)
- [mlswg/mls-implementations](https://github.com/mlswg/mls-implementations) — IETF interop runner
- RFC 9420, July 2023
