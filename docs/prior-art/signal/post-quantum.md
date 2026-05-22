**Date:** 2026-05-22
**Status:** active
**Subject:** Signal's post-quantum upgrade — PQXDH (X3DH + Kyber hybrid) deployed 2023-09. State of ratchet-level PQ.

# PQXDH — Signal's post-quantum upgrade

PQXDH ("Post-Quantum Extended Diffie-Hellman") replaces X3DH for new sessions.
Announced 2023-09-19, deployed in Signal client apps before announcement
(server side existed by then; clients gradually rolled it in over months).

Authors: Ehren Kret + Rolfe Schmidt. Spec revision 3, dated 2023-05-24, last
updated 2024-01-23.

## What changed

PQXDH = X3DH + a post-quantum KEM, hybrid. The session key is derived from:

```
SK = HKDF(DH1 || DH2 || DH3 || DH4 || SS)
```

where `DH1..DH4` are the same four X25519 exchanges as in X3DH, and `SS` is
the shared secret from a post-quantum **Key Encapsulation Mechanism** (KEM).
Bob publishes a **PQ prekey** alongside his classical prekey bundle, signed
by his identity key; Alice runs KEM-encapsulate to that PQ prekey, gets a
ciphertext + shared secret, and includes the ciphertext in her first
message.

The hybrid construction is the load-bearing safety property: **an attacker
must break both X25519 and the PQ KEM to recover the session key.** A
discovered weakness in either primitive does not break the system.

### Which PQ KEM

The PQXDH spec (rev 3, 2023-05-24) is parameterized over the KEM choice — it
names `CRYSTALS-Kyber-1024` as the example in Section 2.1 and references
`CRYSTALS-KYBER-1024` elsewhere but does not strictly mandate the choice.

Signal's deployed implementation (per the 2023-09-19 blog post and the
libsignal-protocol Cargo.toml dependency on `libcrux-ml-kem` for both
`Kyber768` and `ML-KEM1024` features) uses **CRYSTALS-Kyber-1024** in
production. This is the pre-FIPS-203 Kyber, not the NIST-finalized ML-KEM
(ML-KEM is FIPS 203, published 2024-08-13). Signal had shipped before ML-KEM
was finalized; the migration to ML-KEM-1024 is a subsequent transition that
libsignal's feature flags suggest is in progress.

Verify the current state for any spec citing this: ML-KEM vs Kyber-1024
parameter compatibility is **not guaranteed** — they differ in details. Don't
assume "Signal uses ML-KEM" without checking the current `libcrux-ml-kem`
version pinned in libsignal at the time of citation.

## What PQXDH does NOT cover

**The ratchet itself is still purely classical.** The Double Ratchet's per-
message KDF derives keys from a chain key seeded by the X3DH/PQXDH root
key. Once the session is bootstrapped, ongoing messages do *not* mix in
fresh PQ KEM exchanges — only fresh DH.

Implication: an attacker who *records* PQXDH ciphertexts now and breaks
X25519 in the future (via a quantum computer) can decrypt the *initial*
key agreement only because of `SS`. But subsequent ratchet steps mix in
fresh DH-derived material from `DH(ratchet_pubkey)` — which the quantum
attacker can also break post-hoc. So the **post-compromise security**
property of the ratchet does not survive against a quantum adversary.

Apple's iMessage PQ3 protocol (announced 2024-02-21) explicitly markets
itself as "Level 3" against Signal's "Level 2" specifically on this
distinction: PQ3 runs PQ rekeying *throughout* the session, not just at
setup. See [`comparisons.md`](comparisons.md#imessage-pq3).

Signal has not publicly committed to a ratchet-level PQ upgrade as of the
2024-01-23 PQXDH spec update. The 2023-09 blog post explicitly says
"further upgrades to address the threat of an attacker with a
contemporaneous quantum computer" remain future work.

## "Harvest now, decrypt later" — the actual threat model

The reason PQXDH ships *now* despite no current quantum computer being
large enough to break X25519: an adversary can record ciphertexts today
and decrypt them in N years when quantum computers exist. The threat is
*long-term confidentiality*, not real-time interception.

PQXDH defeats this attack for the *initial key agreement*. The ratchet
gap above means: harvest-now-decrypt-later still wins against the message
keys derived after the initial root key, because the DH ratchet steps are
classical.

The pragmatic argument: by the time a quantum computer that can break
X25519 ratchet steps in real time exists, the world will have transitioned
all DHs to PQ KEMs. PQXDH is a stop-gap that hardens the *initial* key
exchange, which is what an adversary recording today is most able to use
later. The ratchet step is harder to use against many years later because
the volume of ratchet steps to enumerate is enormous and forward secrecy
of the chain key (a one-way HMAC) is not threatened by quantum (HMAC-SHA256
remains secure against Grover with key-length headroom).

## Implementation notes

Signal uses **libcrux-ml-kem** (the Cryspen-maintained Rust implementation
extracted from F\*/hax-verified specifications). The libcrux relationship
is detailed in `prior-art/mls/governance.md` — libcrux is also OpenMLS's PQ
KEM dependency. Same Rust crate, same verified-cryptography lineage.

The PQ KEM operation costs (Kyber-1024 keygen: ~50µs on modern mobile;
encapsulate: ~30µs; decapsulate: ~30µs) are small enough that Signal added
PQXDH without UX regression. Prekey-bundle size grew significantly though
— Kyber-1024 public key is 1568 bytes, vs X25519's 32 bytes — and this
shows up in bandwidth-per-recipient for fan-out multi-device delivery.

## Implications for Myrhiza

- **Pre-quantum forward secrecy is the default property; PQ is an
  add-on.** If Myrhiza adopts X3DH or Double Ratchet for DMs, PQXDH is
  the next-stage hybrid upgrade — not the place to start. The 2023-09
  rollout shows the upgrade is feasible without protocol redesign; the
  KEM ciphertext rides in the existing message header structure.
- **The ratchet-level PQ gap is real and unresolved.** A Myrhiza spec
  that claims post-quantum security needs to be honest: PQXDH alone is
  "Level 2" in Apple's framing. Closing the gap requires research-grade
  work (Signal hasn't shipped it; only Apple has, and PQ3 is not yet
  formally analyzed to Double Ratchet's depth).
- **Library selection: libcrux-ml-kem is a viable Rust dependency.**
  Same crate as OpenMLS uses (see `prior-art/mls/`). Dual Apache-2.0 /
  MIT, verified-via-hax provenance. Don't roll your own Kyber.
- **Prekey-bundle size matters for P2P bandwidth.** A 1568-byte PQ
  prekey × fan-out to N devices × storage at a relay starts to add up.
  Worth being explicit in the spec.

## Sources

- "Quantum Resistance and the Signal Protocol" (2023-09-19): <https://signal.org/blog/pqxdh/>
- PQXDH specification (rev 3, 2023-05-24, updated 2024-01-23): <https://signal.org/docs/specifications/pqxdh/>
- libsignal-protocol Cargo.toml (Kyber768 + ML-KEM1024 features): <https://github.com/signalapp/libsignal/blob/main/rust/protocol/Cargo.toml>
- libcrux project (Cryspen, F\*/hax-verified): <https://github.com/cryspen/libcrux>
- Apple's PQ3 announcement (Level-3 claim): <https://security.apple.com/blog/imessage-pq3/> (2024-02-21)
- NIST FIPS 203 (ML-KEM standardization, 2024-08-13): <https://csrc.nist.gov/pubs/fips/203/final>
- Comparator: `prior-art/mls/` (OpenMLS also depends on libcrux-ml-kem)
