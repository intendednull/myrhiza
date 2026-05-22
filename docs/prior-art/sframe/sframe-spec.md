**Date:** 2026-05-22
**Status:** active
**Subject:** RFC 9605 technical summary — payload format, key schedule, ciphersuites, MLS integration.

# SFrame (RFC 9605) — protocol summary

SFrame is one focused thing: a function that takes (key material, codec frame, KID, counter) and returns an authenticated ciphertext + a short header. The function is independent of the surrounding transport — you can wrap an SFrame ciphertext in RTP, in WebTransport datagrams, in QUIC streams, or in a P2P data-channel and the security analysis does not change.

## 1. Design intent — the PERC postmortem

The SFrame WG charter is explicit about what SFrame is *not*:

> "The PERC WG developed a 'double encryption' scheme for end-to-end encryption that was deeply tied to SRTP as its underlying transport. This entanglement has prevented widespread deployment. This working group will define the SFrame secure encapsulation to provide authenticated encryption for real-time media content that is independent of the underlying transport."

PERC (Privacy-Enhanced RTP Conferencing, RFCs 8723/8871) tried to keep SRTP as the wire format and bolt E2EE on top by adding a second AES-GCM pass before the hop-by-hop SRTP encryption. The result was correct cryptographically but unimplementable in practice: every SFU vendor had to teach its SRTP stack about two layers of keys, header extensions had to survive intermediaries, and key signalling fell on top of SDP. SFrame **decouples** from SRTP: it is no longer a profile inside SRTP, but a transport-independent E2EE transform on *codec frames*. Per RFC 9605 §1 + §4.1, SFrame is "intended to be used as an E2EE layer over an underlying HBH-encrypted transport such as SRTP or QUIC" — composable with SRTP at the HBH layer, not a replacement for it. Bandwidth amplification (frame split across N RTP packets) ceases to be the security layer's problem.

## 2. The transform

```
SFrame_encrypt(KID, CTR, plaintext, metadata) -> (header_bytes, ciphertext_bytes)
SFrame_decrypt(header_bytes, ciphertext_bytes, metadata) -> plaintext or error
```

Where:
- `KID` is a sender/key identifier (variable-length, encoded in the header).
- `CTR` is a per-(sender, key) monotonically increasing counter (the nonce input).
- `metadata` is associated data fed to the AEAD — typically frame metadata the sender wants integrity-protected but not encrypted.

The header carries KID + CTR in a compact variable-length encoding. Everything else (the ciphertext, auth tag) is opaque to intermediaries.

## 3. Ciphersuites

RFC 9605 defines **five** ciphersuites (§4.5 "Cipher Suites"; the suite constants live in §4.4):

| Suite ID | Construction | Tag length | Notes |
|---|---|---|---|
| 0x0001 | `AES_128_CTR_HMAC_SHA256_80` | 80 bits | CTR + truncated HMAC; legacy-compatible |
| 0x0002 | `AES_128_CTR_HMAC_SHA256_64` | 64 bits | Smaller tag for bandwidth-sensitive media |
| 0x0003 | `AES_128_CTR_HMAC_SHA256_32` | 32 bits | Aggressive tag truncation; audio-only contexts |
| 0x0004 | `AES_128_GCM_SHA256_128` | 128 bits | Modern AEAD default |
| 0x0005 | `AES_256_GCM_SHA512_128` | 128 bits | 256-bit security level |

The CTR-mode suites exist because some embedded codec pipelines can pipeline counter-mode trivially but find GCM awkward. The 32-bit-tag suite is contentious: 2^32 forgery resistance is fine for short-lived audio frames but is conservative-feeling for video. RFC 9605 §7.5 ("Risks Due to Short Tags") spells out the threat model and brute-force-attack analysis under which each tag-length variant is acceptable.

## 4. Key schedule

The input is a single externally-supplied `base_key`. SFrame does not specify how `base_key` is established — that's the group-key-protocol's problem (MLS, hand-rolled, whatever). Per-sender keys derive via HKDF:

```
secret = HKDF-Extract(salt = 0, IKM = base_key)
sframe_secret      = HKDF-Expand(secret, "SFrame 1.0 Secret",     N_x)
sframe_key   (KID) = HKDF-Expand(sframe_secret, "SFrame 1.0 Secret key"  || KID, N_k)
sframe_salt  (KID) = HKDF-Expand(sframe_secret, "SFrame 1.0 Secret salt" || KID, N_n)
```

