**Date:** 2026-05-22
**Status:** active
**Subject:** AT Protocol compared to ActivityPub, Nostr, Matrix, Holochain, and Willow

# Comparisons

A focused walkthrough of how atproto differs from the other federated/p2p social/identity protocols Myrhiza considers as design-space neighbors. Each comparison is short — the goal is to surface the load-bearing design choice that distinguishes atproto, not to do a full feature-by-feature review.

## atproto vs ActivityPub (Mastodon, Pleroma, Misskey)

| Axis | AT Protocol | ActivityPub |
|---|---|---|
| **Federation shape** | Tiered: PDS / Relay / AppView | Mesh: every instance talks to every other |
| **Identity** | DID (durable, portable across hosts) | Handle bound to instance (`@user@instance.example`) |
| **Account migration** | Lossless via rotation key + CAR | Lossy — followers must re-follow |
| **Data model** | Typed records in content-addressed repo (MST) | Typed activities, server-stored, no canonical hash |
| **Wire protocol** | XRPC over HTTPS + WebSocket firehose | ActivityPub HTTP + inbox/outbox push |
| **Schema system** | Lexicon (DNS-rooted, NSID, strict additive evolution) | JSON-LD (W3C semantic-web roots) |
| **Centralization profile** | ~99% on Bluesky-operated tiers | Thousands of instances, no >5% concentration |
| **E2E** | None native | None native (some forks add it) |

**The key distinction**: atproto's identity layer survives instance migration; ActivityPub's doesn't. This is the headline difference and it's the one that matters most for Myrhiza, because the Mastodon-style "your identity dies if your instance dies" failure mode is exactly what Myrhiza needs to avoid for the multi-device-identity problem.

**The structural cost**: atproto needs the DID registry (`plc.directory`) to make this work, and that registry is a centralization vector ActivityPub doesn't have. Mastodon has no equivalent because it doesn't try to provide portable identity — and that's a feature for the federation-diversity story even though it's a bug for the user-experience story.

## atproto vs Nostr

| Axis | AT Protocol | Nostr |
|---|---|---|
| **Identity** | DID (rotation key + signing key separated) | Single secp256k1 keypair (`npub` / `nsec`) — no rotation |
| **Data model** | Typed records, signed commits, content-addressed | Typed events, signed, replicated to many relays |
| **Storage** | PDS-hosted repos (one canonical host per user) | Many independent relays; no canonical home |
| **Discovery** | Relay aggregates the firehose from PDSes | Client picks which relays to read from |
| **Schema system** | Lexicon (typed, validated, code-genned) | NIP-N specs (lighter convention, less strict typing) |
| **Centralization profile** | High (Bluesky tier dominance) | Low (no equivalent of "the" Relay) |
| **E2E** | None native | NIP-44 / NIP-17 for DMs (per-event encryption, no MLS-style group state) |

**The key distinction**: Nostr is much closer to a P2P system — there's no "the" Relay, no central identity registry, no concept of a user having "a home host." But it pays for this with weaker identity (no rotation = key loss is terminal) and weaker structure (no canonical hash of a user's full data, no typed schema validation).

**Lesson for Myrhiza**: Nostr's "single keypair, no rotation" is the cautionary tale. Plan B-2's split of `PeerKeypair` and `AuthorKeypair` is already past Nostr; atproto's rotation-key-priority list is the next step beyond that. Don't ship the Nostr model and discover it's terminal later.

## atproto vs Matrix

| Axis | AT Protocol | Matrix |
|---|---|---|
| **Identity** | DID (portable) | MXID (`@user:homeserver.example`) — bound to homeserver |
| **Federation shape** | PDS / Relay / AppView | Mesh of homeservers; rooms replicated across all participant servers |
| **Data model** | Typed records in CAR-exportable repo | Room state as a hash-linked DAG; per-room replication |
| **E2E** | None native; Germ DM overlays MLS | Megolm (Olm-derived; pre-MLS) for E2E rooms |
| **Schema system** | Lexicon | Room state events with `m.*` namespace, loosely typed |
| **Centralization** | ~99% Bluesky tier | Less concentrated; matrix.org is largest single homeserver but well under 50% |
| **Sync model** | PDS push to Relay; Relay push to AppViews | Federated room state DAG; servers fetch missing events as needed |

