**Date:** 2026-05-29
**Status:** active
**Subject:** SSB classic feed format — the signed single-author append-only log Myrhiza inherits

# The SSB feed: a single-author signed hash chain

Secure Scuttlebutt's core data structure is the **feed**: an append-only list of
messages published by one identity, identified by that identity's Ed25519 public
key (`@<base64-pubkey>.ed25519`). A feed *is* its author's key. This is the same
identity-equals-feed shape Myrhiza uses for its per-author chain
([convergence.md](../../specs/2026-05-09-myrhiza-master-design/convergence.md):
"author, sequence number (monotonic per-author starting at 1), `prev`").

## The classic message

A classic SSB message has exactly these fields, in a fixed order:

| Field | Meaning |
|---|---|
| `previous` | Message ID (hash) of the prior message in this feed; `null` for the first |
| `author` | The feed's Ed25519 public-key identity |
| `sequence` | Monotonic counter; `1` for the first message, then `previous.sequence + 1` |
| `timestamp` | Milliseconds since Unix epoch (author-asserted; **not trusted for ordering**) |
| `hash` | The literal string `"sha256"` — names the hash algorithm |
| `content` | The application payload, a JSON object with a required `type` field |
| `signature` | Ed25519 detached signature over the other six fields |

The **message ID** is `%<sha256-of-message>.sha256`. The `previous` field
references that ID, so the feed forms a hash-linked chain back to the genesis
message. The SSB spec calls this structure a *sigchain* and draws the blockchain
contrast explicitly: "Links are implemented via cryptographically secure hashes,
in that sense feeds behave mostly like blockchains. But whereas blockchains are
traditionally used to create a global, single source of truth, ssb chooses a
different approach. Instead of one single, global linked list for all data, each
ssb user has their own linked list." (spec.scuttlebutt.nz, Feeds and Messages).

Validation rule (verbatim shape): *if `previous` is `null` the sequence must be
`1`, else it must be one larger than the sequence of the message whose hash is
`previous`.* This is exactly Myrhiza's `EventDag::insert` invariant
(`seq == latest_seq + 1` and `prev == current_head`, convergence.md §4.4.1).

## The canonical-JSON signing surface (a cautionary detail)

SSB signs and hashes the message in a **canonical JSON serialization**: "for any
given message there is exactly one way to serialize it" — fixed two-space
indentation, specific float/escape rules, matching ECMA-262 `JSON.stringify`. The
infamous wrinkle: classic SSB pinned itself to *V8's* `JSON.stringify` behavior,
including its handling of large numbers and Unicode. Re-implementing validation
in another language (Rust's `ssb-validate`, `ssb-validate2-rsjs`) meant
**reproducing V8's stringify quirks byte-for-byte** to verify signatures. The
lesson is a determinism one: when the signed bytes are derived by re-serializing
structured data, the serializer becomes load-bearing protocol surface.

Myrhiza dodges this by signing **opaque payload bytes** and pinning the
state-digest encoding to `bincode 1.3.x` with an explicit Options chain
(convergence.md §4.3, [determinism.md](../../specs/2026-05-09-myrhiza-master-design/determinism.md)
§5.4) — see [lessons.md](lessons.md) Avoid §1. The SSB experience is the
cautionary tale that makes that pin worth its cost.

## What the feed buys you

- **Unforgeable provenance.** Only the keyholder can extend the feed; every
  message is independently verifiable from `(author, signature)`.
- **Order recovery despite unreliable delivery.** The `previous`/`sequence` chain
  lets a replica reassemble the total per-author order from out-of-order gossip.
- **Self-certification.** A message + its ID is verifiable without consulting any
  authority — the same property 2P-BFT-Log formalizes
  ([2p-bft-log.md](2p-bft-log.md)).

## What it does NOT buy you

Nothing in this format prevents the author from signing **two** valid messages at
the same `sequence` against the same `previous`. Both verify. Both are
self-certifying. That is the feed-fork problem — see
[ssb-fork-problem.md](ssb-fork-problem.md). The hash chain detects *external*
tampering perfectly and *author* equivocation not at all.

## Sources

- SSB protocol guide — [ssbc.github.io/scuttlebutt-protocol-guide](https://ssbc.github.io/scuttlebutt-protocol-guide/) (feed/message structure, canonical JSON).
- SSB spec, Feeds and Messages — [spec.scuttlebutt.nz/feed/messages.html](https://spec.scuttlebutt.nz/feed/messages.html).
- `ssb-validate2-rsjs` — [github.com/ssbc/ssb-validate2-rsjs](https://github.com/ssbc/ssb-validate2-rsjs) (Rust validation re-deriving the signing surface).
- Secure Scuttlebutt — [Wikipedia](https://en.wikipedia.org/wiki/Secure_Scuttlebutt).
- Myrhiza spec — [`convergence.md`](../../specs/2026-05-09-myrhiza-master-design/convergence.md), [`determinism.md`](../../specs/2026-05-09-myrhiza-master-design/determinism.md) §5.4.
