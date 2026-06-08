**Date:** 2026-06-08
**Status:** active
**Subject:** What Sliding Sync structurally does NOT solve — Myrhiza's inherited risk list

# Open problems

Sliding Sync solves windowed subscription *in a client-server world with a single
trusted, authoritative homeserver per account*. Most of what it leaves unsolved is
exactly what Myrhiza's P2P, sandboxed, gossip-transport setting reintroduces. Each
item below is a risk `host.subscribe`'s spec must address explicitly — Matrix
gives no answer to copy.

## 1. There is no trusted central server — convergence is harder

Sliding Sync assumes the homeserver is the single source of truth: it owns the
canonical room state, the authoritative ordering, and the resume state. Myrhiza
has **no such central authority**; convergence is achieved per-topic by
independent deterministic replay on each peer. Sliding Sync therefore offers *zero*
guidance on the genuinely hard part — making per-topic state converge across peers.
It only addresses *delivery of an already-authoritative feed to a UI*. Keep the
analogy scoped to delivery; do not let it leak into convergence design.

## 2. Resume/ordering state lives server-side — who holds it in Myrhiza?

`conn_id` + `pos` connection state is held by the homeserver and discarded at will.
In Myrhiza the kernel is the local analogue, but: per-topic feeds come from a
**gossip mesh** (iroh-gossip, HyParView + Plumtree), not a single backing store.
"Cold-start the subscription" means re-establishing topic membership and
re-pulling state across a churning peer set — far more expensive and failure-prone
than re-hitting one homeserver. Matrix's "just expire `pos` and cold-start" is
cheap *because* the server still has everything; Myrhiza's cold-start may need a
backfill protocol (cf. the existing direct-stream pull for stale-network backfill).
Risk: the resume token's cold-start fallback is not free here.

## 3. Per-message stream delivery is not actually solved by `pos`

`pos` resumes a *connection*, not individual messages, and Matrix explicitly
treats consume-once data (to-device, E2EE keys) as dangerous across connections,
with a bare "may result in data loss" warning and no dedup/ack protocol. Matrix
*sidesteps* per-message guarantees rather than solving them: replayable room state
is idempotent, so dropping a delta is recoverable on next sync; only the
genuinely-once channels are fragile, and those are just declared hazardous. Myrhiza
inherits the open problem the brief names — single-use tokens don't fit
per-message streams — and Sliding Sync does **not** close it. The cursor model
helps for *idempotent state feeds*; any *once-delivered* control/key channel needs
its own ack/dedup design that Matrix never built.

## 4. Out-of-order / lossy transport

Sliding Sync rides ordered, reliable HTTP long-poll against one server. iroh-gossip
(Plumtree) gives epidemic, eventually-consistent, **unordered and lossy** delivery
per topic. Sliding Sync's batching, `limited`/`prev_batch` gappy-timeline handling,
and `num_live` counts all assume the server can present a clean ordered tail.
Myrhiza's kernel must produce that clean per-topic view *out of* gossip before
`host.subscribe` can present anything analogous — Matrix assumes that layer already
exists.

## 5. Capability scoping, attenuation, revocation are entirely out of scope

Sliding Sync has no capability model. Access control is "the homeserver checks the
user is joined" (`required_state` permission checks). There is no unforgeable
handle, no attenuation, no third-party-delegable subscription, no revocation
primitive beyond "the server stops answering." Myrhiza's core requirements —
manifest-declared per-topic scope, attenuable + revocable capability, unforgeable
handle across the WASM boundary — get **no prior art** from Sliding Sync. This must
be designed from Myrhiza's capability model, not borrowed.

## 6. Sandbox boundary doesn't exist in Matrix

In Matrix the client is fully trusted with everything the server sends; the
matrix-rust-sdk runs in-process with full access. Myrhiza's subscriber is a
*sandboxed* WASM interaction component that must reach the feed only through a
declared host import. The marshalling of a streaming, windowed, multi-topic feed
across the WASM ABI — handle representation, batch encoding, backpressure, the
exported re-entry handler — is a Myrhiza ABI-design problem with no Sliding Sync
analogue. (Adding `host.subscribe` is itself an ABI change.)

Backpressure specifically has no Matrix analogue: Sliding Sync's long-poll
backpressure is HTTP flow control between client and server, with a fully-trusted
in-process client that always drains. Myrhiza must answer what happens when the
**sandbox can't drain a batch as fast as gossip produces** — queue between kernel
and sandbox, drop-and-flag-stale, or block the producer. The poll-cadence pacing
borrowed from `pos` (see [lessons.md](./lessons.md)) bounds request rate but does
not bound a backlog the sandbox never picks up; that overflow case is unsolved
upstream.

## 7. Native + browser parity

Matrix achieves this by shipping matrix-rust-sdk compiled to WASM, but the
*subscription* still terminates at an HTTP server reached identically from both. In
Myrhiza, native (Wasmtime) vs. browser (jco transpile) must both drive the same
gossip transport through the same host import. Sliding Sync gives no guidance on a
transport that itself differs by environment; the kernel must abstract iroh-gossip
identically across both, beneath `host.subscribe`.

## 8. Topic IDs are content-addressed, not human-meaningful

Matrix room IDs (`!room:server`) are opaque but server-scoped and discoverable via
the homeserver's room directory and the sorted list itself. Myrhiza topic IDs are
content-addressed BLAKE3 hashes with no central directory. "Which topics exist /
should be in my list" has a server answer in Matrix (the homeserver enumerates and
sorts the user's rooms); in Myrhiza there is **no enumerator** — topic discovery,
naming, and the membership of the candidate set the window slides over are
unsolved upstream of `host.subscribe`. The window needs a list to slide over;
Matrix's server provides that list, Myrhiza must source it elsewhere.

## 9. Non-determinism containment is assumed, not enforced

Sliding Sync's per-peer view (which rooms, what order, delivery timing) is
trivially non-deterministic and that's fine — there's no convergence requirement on
the *view*. Myrhiza shares the non-determinism but must *enforce* that it never
leaks into canonical state (`state-apply` must reject `host.subscribe`). Matrix had
no reason to build this firewall, so there's no pattern to copy; Myrhiza must
construct the enforcement boundary itself.

## Summary risk table

| Risk | Matrix's answer | Myrhiza must build |
|---|---|---|
| Cross-peer convergence | central server owns truth | per-topic deterministic replay (separate concern) |
| Resume after long absence | server still has state, cheap | backfill over churning mesh |
| Per-message once-delivery | sidestepped; "may lose data" | ack/dedup for any consume-once channel |
| Lossy/unordered transport | reliable ordered HTTP | clean per-topic view out of gossip |
| Capability scope/attenuate/revoke | none (join check only) | full capability model + handle |
| Sandbox ABI for a stream | none (in-process SDK) | WASM ABI for windowed stream |
| Native+browser transport | same HTTP both sides | abstract iroh-gossip both sides |
| Topic discovery/enumeration | homeserver room directory | discovery for content-addressed IDs |
| Keep non-determinism out of state | not required | enforced firewall (`state-apply` rejects) |

## Sources

- https://github.com/matrix-org/matrix-spec-proposals/blob/erikj/sss/proposals/4186-simplified-sliding-sync.md
- https://github.com/matrix-org/matrix-spec-proposals/blob/kegan/sync-v3/proposals/3575-sync.md
- https://github.com/matrix-org/sliding-sync
- https://matrix.org/blog/2024/11/14/moving-to-native-sliding-sync/
