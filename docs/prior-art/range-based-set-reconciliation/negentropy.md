**Date:** 2026-05-29
**Status:** active
**Subject:** Negentropy + Nostr NIP-77 — the cleanest deployed 1D embodiment of RBSR

# Negentropy / NIP-77

Negentropy is Doug Hoyte's (hoytech) implementation of RBSR: a small,
MIT-licensed, transport-agnostic protocol that does the "diff two sets of 32-byte
IDs" job — and nothing else. Protocol design is, per the README, "based on
Aljoscha Meyer's Range-Based Set Reconciliation research"
([github.com/hoytech/negentropy](https://github.com/hoytech/negentropy)).
(Whether it is the *right* exemplar for Myrhiza is the recommendation in
[lessons.md](lessons.md); this file records what it is.)

## The protocol surface

- **Items are 32-byte IDs.** "Record identifiers must be exactly 32 bytes,
  typically cryptographic hashes of complete records" (negentropy README;
  NIP-77 calls it "a 256-bit ID … `Byte{32}`"). Negentropy never sees record
  *contents* — only IDs and an associated sort key.
- **Sort order is `(timestamp, id)`.** Ranges use timestamp + ID-prefix bounds,
  not array indices. "Lower bounds are inclusive; upper bounds are exclusive."
- **Incremental fingerprints.** "Negentropy fingerprints are specified as an
  incremental hash," so a node computes any sub-range's digest without
  re-hashing the whole set. The concrete V1 construction — *add IDs mod 2²⁵⁶,
  append the count as a varint, SHA-256, take the first 16 bytes* — is the
  known-good cryptographically-secure scheme; the full breakdown and why the
  SHA-256 wrap defeats forgery live in [rbsr-algorithm.md](rbsr-algorithm.md)
  "Fingerprints."
- **Frame size limits.** An optional `frameSizeLimit` caps message bytes; "After
  processing each message, any discovered differences will be included in the
  have/need arrays on the client." Bounding frame size trades more round trips
  for smaller messages — important when relaying over constrained links.

The reconciliation produces two arrays per side: **`have`** (IDs the local side
holds that the remote lacks) and **`need`** (IDs the remote holds that the local
lacks). The caller then fetches the `need` items over whatever transport it
likes — negentropy itself does not move records.

## NIP-77: the Nostr wrapper

[NIP-77](https://nips.nostr.com/77) ("Negentropy Syncing") makes negentropy
speak Nostr. Status: **`draft` `optional` `relay`** — experimental, not
finalized. It "is a Nostr-friendly wrapper around the Negentropy protocol" and,
because negentropy is a binary protocol, the wrapper **hex-encodes its messages**.

Four message types:
- `NEG-OPEN` — client → relay, opens a session against a filter (a Nostr query).
- `NEG-MSG` — bidirectional, carries the recursive fingerprint/ID-list payloads.
- `NEG-CLOSE` — client terminates the session.
- `NEG-ERR` — relay error response.

Protocol versioning is a single byte: the current Negentropy protocol version is
`0x61`; "Protocol version 2 will be `0x62`, and so forth." Both sides negotiate
to the highest mutually understood version. The `idSize` is 32 (Nostr event IDs
are 32-byte SHA-256 hashes), and the sort key is the event `created_at`
timestamp — a natural fit because Nostr events are already
`(timestamp, 32-byte-id)` tuples.

## Deployment: strfry and rust-nostr

- **strfry** (Hoyte's C++ Nostr relay) uses negentropy for relay↔relay and
  relay↔client sync: "figuring out which events each side has that the other
  doesn't." strfry's own doc: "If both sides of the sync have events in common,
  then this protocol will use less bandwidth than transferring the full set of
  events (or even just their IDs)."
- **Scale claim (verified):** "routinely used to synchronise data-sets of sizes
  in the 10s of millions of elements"
  ([logperiodic.com/rbsr.html](https://logperiodic.com/rbsr.html)) — i.e. RBSR
  has production deployment at the 10s-of-millions scale, not just paper results.
- **rust-nostr** ships the [`negentropy` crate](https://crates.io/crates/negentropy)
  (`rust-nostr/negentropy`, version `0.5.0`, MIT, ~804K downloads, verified
  2026-05-29) and uses it inside the `nostr` SDK (`nostr` crate at
  `0.45.0-alpha.1`). This is the Rust embodiment Myrhiza would actually read.
- Other ports exist (Go, C#, Kotlin/`@nostr-dev-kit/sync`, JS) — the protocol
  is small enough to reimplement, which is itself a point in its favor.

## The deliberate non-choice: no persistent tree

Negentropy is notable for what it does *not* do. It keeps **one mutable sorted
source of truth** and recomputes range fingerprints on the fly, rather than
maintaining a persistent Merkle Search Tree or Prolly Tree. Hoyte's stated
rationale: where copy-on-write Merkle structures must snapshot, an RBSR
implementation over a mutable store need not — "RBSR can freely modify its
single source of truth without invalidating sync sessions started in the past"
([logperiodic.com/rbsr.html](https://logperiodic.com/rbsr.html)). This sidesteps
the unbounded-node-size and history-independence headaches that MSTs and Prolly
Trees inherit (see [structure-stability.md](structure-stability.md)).

The cost: each sync session does `O(log n)` fingerprint computations over the
live set, so a node needs an order-statistics index (e.g. a balanced search tree
keyed by `(timestamp, id)` with cached subtree fingerprints) to answer
range-fingerprint queries quickly. That index is an *implementation detail of
the local store*, not part of the wire protocol or the convergence contract.

## Implications for Myrhiza

Negentropy is the closest exemplar to the deferred `networking.md` §11.3 work,
named there as "negentropy-shape range reconciliation." It fits because:

- Myrhiza events are already `(HLC-timestamp, 32-byte EventHash)` shaped, so the
  `(timestamp, id)` sort order drops in directly. (HLC is materialization-only,
  not authority — but it is a fine *sort* key for a discovery protocol that
  doesn't affect the canonical topo-sort.)
- Negentropy is **transport-agnostic and authority-agnostic** — it discovers
  missing event IDs; the kernel still verifies signatures, `prev`/`deps`, and
  runs `state-apply`. It bolts onto the existing gossip plane without touching
  the convergence contract (`convergence.md` §4.3).
- Its "no persistent tree" stance matches Myrhiza's preference to avoid a second
  on-disk structure to keep crash-consistent.

See [lessons.md](lessons.md) for the validate/borrow synthesis and the honest
"v1 doesn't need this" framing.

## Sources

- [Negentropy (Doug Hoyte / hoytech)](https://github.com/hoytech/negentropy)
- [Negentropy Protocol V1 spec — Fingerprint Algorithm](https://github.com/hoytech/negentropy/blob/master/docs/negentropy-protocol-v1.md#fingerprint-algorithm)
- [NIP-77 Negentropy Syncing](https://nips.nostr.com/77)
- [NIP-77 source (nostr-protocol/nips)](https://github.com/nostr-protocol/nips/blob/master/77.md)
- [strfry relay](https://github.com/hoytech/strfry)
- [strfry negentropy docs](https://github.com/hoytech/strfry/blob/master/docs/negentropy.md)
- [Doug Hoyte — RBSR explainer (10s-of-millions scale; no-persistent-tree rationale)](https://logperiodic.com/rbsr.html)
- [`negentropy` crate (rust-nostr)](https://crates.io/crates/negentropy) — `0.5.0`, MIT, verified 2026-05-29
- [`nostr` crate](https://crates.io/crates/nostr) — verified 2026-05-29
- Myrhiza spec: `networking.md` §11.3, `convergence.md` §4.1–4.3
