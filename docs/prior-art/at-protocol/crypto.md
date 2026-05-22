**Date:** 2026-05-22
**Status:** active
**Subject:** AT Protocol cryptography — curve choices, signature scheme, commit signing, why not Ed25519

# Cryptography

AT Protocol's cryptographic choices are conservative and slightly idiosyncratic. The headline decisions:

- **Two curves: `secp256k1` and `NIST P-256`.** Both ECDSA. No Ed25519 anywhere in the rotation-key / signing-key path.
- **ECDSA-SHA256, low-S canonical form, base64url-without-padding encoding.**
- **`did:key` representation** with multicodec prefix for curve identification.
- **No native E2E encryption.** The protocol provides identity and authenticity, not confidentiality. Encrypted messaging is overlaid by third-party apps (see [open-problems.md](open-problems.md) §"No native E2E").

## Curves

| Curve | atproto name | Multicodec prefix | Use case |
|---|---|---|---|
| secp256k1 | `k256` | `0xE7` | Default for `did:plc` rotation keys; bitcoin-ecosystem hardware compatibility |
| NIST P-256 | `p256` | `0x1200` | Alternative for `did:plc` rotation keys; secure-enclave / TPM compatibility |

Both are valid for rotation keys (the high-authority keys). The atproto signing key is more permissive — it can be any `did:key`-compatible curve in principle, but in practice Bluesky uses the same two.

### Why these and not Ed25519?

This is the most-asked question by anyone coming from a modern P2P protocol. The published rationale (from various Bluesky engineering discussions) breaks down as:

1. **Hardware compatibility.** secp256k1 has bitcoin-ecosystem hardware support (Trezor, Ledger, YubiKey). P-256 has TPM and Apple Secure Enclave support. Ed25519 hardware support is patchier; for a system that wants the rotation key to live in a hardware token, ECDSA curves were the more practical choice.
2. **Web Crypto API compatibility.** Browsers' Web Crypto API supports P-256 directly; Ed25519 support is recent and inconsistent. atproto wanted browser-side verification to work without a polyfill.
3. **W3C DID method compatibility.** The DID specs cluster around ECDSA; using Ed25519 would have required either a DID method extension or running outside the WG consensus.
4. **Acceptance of ECDSA's nonce-reuse footgun.** The Bluesky team's stance is essentially "ECDSA's failure modes are well-understood and the libraries are mature; the marginal Ed25519 win isn't worth the ecosystem cost."

This is a defensible call for atproto's deployment posture. **It is the wrong call for Myrhiza**, which is designed peer-symmetric (no browser dependency required), targets WASM with mature Ed25519 libs (`ed25519-dalek`), and doesn't need to interoperate with the DID-method consensus. Willow precedent (Ed25519 throughout) holds.

## Signature scheme

All signatures are **ECDSA-SHA256** in **low-S canonical form**. Encoding is **base64url without padding**.

"Low-S" means: for an ECDSA signature `(r, s)`, the canonical form requires `s ≤ n/2` where `n` is the curve order. This eliminates signature malleability — without it, both `(r, s)` and `(r, n-s)` are valid signatures for the same message, which lets attackers transform signatures and break some application-layer invariants. Bitcoin enforces the same rule.

The encoding choice — **base64url without padding** — is small but important. It means signatures slot into URL parameters, JWT compact-serialization, and other URL-safe contexts without escaping.

## Commit signing

Repository commits are signed by the user's `#atproto` signing key (held by the PDS). The signed object is:

```cbor
{
  "did": "did:plc:ewvi7nxzyoun6zhxrhs64oiz",
  "version": 3,
  "data": <MST root CID>,
  "rev": <TID timestamp>,
  "prev": <prior commit CID or null>
}
```

Signing process:

1. Serialize the unsigned commit as **DAG-CBOR** (deterministic CBOR encoding — see below).
2. Hash the serialization with **SHA-256**.
3. Sign the hash with the PDS's instance of the user's signing key, producing a low-S ECDSA signature.
4. The signed commit object adds a `sig` field containing the raw signature bytes.

Verification (e.g., by a Relay):

1. Fetch the user's DID document.
2. Extract the `#atproto` verification method's public key.
3. Re-serialize the unsigned commit as DAG-CBOR.
4. Hash with SHA-256.
5. Verify the signature against the hash + public key.

Anyone with the DID document can verify any commit. The DID document is publicly resolvable (`plc.directory` for `did:plc`, HTTPS for `did:web`).

## PLC operation signing

