**Date:** 2026-05-22
**Status:** active
**Subject:** libp2p — identity, PeerId, multihash, key types, Noise XX handshake

# Identity & crypto

A libp2p peer identifies itself with a **PeerId** — a multihash of its public key. Default key type is Ed25519. The handshake protocol (Noise XX or libp2p-TLS) verifies the remote PeerId by checking the remote's signature against the public key, and uses the public key to derive session keys.

This is the same shape as iroh's `EndpointId` ([`../iroh/identity.md`](../iroh/identity.md)) — a public key, no DID layer, no certificate authority. The differences are in **(a) which key types are supported**, **(b) the PeerId encoding**, and **(c) the handshake protocol details**.

## PeerId

A PeerId is a **multihash** of the peer's serialized public key. Format:

```
PeerId = multihash(serializedPublicKey, hash_algo)
```

Where `serializedPublicKey` is a [protobuf-encoded](https://github.com/libp2p/specs/blob/master/peer-ids/peer-ids.md) `PublicKey` message and `hash_algo` is selected as follows:

- **For keys ≤ 42 bytes** (Ed25519, Secp256k1, ECDSA) — `identity` multihash (no hash, just the raw bytes). This optimisation means most PeerIds are *the entire public key with a multihash prefix*.
- **For larger keys** (RSA) — `sha2-256` multihash.

The wire encoding is multibase. Most-seen forms:

- **`12D3KooW...`** — base58btc-encoded PeerId. Standard since ~2020.
- **`Qm...`** — base58btc-encoded SHA-256 PeerId (legacy RSA-key peers from the early IPFS era).
- **`bafzaaja...`** — CIDv1-encoded PeerId. Used when the PeerId is treated as a CID.

A PeerId is **case-sensitive** in base58 but **case-insensitive** in CIDv1.

The implication: **for Ed25519 / Secp256k1 / ECDSA keys, the PeerId is fundamentally the public key**. Compare iroh's `EndpointId` — a raw 32-byte Ed25519 public key with base32 encoding. Same conceptual shape, different framing.

## Key types

libp2p supports four key types, configurable per-peer:

| Key type | Status | Default in | Use case |
|---|---|---|---|
| **Ed25519** | Active, default everywhere | rust, go, js, nim (default) | Modern peers; the "right" choice |
| **Secp256k1** | Active | Filecoin, Ethereum-derived clients | Compatibility with chain identities (Eth, Bitcoin) |
| **ECDSA (NIST P-256)** | Active | TLS-compat scenarios | When TLS-1.3 chains require P-256 |
| **RSA (≥2048-bit)** | **Legacy** | Early IPFS peers | Historical only; **deprecated for new keys** |

Each implementation defaults to **Ed25519** for new keys. RSA is universally on its way out — the spec allows it for backward compatibility with pre-2020 IPFS peers, but no current best-practice recommends it.

The protobuf encoding of `PublicKey` is:

```protobuf
message PublicKey {
    KeyType Type = 1;          // enum: RSA=0, Ed25519=1, Secp256k1=2, ECDSA=3
    bytes Data = 2;            // the public key bytes (DER for RSA, raw for others)
}
```

`PrivateKey` is the same shape with `Data` being the secret key.

## Identity protocol (identify, identify-push)

Every libp2p connection produces an identify exchange. See [`architecture.md`](architecture.md) §"Identify" for the full message contents. The PeerId-relevant fields:

- `public_key` — the protobuf-encoded public key. Verifying that `multihash(public_key) == PeerId` is the proof of identity.
- `signed_peer_record` — a signed bundle of `(peer_id, addresses, seq, timestamp)`. The signature is by the peer's identity key. Other peers can forward this bundle (e.g. in gossipsub PX) without forging the address claim.

Identify-push is a follow-on protocol: when a peer's listen addresses change, it pushes an updated identify message to its connections without waiting for them to ask.

## Noise XX handshake (the default)

libp2p's primary security layer is Noise XX. Spec: [`noise` r5, 2022-12-07](https://github.com/libp2p/specs/blob/master/noise/README.md), 3A Recommendation.

Author: [`@yusefnapora`](https://github.com/yusefnapora). Production Noise pattern: **`Noise_XX_25519_ChaChaPoly_SHA256`** — the only pattern libp2p supports. Per the spec: *"Noise_XX_25519_ChaChaPoly_SHA256 must be used"* — no negotiation, no alternatives.

The XX pattern is a 3-message handshake with mutual authentication:

```
1. → e                            # initiator sends ephemeral
2. ← e, ee, s, es                 # responder sends ephemeral + static
3. → s, se                        # initiator sends static
```

Where `e` is the ephemeral key, `s` is the static key, and `ee/es/se` are DH operations.

The **libp2p extension** is the *handshake payload* — both peers attach a payload containing:

- `identity_key` — the peer's identity public key (which the PeerId is derived from).
- `identity_sig` — a signature over `("noise-libp2p-static-key:" || noise_static_key)` by the identity key.

The libp2p-extension proves: "I (the peer with PeerId X) authorize this Noise static key to be used on my behalf." This is the **distinct Noise key + identity key pattern**:

- Noise has its own static keypair, used for the Noise handshake.
- The identity key (Ed25519 / Secp256k1 / etc.) signs the Noise static key.
- The PeerId is derived from the identity key, *not* the Noise key.

Why distinct keys? Per the spec's "Distinct Noise and Identity Keys" section: it lets the identity key be a long-term key in a hardware module (e.g. TPM, secure enclave) while the Noise static key is rotated per-process or per-deployment without affecting the PeerId. iroh, by contrast, uses a single Ed25519 key for both transport identity *and* TLS — simpler, but the key has to be exposed to the QUIC stack.

### Why XX (and not IK or NK)?

The spec section "Why the XX handshake pattern?" answers this verbatim:

- **XX** transmits both peers' static keys during the handshake — both can be unknown to the other ahead of time. **Mutual authentication is achieved during the handshake itself.**
- **IK** assumes the initiator already knows the responder's static key. Useful for client-to-known-server but doesn't fit P2P where peers discover each other.
- **NK** has the initiator anonymous; doesn't fit libp2p's "every peer has an identity" model.

XX is the right pattern for true peer-to-peer where neither side is privileged.

### Why ChaChaPoly (and not AES-GCM)?

ChaCha20-Poly1305 is the libp2p default because it has **better performance without hardware AES acceleration**. AES-GCM is fast on x86 with AES-NI but slower on ARM (mobile) and constant-time-by-construction is harder to guarantee in pure-software implementations.

## libp2p-TLS (the QUIC requirement)

For QUIC transport, security is TLS 1.3 (which is part of QUIC by definition). libp2p-TLS is the spec ([`/tls`](https://github.com/libp2p/specs/tree/master/tls)) for how libp2p uses TLS 1.3 for peer authentication.

The trick: **self-signed certificate with a libp2p extension**. The TLS cert is signed by an ephemeral keypair; the cert's `signatureExtension` field contains a signature by the *identity key* over the ephemeral public key. During the handshake, both sides:

1. Receive the remote's self-signed cert.
2. Extract the libp2p extension.
3. Verify the extension's signature using the identity key.
4. Verify the cert's signature using the ephemeral key.
5. Compute the remote PeerId from the identity key.

If any step fails, the connection is aborted. The result: TLS 1.3 with a libp2p PeerId binding, no CA infrastructure.

This is the same pattern iroh uses for its QUIC transport — the QUIC TLS cert is self-signed by the EndpointId key. The libp2p spec is more elaborate (ephemeral cert key, signed extension) where iroh uses the identity key directly to sign the cert.

## No DID, no rotation, no portability

libp2p has explicitly no support for:

- **DIDs** (Decentralized Identifiers — W3C spec). A libp2p PeerId is a public key, not a resolver-backed identifier. Apps that need DIDs build them above libp2p.
- **Key rotation.** Changing your key changes your PeerId — there is no continuity across rotations. Lose your key, lose your identity. Same as iroh.
- **Multi-device identity.** One PeerId = one keypair = (effectively) one device. There is no analog to FROST threshold signatures or SSI-style key federation. Apps that need this (e.g. Status, Nostr-over-libp2p experiments) build it above.

These are *transport-layer* design choices — the libp2p team's position is the same as iroh's: "a peer identity is a transport credential; application identity is the application's problem." For Myrhiza this means the same lesson holds: **PrincipalID (application identity, recoverable, multi-device) and PeerId/EndpointId (transport credential, per-device) are separate concepts.** See [`../iroh/identity.md`](../iroh/identity.md) and [`lessons.md`](lessons.md) §"Avoid".

## Sybil resistance: none

Same as iroh. Anyone can spin up arbitrarily many PeerIds at zero cost. Per-app membership proofs, capability-token gating, social-graph trust, or proof-of-personhood mechanisms are application-layer concerns. The transport layer is intentionally Sybil-permissive — libp2p ships gossipsub's *peer scoring* as the closest-to-Sybil-resistance primitive available at the transport layer (see [`gossipsub.md`](gossipsub.md) §"Peer scoring"), but it's a local-knowledge system, not a global Sybil floor.

## Implications for Myrhiza

- **Ed25519 PeerId is the right primitive.** Universal libp2p default, matches iroh, matches every modern P2P system. Don't invent.
- **The distinct-Noise-key-from-identity-key pattern is worth considering** if Myrhiza ever wants identity keys in hardware modules. iroh's single-key approach is simpler today; libp2p's pattern is more flexible at the cost of one signature operation.
- **PeerId vs EndpointId framing is just naming.** Both are public-key-as-identity. Myrhiza's internal naming (`PeerPubkey` per `network` crate) is cleaner than either; keep it.
- **The libp2p-TLS spec is worth reading if Myrhiza ever has to write its own TLS-based transport.** The self-signed-cert-with-libp2p-extension pattern is the canonical way to do "PKI without a CA" over TLS 1.3.
- **No DID, no rotation, no multi-device is a deliberate transport-layer choice.** Myrhiza's PrincipalID layer must solve this independently. See `lessons.md` for the architectural split.

## Sources

- [libp2p peer-id spec](https://github.com/libp2p/specs/blob/master/peer-ids/peer-ids.md)
- [libp2p Noise spec (r5, 2022-12-07)](https://github.com/libp2p/specs/blob/master/noise/README.md)
- [libp2p TLS spec](https://github.com/libp2p/specs/tree/master/tls)
- [Noise Protocol Framework](https://noiseprotocol.org/)
- [multiformats/multihash](https://github.com/multiformats/multihash)
- [libp2p-identity crate](https://crates.io/crates/libp2p-identity) (rust-libp2p, MIT, v0.2.13)
- [iroh — identity (sibling doc)](../iroh/identity.md)
- [Myrhiza prior-art: MLS — group key agreement (related identity discussion)](../mls/openmls.md)
