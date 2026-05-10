**Date:** 2026-05-09
**Status:** draft
**Parent:** [README.md](README.md)
**Subject:** Myrhiza master design — Crypto primitives


## 9. Crypto primitives and key custody

### 9.1 Kernel custody

All secret material lives in the kernel:

- Private signing keys (per IdentityScope, [identity.md](identity.md) §6).
- Symmetric channel/group keys.
- Ratchet state.
- MLS group state when adopted.

Components hold opaque handles to keys; the kernel custodies bytes.
Secrets do not enter component memory in their raw form.

### 9.2 Primitive crypto host imports

Provisional WIT contract (refined in crypto-and-key-custody child
spec):

```wit
// signature primitives
host.author-event(scope: identity-scope, event-payload: list<u8>) -> sig
host.verify-signature(pubkey: list<u8>, msg: list<u8>, sig: list<u8>) -> bool

// key agreement
host.x25519-ecdh(scope: identity-scope, peer-pubkey: list<u8>) -> secret-handle

// key derivation
host.hkdf-derive(input: secret-handle, info: list<u8>, length: u32) -> secret-handle

// authenticated encryption
host.aead-seal(key: secret-handle, nonce-handle: nonce-handle, plaintext: list<u8>, ad: list<u8>) -> list<u8>
host.aead-open(key: secret-handle, nonce: list<u8>, ciphertext: list<u8>, ad: list<u8>) -> result<list<u8>, error>
// nonce-handle is kernel-allocated, monotonically-derived per (scope, key);
// app components do not pick raw nonces on the seal path. Open path takes
// raw nonce because the ciphertext author transmits it on the wire.

// hashing (also a deterministic helper for state-apply per §5.1)
host.hash(bytes: list<u8>) -> list<u8>
```

Algorithm choices: Ed25519 (signing), X25519 (ECDH), ChaCha20-Poly1305
(AEAD), HKDF-SHA256 (KDF), BLAKE3 (hashing). Provisional; final
selection is in the crypto child spec.

### 9.3 MLS as a module

The official `myrhiza-crypto-mls` module ships as Myrhiza's canonical
group encryption solution when the first MLS-needing app emerges.
The module implements RFC 9420 entirely in WASM, calling the kernel
crypto primitives for cryptographic operations and the kernel
IdentityScope primitive for member/leaf signing keys.

Kernel does not bake any specific MLS implementation. Module authors
may compete; users may choose alternatives. Post-quantum migration
is a module swap, not a kernel ABI change.

The kernel-baked MLS path (PR #636's `host.mls.*` host imports)
remains open as a future-additive ABI change if module-based MLS
proves insufficient. v1 commits the module path.

### 9.4 Other crypto modules

Common patterns ship as additional modules:

- `myrhiza-crypto-channel-key` — symmetric channel-key encryption
  (Willow-shape).
- `myrhiza-crypto-double-ratchet` — Signal-style DM ratchets when DM
  apps emerge.
- `myrhiza-crypto-sealed-content` — NIP-44/59-shape sealed payloads.

All compose the same primitive crypto host imports.


