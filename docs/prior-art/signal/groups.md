**Date:** 2026-05-22
**Status:** active
**Subject:** Signal's private-groups infrastructure — zkgroup (KVAC anonymous credentials) and pairwise-Double-Ratchet group messaging. Why Signal hasn't moved to MLS.

# Signal groups — pairwise Double Ratchet + zkgroup credentials

Signal's group messaging is structurally different from group messaging in
MLS, and the design decision deserves scrutiny because Myrhiza will face
the same fork.

## Two layers

1. **zkgroup** (announced 2019-12-09 as "Technology Preview: Signal Private
   Group System"). A keyed-verification anonymous credential (KVAC) scheme
   for group state stored on Signal's server *without the server seeing
   member identities*. Members prove "I'm a member of group X" without
   revealing *who* they are to the server.

2. **Pairwise Double Ratchet fan-out** for the message-content cipher.
   Every group message is encrypted *N times* — once per recipient member
   — and delivered via N separate sealed-sender envelopes. The same
   Double Ratchet session as in 1:1 messaging is used for each recipient.

Concretely: a group with 50 members and one message → 50 individual
Double Ratchet ciphertexts, sent through sealed-sender delivery.
**Signal does NOT use MLS or any tree-based group key agreement.**

## zkgroup — the metadata privacy layer

The problem zkgroup solves: in a naive design, the server has to know
which users are members of which groups in order to deliver messages. That
group-membership graph is high-value metadata even when message contents
are E2E encrypted.

The KVAC scheme:

- Each user has an **AuthCredential** for their `(ACI, current_credential_
  expiration)` pair, issued by the server.
- The group has a **GroupMasterKey** known only to its members.
- Each member's ACI is encrypted with a key derived from the
  `GroupMasterKey` and stored in the server's group state record. The
  server sees ciphertexts, not ACIs.
- When a member wants to send a message to the group, they present a
  zero-knowledge proof to the server: "I know an AuthCredential and an
  ACI, and the encrypted ACI in this group's member list decrypts to my
  ACI." The server verifies the proof and accepts the message, never
  learning the ACI.
- Group admins can add/remove members by submitting proof+encrypted-ACI
  pairs; the server validates the proof but cannot enumerate the
  resulting member list.

The KVAC scheme was developed collaboratively with Melissa Chase and Greg
Zaverucha at Microsoft Research. The paper is "Signal Private Group System
and Anonymous Credentials Supporting Efficient Verifiable Encryption"
(Chase, Perrin, Zaverucha — CCS 2020).

### What zkgroup buys

- Server cannot enumerate group membership.
- Server cannot link a member to multiple groups (each membership uses an
  independent encrypted-ACI).
- Server cannot tell *who* is sending each message (combined with sealed
  sender).

### What zkgroup costs

- Proof generation is expensive: ~10-50ms per message on mobile.
  Rayon-parallelizable (libsignal uses it). For low-frequency group
  messages this is fine; for active conversations it adds up.
- Proof size: ~700 bytes per message — added to every send.
- Implementation complexity: zkgroup is *not* a standard primitive; it's a
  custom KVAC scheme with custom NIZK proofs over Curve25519 (the `poksho`
  crate). Hard to audit, hard to reimplement.

## Pairwise fan-out — why not tree-based groups?

Signal made the deliberate choice to encrypt group messages with N pairwise
Double Ratchet sessions instead of a shared group key. The trade-offs:

| Property | Pairwise fan-out (Signal) | Tree-based (MLS) |
|---|---|---|
| Send cost | `O(N)` ciphertexts per message | `O(1)` ciphertext per message |
| Server storage | Each ciphertext queued separately | Single ciphertext per epoch |
| Forward secrecy | Per-message (Double Ratchet's chain key) | Per-epoch (between commits) |
| Post-compromise security | Per-round-trip (Double Ratchet's DH step) | Per-commit (TreeKEM) |
| Adding a member | Linear: existing members initiate Double Ratchet with new member | Logarithmic: new leaf added, tree updated |
| Removing a member | Linear: existing members drop the removed member's session | Logarithmic: removed leaf blanked, tree updated |
| Wire complexity | Simple — same as 1:1 | Complex — Welcome, Commit, Application messages |
| Battle-tested | Yes (since 2014, at scale) | Yes (RFC 9420 published 2023-07; production deployments since 2022) |

Signal's choice has held up because:

1. Most Signal groups are small (median group size is ~5 members per
   Signal's published numbers; 99th percentile is ~100). At small N, the
   `O(N)` cost is acceptable.
2. The pairwise model inherits the Double Ratchet's per-message forward
   secrecy and PCS *for free*. MLS's per-epoch model is coarser; a single
   compromised epoch leaks the messages in that epoch.
3. The implementation complexity of Double Ratchet for 1:1 is already
   absorbed; adding tree-based groups would be net new code.

Signal has not publicly committed to MLS. Internal discussions exist (see
the MLS WG mailing-list archive for Signal-affiliated contributors), but
no public roadmap commits to a migration.

## What Signal calls a "group"

There are two kinds of multi-recipient messaging in Signal:

- **Direct multi-recipient sends** — what the UI calls "groups." Pairwise
  Double Ratchet fan-out + zkgroup credentials for membership.
- **Stories** (added in 2022) — broadcast to all your contacts or to a
  custom list. Same pairwise-fan-out cipher, no group identity object.
  Effectively N separate 1:1 sends from a UI perspective.

Group calls (audio/video) are a different protocol — Signal uses an SFU
("Selective Forwarding Unit") with MRP (Media Routing Protocol). The audio/
video streams are encrypted via SRTP + an SFU-distributed key. Out of
scope for this corpus, but worth knowing the group-call cryptography is
*not* zkgroup-based.

## Implications for Myrhiza

- **For 1:1 DMs in Myrhiza, the pairwise Double Ratchet pattern is
  Signal-validated and stays Signal-validated even when MLS is in the
  mix for groups.** Don't drop Double Ratchet for 1:1 just because
  you're adopting MLS for groups.
- **For groups in Myrhiza, the choice is genuine: pairwise fan-out
  (simple, well-understood) vs MLS (richer, more standardized).** Signal
  itself sticks with pairwise. MLS's load-bearing advantage is *large
  groups* (>~100 members) — if Myrhiza's expected group sizes are
  small, pairwise is honestly fine.
- **zkgroup-style anonymous credentials are research-grade for P2P.**
  In Signal, the server is the verifier; in Myrhiza there is no central
  verifier. Translating zkgroup to "any member of the group can verify"
  is an open problem — multi-show anonymous credentials in a distributed
  setting are an active research area, not a deployed pattern.
- **Sealed sender + zkgroup credentials together hide both
  who-sent-what and who's-in-the-group.** If Myrhiza wants this dual
  property in a P2P setting, both layers need translation. The naive
  P2P design (everyone broadcasts encrypted to a topic, members
  decrypt) leaks group membership to network observers via gossip
  traffic patterns. Solving this requires either cover traffic, or
  rotating topic IDs (see Willow's open-problems on topic-ID rotation
  through dumb relays — `prior-art/willow/open-problems.md:192-218`).
- **Group call cryptography is a separate problem.** Signal uses
  SRTP+SFU; MLS doesn't naturally extend to media streams either.
  Myrhiza-side group calls are out of scope here.

## Sources

- "Technology Preview: Signal Private Group System" (2019-12-09): <https://signal.org/blog/signal-private-group-system/>
- "Signal Private Group System and Anonymous Credentials Supporting Efficient Verifiable Encryption" (Chase, Perrin, Zaverucha — CCS 2020): <https://eprint.iacr.org/2019/1416>
- zkgroup crate in libsignal: <https://github.com/signalapp/libsignal/tree/main/rust/zkgroup>
- Comparator: `prior-art/mls/protocol.md` (TreeKEM-based group state)
- Comparator: `prior-art/mls/comparisons.md` §pairwise-vs-tree
- Willow open-problems cross-link: `prior-art/willow/open-problems.md:192-218` (topic-ID rotation)