PLC operations are similarly signed but by **rotation keys** rather than signing keys. The operation object:

```json
{
  "type": "plc_operation",
  "rotationKeys": ["did:key:zQ3sh...", "..."],
  "verificationMethods": { "atproto": "did:key:zDn..." },
  "alsoKnownAs": ["at://alice.bsky.social"],
  "services": { "atproto_pds": { "type": "AtprotoPersonalDataServer", "endpoint": "https://..." } },
  "prev": "<CID of prior PLC operation, or null for genesis>",
  "sig": "<base64url-no-padding ECDSA signature>"
}
```

Signing:

1. Serialize unsigned operation as DAG-CBOR.
2. Hash with SHA-256.
3. Sign with **any one of the current rotation keys**.
4. PLC directory validates against the current rotation key set and the priority-order rules for recovery (see [identity.md](identity.md) §"72-hour recovery window").

Priority matters: a recovery operation must be signed by a higher-priority rotation key than the operation it's clobbering. Lower-priority key tries to override higher-priority key's recent operation? Rejected.

## DAG-CBOR — deterministic serialization

Both commit signing and PLC operation signing rely on **DAG-CBOR**, IPFS's deterministic CBOR encoding. Properties:

- **Canonical key ordering** in maps (lexicographic byte order).
- **No floating-point** (or restricted-form floats; atproto avoids them entirely in records).
- **No semantic tags** other than the CID tag (tag 42).
- **No indefinite-length items.**

The "deterministic" part means: two implementations encoding the same logical CBOR object produce byte-identical output. This is **essential** for signature verification — if the serialization weren't deterministic, the signer's hash and the verifier's hash would diverge on irrelevant encoding differences.

This is the same property Myrhiza's state-apply needs and the same property Willow's bincode-with-sorted-collections discipline gives. The pattern is identical: when you need cryptographic verification of structured data, you need a canonical serialization.

## What's missing — confidentiality

AT Protocol has **no native end-to-end encryption.** Repos are public by default; the protocol gives you:

- **Authenticity** (PDS signed it; the rotation key chain authorizes the PDS).
- **Integrity** (MST + commit chain detect tampering).
- **Identifier permanence** (the DID survives PDS migration).

It does not give you:

- **Confidentiality** between users.
- **Forward secrecy** for any message.
- **Post-compromise security** for any key.

This is a deliberate scope choice — atproto is "Twitter, but federated and credibly-exit-able," and Twitter has no E2E messaging. Direct messages on `bsky.app` were added with server-side-readable encryption (Bluesky can see your DMs); native E2E DMs were not in the protocol as of late 2025.

**Germ DM**, launched February 2026, fills this gap by overlaying **MLS** (Messaging Layer Security, RFC 9420 — see `prior-art/mls/`) on top of atproto identities. Germ is a third-party app that uses your atproto DID + handle as the identity primitive but runs its own MLS group state. The integration model is: identity comes from atproto, encryption comes from MLS, and the two don't mix at the protocol level.

**For Myrhiza**: this is a clean separation Myrhiza can either copy or improve on. Copying = keep `prior-art/mls/` as the canonical encryption story, layered over Myrhiza identity. Improving = expose MLS as a `host.mls` capability in the kernel (per PR #636's master spec), making encryption a first-class profile rather than an app-layer overlay.

## What's missing — post-quantum

No post-quantum keys, no post-quantum signature scheme. The MLS WG is working on post-quantum extensions (`prior-art/mls/governance.md` notes work targeting Dec 2026); atproto has no equivalent track of work as of mid-2026.

This is a future-Myrhiza-problem as well. The choice of curve isn't binding forever — adding a post-quantum verification-method type to the DID document is structurally permitted by `did:plc` (it allows any `did:key` curve for signing keys). But the rotation key curves are baked into the PLC operation log and a curve change there is a forklift upgrade. Lesson for Myrhiza: **keep the curve choice contained**, don't bake it into the on-disk operation log.

## Sources

- atproto cryptography spec: <https://atproto.com/specs/cryptography>
- did:key spec: <https://w3c-ccg.github.io/did-method-key/>
- did:plc spec on curves: <https://github.com/did-method-plc/did-method-plc>
- DAG-CBOR spec: <https://ipld.io/specs/codecs/dag-cbor/spec/>
- Germ DM launch (2026-02): <https://techcrunch.com/2026/02/18/a-startup-called-germ-becomes-the-first-private-messenger-that-launches-directly-from-blueskys-app/>
- MLS prior art: [`prior-art/mls/`](../mls/)
