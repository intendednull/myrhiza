**Date:** 2026-05-22
**Status:** active
**Subject:** libsignal — Signal's Rust reference implementation of the Signal Protocol. Crate layout, FFI bindings, AGPL-3.0 implications.

# libsignal — the Rust reference implementation

Repository: <https://github.com/signalapp/libsignal>. **AGPL-3.0.** ~5.8k
GitHub stars. Tag-line: "Home to the Signal Protocol as well as other
cryptographic primitives which make Signal possible."

The repo is the canonical implementation of the Signal Protocol family
shipped by Signal Foundation in production. Other implementations of the
*protocol* exist (WhatsApp, Google Messages RCS, Matrix's `vodozemac`), but
libsignal is the one Signal Foundation ships and supports.

## Top-level layout

```
libsignal/
├── rust/             # Pure-Rust crates (the substance)
├── java/             # JNI bindings for Signal-Android
├── swift/            # Swift bindings for Signal-iOS
├── node/             # Node.js bindings for Signal-Desktop
├── bin/              # CLI utilities (signal-protocol-bench etc.)
├── doc/              # internal-facing docs
├── acknowledgments/  # third-party-license metadata
└── .github/          # CI
```

The Rust workspace is the substance. The other directories are FFI thin
shims that compile the Rust workspace as a static library and re-export the
public API for each target platform.

## Rust workspace crates

From the libsignal repository's `rust/` directory (verified via README and
Cargo.toml inspection 2026-05-22):

| Crate | Purpose |
|---|---|
| `libsignal-protocol` | The Double Ratchet + X3DH/PQXDH implementation. The headline crate. Version 0.1.0, edition 2024, rust-version 1.85. |
| `libsignal-core` | Shared types: identifier types, identity keys, address types. |
| `signal-crypto` | Lower-level cryptographic primitives: AES-CBC, AES-GCM, HMAC, KDFs. |
| `device-transfer` | Direct device-to-device transfer (when migrating phones, etc.) |
| `attest` | Remote attestation for SGX enclaves (contact discovery, SVR). |
| `zkgroup` | Zero-knowledge group anonymous credentials (the KVAC scheme — see `groups.md`). |
| `zkcredential` | Generic zero-knowledge credential abstractions used by `zkgroup`. |
| `poksho` | Zero-knowledge proof toolkit (custom NIZK over Curve25519). |
| `account-keys` | PIN-based password derivation and master-key management (SVR client side). |
| `usernames` | Username generation, hashing, discovery proof. |
| `media` | Media manipulation for attachments. |
| `libsignal-debug` | Debug-only logging helpers. |

The crate count is moderate (~12-15 internal crates plus dependencies);
the workspace is laid out for clean FFI export rather than for external
Rust consumers.

## Key dependencies

From `rust/protocol/Cargo.toml` (verified 2026-05-22):

**Cryptography (RustCrypto family):**
- `aes`, `aes-gcm-siv` — AES + nonce-misuse-resistant AEAD
- `hkdf`, `hmac`, `sha2` — KDF + MAC + hash
- `curve25519-dalek` — implied via re-exports (X25519 + Ed25519)

**Protocol buffers:** `prost` — Signal's wire format is Protobuf.

**Post-quantum:** `libcrux-ml-kem` — Kyber768 and ML-KEM1024 feature-flagged.
This is the same Cryspen-maintained verified-Rust ML-KEM crate that OpenMLS
uses (see `prior-art/mls/openmls.md`).

**Serialization:** `serde` + `serde_bytes` for some interop paths.

**Concurrency:** `rayon` for parallelizable bulk operations (zkgroup proof
generation in particular).

The dependency surface is comparable to OpenMLS's: pure-Rust crypto, no
OpenSSL, no foreign C dependencies (except via libcrux's internal HACL\*
extraction, which is statically linked).

## FFI bindings

Three bindings, all hand-maintained:

- **Java/Android** (`java/`): JNI bridge. Signal-Android consumes
  libsignal as an `aar` artifact built by libsignal's CI. The JNI surface
  is a deliberate subset of the Rust API — not every Rust function is
  exported, to keep the FFI ABI stable.
- **Swift/iOS** (`swift/`): Swift Package Manager package. Generates a
  `.xcframework` for iOS distribution. Same subset-API constraint.
- **Node.js** (`node/`): N-API bindings (via the `napi` crate ecosystem).
  Consumed by Signal-Desktop (an Electron app).

A `wasm32` target *does* compile some libsignal crates (e.g., zkgroup) but
not the full protocol crate, and there's no shipped WASM artifact. WASM is
not on Signal's deployment roadmap.

## License — AGPL-3.0 implication

The libsignal repo's LICENSE file declares **GNU AGPLv3** ("Copyright
2020-2026 Signal Messenger, LLC. Licensed under the GNU AGPLv3"). The same
license applies to Signal-Android and Signal-iOS.

