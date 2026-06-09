**Date:** 2026-06-08
**Status:** active
**Subject:** Sliding Sync — the extension framework and the failure / resync model

# Extensions and failure modes

Two things the window mechanics don't cover but `host.subscribe` will inherit:
how *side-channel* data rides the same connection, and what happens when the
connection breaks.

## The extensions framework

Sliding Sync deliberately keeps the core API to "rooms in a window." Everything
else — data that isn't per-room-timeline — is an **opt-in extension** attached to
the connection, each with its own request/response section and an `enabled` flag
(default `false`). Documented extensions:

- **`to_device`** — device-targeted messages (the transport for E2EE key
  distribution). Order- and delivery-sensitive (see data-loss note below).
- **`e2ee`** — encryption key/device metadata (`device_lists`,
  `device_one_time_keys_count`, fallback keys).
- **`account_data`** — client's own server-side storage (room tags, DM list).
- **`receipts`** — read markers.
- **`typing`** — typing indicators.
- (`presence` also appears in MSC3575.)

In MSC3575 each extension takes `lists` and `rooms` selectors controlling which
windows/subscriptions activate it:
`[]` = none, `["list_name"]` = that list, `["*"]` = all (the default). So an
extension can be scoped to "only the rooms in my current window" rather than
globally.

The design principle worth stealing: **the core capability is narrow (windowed
room state); orthogonal concerns are explicit, named, individually-enabled
extensions on the same connection** — not crammed into the core payload and not a
separate connection each.

## Batching of server→client deltas

A sync response batches everything changed since the client's last `pos` into one
payload: the affected `rooms` (changed `required_state` + `timeline_events`, with
`limited` / `num_live` / `prev_batch` for gappy timelines), the `lists[name].count`
deltas, and any enabled extension sections. The client long-polls with a `timeout`
(ms; `0` = return immediately); the server holds the request open until there is
something to send or the timeout elapses. One round-trip carries multi-room +
multi-extension deltas — not one stream per room.

## Failure modes — the resync model

This is the part `host.subscribe` must design for up front, because the analogous
failures (peer churn, dropped gossip, stale handle) are *more* frequent in a P2P
gossip mesh than in Matrix's client-server long-poll.

### 1. `pos` expiry / unknown `pos` → `M_UNKNOWN_POS`

The central failure primitive. The server can decide a connection is too stale or
too expensive to continue and respond **HTTP 400 with error code
`M_UNKNOWN_POS`**, when "the server thinks it would be faster for the client to
start from scratch." Same error if the client sends an old or invalid `pos` (e.g.
after reusing a token). Recovery: **drop `pos`, reissue with no `pos` → full
initial sync.** The connection is disposable; the room data is rebuildable.

Key property: **the resume token is advisory, not a guarantee.** The client must
*always* be able to cold-start. There is no "the server promised to remember me."

### 2. Connection / per-connection state cleanup

Server-side connection state is ephemeral and `pos`-gated: after seeing a request
with a given `pos`, the server may discard all per-connection state from before
it. A client that vanishes and returns much later finds its connection gone and
gets `M_UNKNOWN_POS` → cold start.

### 3. Response ≠ request (sticky-param races)

Because params are sticky and applied asynchronously, a response may reflect an
*earlier* parameter set than the client's latest request. MSC3575's `txn_id` lets
the client detect which param-generation a response corresponds to. The client
must tolerate "I asked for range `[20,39]` but this response is still for
`[0,19]`" and reconcile.

### 4. Concurrent-connection data loss

Explicit warning in MSC3575: using multiple connections (distinct `conn_id`s) on
one device "may result in data loss if used inappropriately." The hazard is
**destructive, once-delivered data** — chiefly `to_device` messages and E2EE
keys: if connection A consumes/acknowledges a to-device message, connection B
never sees it. Sliding Sync's answer is "don't fan destructive streams across
connections," not a dedup protocol.

### 5. Gappy timelines (`limited`)

When more events occurred than the `timeline_limit`, the room result is marked
`limited: true` with a `prev_batch` token; the client must backfill history via a
separate `/messages` paginate. The live stream and historical backfill are
**different mechanisms** — the subscription gives you the recent tail, not
arbitrary history.

## What this implies for a delivery token

Matrix's `pos` is the working model for stream delivery and the contrast Myrhiza
needs against single-use `*-submit` tokens:

- It is **renewable**: each response hands the next `pos`. Continuous stream, not
  one-shot.
- It is **disposable**: any `pos` can be invalidated; the client always falls back
  to cold start. No durability promise.
- It is **per-connection, not per-message**: one token resumes the *whole*
  multi-room + multi-extension feed, not an individual message.
- It is **opaque**: carries no app-meaningful structure across the boundary.

See [lessons.md](./lessons.md) for how this maps onto Myrhiza's capability handle.

## Sources

- https://github.com/matrix-org/matrix-spec-proposals/blob/erikj/sss/proposals/4186-simplified-sliding-sync.md
- https://github.com/matrix-org/matrix-spec-proposals/blob/kegan/sync-v3/proposals/3575-sync.md
- https://github.com/matrix-org/sliding-sync
- https://matrix.org/blog/2024/11/14/moving-to-native-sliding-sync/