**The key distinction**: Matrix has *better* E2E (it's been deployed for years) and *better* per-room replication shape (DAG-based, multi-server-authoritative). Atproto has *better* identity portability and *better* schema typing.

The two systems answered different questions. Matrix asked "how do groups of people communicate securely across many servers?" Atproto asked "how do users have a portable identity that survives host changes?" If Myrhiza wants to combine the two — portable identity AND multi-writer secure messaging — neither protocol is a direct fit. The combination shape is closer to "Matrix room model + atproto-style DID identity + Willow's state-apply" which is essentially what Myrhiza is designing.

## atproto vs Holochain

See `prior-art/holochain/` for the canonical Holochain reference. Compare:

| Axis | AT Protocol | Holochain |
|---|---|---|
| **Topology** | Federated client-server (tiered) | Peer-symmetric |
| **Identity** | DID + rotation/signing key separation | Per-agent Ed25519 keypair; **no multi-device identity story** |
| **Data validation** | PDS validates against Lexicon schemas | Every peer validates entries against per-DNA integrity zomes |
| **Storage** | PDS hosts user's repo authoritatively | DHT shards entries by content hash; every peer holds a slice |
| **App model** | App = AppView + UI; reads from substrate | App = DNA + UI; runs locally on every peer |
| **Centralization profile** | High | Low (no central operator) |
| **Multi-device identity** | Solved (DID + rotation keys + migration) | **Not solved** (cross-references this exact gap in atproto's identity model as the place to look for the answer) |

**The key insight**: this comparison is precisely what `prior-art/willow/open-problems.md` §"Multi-device identity" calls out. **Holochain has the right peer-symmetric topology but no identity story; atproto has the right identity story but the wrong topology.** Myrhiza's challenge is taking atproto's identity model into Holochain's topology — which requires figuring out who plays the `plc.directory` role in a peer-symmetric setting.

The candidate answers (each surfaced in [open-problems.md](open-problems.md)):

- **Replicated state-apply.** Every peer maintains a copy of the identity operation log; recovery rules apply via Myrhiza's deterministic state-apply.
- **MLS group state for identity.** The user's "identity group" is an MLS group where members are the user's devices; key rotation is an MLS commit.
- **Hybrid.** Long-term DID-like identifier rooted in a peer-replicated registry; per-room/per-app MLS for confidentiality.

Holochain has no comparable mechanism. Its "multi-device identity is an unsolved problem" admission is what makes atproto's rotation-key model the operative prior art.

## atproto vs Willow (and Myrhiza)

See `prior-art/willow/` for the Willow reference. Compare:

| Axis | AT Protocol | Willow |
|---|---|---|
| **Identity** | DID + rotation/signing key split | Per-user Ed25519, one key |
| **Multi-device** | Solved via PDS migration + rotation keys | Not solved; Plan B-2 is in flight |
| **Topology** | Federated client-server | Peer-symmetric (single-author chat product as of 2026-05) |
| **Schema system** | Lexicon (typed, NSID, additive) | Bincode + Rust types (typed but not portable) |
| **E2E** | None native | ChaCha20-Poly1305 per-channel keys with `RotateChannelKey` events |
| **Storage** | MST + CAR (content-addressed, deterministic) | DAG of events keyed by hash (similar, deterministic state-apply) |
| **Substrate** | TypeScript + Go reference impls | Rust runtime, single-author chat product |

**The key distinction**: Willow is upstream of Myrhiza's master spec PR #636 and is in the same design family. Atproto's relevant contributions are:

1. **The rotation-key model** — Plan B-2's `AuthorKeypair`/`PeerKeypair` split could borrow the priority-list shape.
2. **The Lexicon-style schema system** — Myrhiza needs a snapshot-portability schema and atproto's strict-additive-evolution discipline is the deployed reference.
3. **The MST as deterministic content-addressed storage** — possibly relevant for Myrhiza's snapshot canonicalization.

Atproto's federation tier is **not** relevant; Myrhiza is structurally peer-symmetric and rejects the centralization-prone tier model.

See [lessons.md](lessons.md) for the validates/avoid/borrow synthesis.

## Sources

- AT Protocol vs ActivityPub: <https://docs.bsky.app/docs/advanced-guides/atproto> and various community comparisons
- AT Protocol vs Nostr: blog comparisons across both communities
- Matrix protocol: <https://spec.matrix.org/>
- ActivityPub: <https://www.w3.org/TR/activitypub/>
- Nostr NIPs: <https://github.com/nostr-protocol/nips>
- Holochain prior art: [`prior-art/holochain/`](../holochain/)
- Willow prior art: [`prior-art/willow/`](../willow/)
- MLS prior art: [`prior-art/mls/`](../mls/)