The labels are stable strings (`"SFrame 1.0 Secret key"`, `"SFrame 1.0 Secret salt"`) so independent implementations interop bit-exact off the same `base_key`.

The per-frame nonce is the per-KID `sframe_salt` XOR'd with the encoded counter:

```
nonce = sframe_salt XOR encode(CTR, N_n)
```

This is the standard SIV/STREAM construction adapted to a media-frame setting: each (KID, CTR) pair yields a unique nonce, the salt prevents trivial cross-stream nonce collisions, and the receiver can decrypt out of order as long as they trust their replay window.

## 5. MLS integration (§5.2 "MLS") — the load-bearing bit for Myrhiza

This is the section [`prior-art/mls/open-problems.md`](../mls/open-problems.md) cites. Quoted from RFC 9605 §5.2 "MLS":

> "MLS creates a linear sequence of keys, each of which is shared among the members of a group at a given point in time. ... To generate keys and nonces for SFrame, we use the MLS exporter function to generate a `base_key` value for each MLS epoch."

Mechanically:

```
base_key = MLS-Exporter(label = "SFrame 1.0", context = "", length = N_x)
```

Every MLS epoch (every commit) produces a new `base_key`, which means a new SFrame key schedule. This is the leaver-recovery story: when a member is removed, the next MLS commit produces a new epoch, the SFrame `base_key` rotates, and the removed member can no longer decrypt frames — *after* the commit is applied. The lag between "remove proposal sent" and "next frame decrypts under new key" is the leaver-window, which is bounded by MLS commit latency.

The KID space in this construction is per-member: each MLS member identity (LeafIndex in the ratchet tree) gets a KID, and members ratchet their own counter independently. RFC 9605 does *not* specify the KID-to-member mapping — that's up to the application.

## 6. Header format

The SFrame header (§4.2) is a 1–8 byte variable-length encoding carrying:

- A 1-bit "extended counter" flag.
- 3 bits of length-of-KID.
- 3 bits of length-of-CTR.
- The KID itself (variable length).
- The CTR itself (variable length).

Followed by ciphertext, followed by the authentication tag. The header is *not* encrypted but *is* authenticated (it's part of the AEAD associated data input).

For an audio frame at 50 fps with 8-bit KID and 16-bit CTR, the overhead is 3 bytes header + tag length. With the 32-bit-tag CTR suite that's 7 bytes per frame.

## 7. What SFrame deliberately omits

- **Group key agreement** — MLS does this, or pre-shared keys, or whatever.
- **Signalling** — how senders and receivers learn each other's KIDs is out-of-scope.
- **Codec interop** — SFrame ciphertext goes wherever the codec payload goes; the WebRTC encoded-transform API is one obvious carrier but not specified.
- **SFU behaviour** — the spec assumes the SFU does not need to decrypt; how it handles forwarding/rewriting (RTP sequence numbers, congestion control hints) is the SFU's problem.
- **Key rotation policy** — when to rotate `base_key` within an epoch (e.g. after 2^N frames) is application-defined.

These omissions are deliberate. PERC failed by trying to specify all of them; SFrame's bet is that decoupling them lets implementers ship.

## 8. Sources

- [RFC 9605 — Secure Frame (SFrame): Lightweight Authenticated Encryption for Real-Time Media](https://www.rfc-editor.org/rfc/rfc9605.html)
- [RFC 9605 §4 — SFrame Encryption](https://www.rfc-editor.org/rfc/rfc9605.html#section-4)
- [RFC 9605 §5.2 — MLS](https://www.rfc-editor.org/rfc/rfc9605.html#section-5.2)
- [RFC 9605 §9 — Security Considerations](https://www.rfc-editor.org/rfc/rfc9605.html#section-9)
- [SFrame WG charter](https://datatracker.ietf.org/wg/sframe/about/)
- [RFC 8723 — PERC double encryption](https://www.rfc-editor.org/rfc/rfc8723.html)
- [RFC 9420 §8.5 — MLS Exporter](https://www.rfc-editor.org/rfc/rfc9420.html#section-8.5)
- [RFC 5869 — HKDF](https://www.rfc-editor.org/rfc/rfc5869.html)