**This is a strong copyleft + network use trigger:**

- Any software that links libsignal must be AGPL-3.0 compatible.
- The "A" in AGPL: making libsignal-derived code *available over a
  network* (running a server that uses it) triggers the AGPL's source-
  release requirement. Signal Foundation makes libsignal source available
  precisely because they themselves run a server.

For Myrhiza:

- **Cannot link libsignal into Myrhiza unless Myrhiza adopts AGPL-3.0
  itself.** This is almost certainly a non-starter for a P2P runtime
  meant to host third-party apps — Myrhiza's apps are at minimum
  un-knowable in terms of licensing, and AGPL would force every app
  using a Signal-protocol component to be AGPL.
- **Re-implement from the spec instead.** The Signal Protocol *spec text*
  (X3DH, Double Ratchet, PQXDH, Sealed Sender) is CC-BY 4.0 on
  signal.org/docs. A clean-room Rust impl built against the spec
  documents is unencumbered.
- **Alternate Rust impls exist.** Matrix.org's `vodozemac` is an MIT-
  licensed Rust impl of the Olm + Megolm protocols (variants of Signal's
  Double Ratchet). Not bit-compatible with libsignal but operationally
  the same shape. See [`comparisons.md`](comparisons.md#vodozemac).

### Contributor License Agreement

External contributors to libsignal must sign Signal's CLA, which grants
Signal Foundation the right to relicense contributions. This is unusual
among open-source crypto libraries (RustCrypto, BoringSSL, Tink do not
require a CLA). Worth flagging: contributions back to libsignal are
relicensable by Signal Foundation alone, not by the community.

## What libsignal does NOT include

- **Server code.** The Signal server (which runs the centralized service)
  has its own repo: `signalapp/Signal-Server` (Java + Kotlin, also
  AGPL-3.0). libsignal is the *client and protocol* library.
- **Transport-layer code.** libsignal does not include the websocket
  connection to Signal's server, message queue management, push
  notification handling, or media-CDN upload code. Those live in the
  per-platform Signal-{Android, iOS, Desktop} apps.
- **UI.** libsignal is headless. Renders nothing. Stores nothing
  persistently — clients are expected to provide a persistence callback
  for session state.
- **Group state replication.** libsignal handles the Double Ratchet
  pairwise; group-state synchronization is layered on top in the
  application code with help from `zkgroup`. See [`groups.md`](groups.md).

## Build complexity

Building libsignal from source requires:

- Rust (stable, version pinned via `rust-toolchain.toml` — currently
  edition 2024 / rust-version 1.85).
- Clang + CMake + Make + protoc (for protobuf generation).
- Python 3.9+ (build scripts).
- Platform-specific tooling for the FFI targets you want: Android NDK +
  JDK 17 for Android, Xcode for iOS, Node.js for the JavaScript binding.

A pure-Rust consumer that only wants the protocol crate can skip the FFI
prerequisites. Build-from-source is well-documented in the repo README;
several CI failure modes are common (NDK version mismatches, protoc
version mismatches), so practical use through prebuilt FFI artifacts is
the default.

## Implications for Myrhiza

- **libsignal is the reference, but you cannot link it.** AGPL-3.0
  forecloses direct dependency for any spec that involves third-party
  apps. Build for clean-room interop from the spec docs.
- **The crate layout is a useful model.** Splitting protocol /
  primitives / FFI / each verified-crypto consumer (zkgroup, attest)
  into separate crates is a clean pattern. Myrhiza's crypto layer (when
  it lands) could borrow the layout without borrowing code.
- **PQ KEM via libcrux-ml-kem is the right shared dependency.** Both
  libsignal and OpenMLS depend on it. Myrhiza should too if it adopts
  PQXDH-style hybrid.
- **`zkgroup` is the production reference for anonymous group
  membership** if Myrhiza ever wants a similar property (private group
  membership where the server can't enumerate members). KVAC is a
  research-heavy area; libsignal's implementation has been deployed at
  70M-MAU scale for ~5 years and is the most-battle-tested example.

## Sources

- libsignal repository: <https://github.com/signalapp/libsignal> (verified 2026-05-22)
- libsignal-protocol Cargo.toml: <https://github.com/signalapp/libsignal/blob/main/rust/protocol/Cargo.toml>
- libsignal README: <https://github.com/signalapp/libsignal/blob/main/README.md>
- AGPL-3.0 license text: <https://www.gnu.org/licenses/agpl-3.0.html>
- Signal-Server repository: <https://github.com/signalapp/Signal-Server>
- Matrix `vodozemac` (MIT alternative): <https://github.com/matrix-org/vodozemac>
- Comparator: `prior-art/mls/openmls.md` (shares libcrux-ml-kem dependency)
