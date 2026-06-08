**Date:** 2026-06-08
**Status:** active
**Subject:** Sliding Sync windowing — lists, ranges, room subscriptions, the conn_id/pos connection model, teardown/rehydrate

# Windowing

This is the mechanical core: how "subscribe to a window of N rooms out of
thousands" actually works on the wire, and how rooms scroll in and out.

## Two layers: lists (windows) + room subscriptions (the open room)

Sliding Sync separates two distinct subscription concerns that Myrhiza must also
separate:

1. **Lists** — *the room list pane.* The client declares one or more named lists,
   each a server-side **filtered + sorted** view of rooms, and subscribes to a
   **range** of it. The range is what's on screen.
2. **Room subscriptions** — *the open room.* An explicit by-ID subscription to a
   specific room, independent of any list. Used for the room the user has open
   (which may be richer than its list entry: bigger `timeline_limit`, full state)
   and for following permalinks to rooms outside any window.

A room can match both. Configs then **merge by superset**: max `timeline_limit`,
union of `required_state` (MSC4186 wording).

## Ranges: the sliding window

```jsonc
// MSC3575 form (required_state uses the $LAZY sentinel)
"lists": {
  "rooms": {
    "ranges": [[0, 19]],
    "filters": { "is_dm": false },
    "required_state": [ ["m.room.name", ""], ["m.room.member", "$LAZY"] ],
    "timeline_limit": 10
  }
}
```

MSC4186 reworked `required_state`: lazy members are no longer a `$LAZY`
sentinel but a `lazy_members: true` boolean inside the `required_state` request
(`{ "required_state": { "include": [...], "lazy_members": true } }`).

- Ranges are **0-indexed, inclusive** (`[0, 19]` = top 20 rooms by the list's
  sort). Multiple ranges are allowed (e.g. `[[0,9],[40,49]]` for two viewport
  chunks).
- In MSC3575 the list is **server-sorted** (server sorts `by_recency`,
  `by_notification_level`, `by_name`); **MSC4186 dropped server-driven ordering**
  — `lists[name].count` gives the total, and the client orders locally. The client
  may sort using `bump_stamp`, an activity-ordering integer the server provides
  ("Greater means more recent"), though the spec is explicit that the list is
  **not** server-ordered by it: *"Rooms are not ordered by `bump_stamp`."*
- **Scrolling = changing the range.** User scrolls down → client updates the range
  to `[[20, 39]]`. Because parameters are sticky, the client sends only the
  changed range; the server starts streaming the newly-windowed rooms.

### How MSC3575 mutated the list: list ops (deleted in MSC4186)

MSC3575 kept a server-authoritative ordered list per connection and mutated the
client's copy with operations — worth recording because **MSC4186 deleted this
whole mechanism**, which is the load-bearing lesson:

- `SYNC` — set a *range* of entries; client discards prior knowledge of that range.
- `INSERT` — single entry at an index; neighbors shift.
- `DELETE` — remove a single entry.
- `INVALIDATE` — remove a *range* of entries (e.g. the window the user scrolled
  away from).

In MSC4186 there are no ops. The server returns changed rooms + a count; the
client re-derives the ordered list itself. The expensive thing — maintaining and
diffing an authoritative ordering per connection — was removed.

## `required_state`: state per room, with lazy members

Per room (list or subscription), the client declares which **state events** it
wants, with wildcards and specials. In **MSC3575** these are `[type, state_key]`
tuples with sentinel state-keys:

- `["m.room.name", ""]` — a specific state event.
- `["m.space.child", "*"]` — all state of a type (`*` state_key).
- `["m.room.member", "$LAZY"]` — lazy members: only senders of timeline events +
  membership targets (descendant of MSC1227).
- `["m.room.member", "$ME"]` — your own membership only.
- `["*", "*"]` — all state (additional entries then act as *filters*).

**MSC4186 dropped the `$LAZY`/`$ME` sentinels.** Lazy-loading becomes a
`lazy_members: true` boolean on the `required_state` request rather than a magic
state-key; the spec defaults it to `false`.

This is the "what to hydrate per room" knob. Window membership controls *which*
rooms hydrate; `required_state` controls *how much* per room.

## The connection model: `conn_id` + `pos` (sticky, stateful)

The single most important borrowable piece. Sliding Sync connections are
**stateful and sticky**, unlike v2's stateless `since` token.

- **`conn_id`** — opaque, scoped to `(user, device)`. Lets one device hold
  *multiple independent* sliding-sync connections (browser tab + push process +
  one-shot request) without interference. Required if >1 concurrent. MSC3575 caps
  it at **16 chars** ("due to it being required with every request"); MSC4186
  describes it only as "an optional string to identify this connection" with no
  stated length limit.
- **`pos`** — opaque ephemeral position/resume token returned in every response;
  the next request must echo the last `pos`. It is the *only* resumption handle.
  The server may invalidate `pos` at any time (see
  [extensions-and-failures.md](./extensions-and-failures.md)).
- **Sticky parameters** — ranges, filters, `required_state`, `timeline_limit`
  persist server-side; the client sends only deltas. MSC3575 used a client
  `txn_id` echoed back to confirm a parameter change was applied; the server may
  return data that doesn't match the latest sent request (it sends the most
  recent *unacknowledged* state).
- **One in-flight request per `conn_id`.** Concurrent requests on the same
  connection have ambiguous ordering and are forbidden. To change params, cancel
  the outstanding long-poll and reissue.
- **`pos`-gated cleanup.** "Once a server has seen a request with a given `pos`,
  the server may clean up any per-connection state from before that `pos`." The
  server cannot assume a response was received until the client comes back with
  that response's `pos`.

The `pos` token is doing the job Myrhiza's single-use `*-submit` request-tokens
*can't*: it is a **renewable, long-lived cursor over a continuous stream**, not a
one-shot. Each response hands you the next cursor. That is the shape a
per-message stream delivery needs.

## Teardown / rehydrate when a channel scrolls off-screen

What happens to a room when it leaves the window:

- **Leaving the window:** in MSC3575 the server `INVALIDATE`s that range and stops
  pushing updates for those rooms; the client drops/collapses their data. In
  MSC4186 the room simply stops appearing in responses (it's outside the count
  range the client cares about). Either way: **no canonical teardown event — the
  client just stops being told about it and reclaims the memory.**
- **Re-entering the window:** the room is re-sent. `RoomResult.initial = true`
  marks "first sent on this connection or on reset," telling the client to treat
  it as a fresh hydrate rather than a delta. MSC3575 offered an optional
  `delta_token` so re-entry could skip re-sending unchanged `required_state` /
  timeline by event-ID comparison; the server may expire it and fall back to full
  re-send. MSC4186 leans on the `initial` flag instead (with `bump_stamp` offered
  only as a client-side sort aid, not the list order).
- **Config grows (open a room):** if `timeline_limit` increases, MSC4186 re-sends
  the latest N events with `expanded_timeline: true` even if some were sent
  before — explicit "rehydrate richer" signal.

The principle: **teardown is silent and local (just stop delivering + drop);
rehydrate is an explicit `initial`/`expanded` flag so the client knows it's a full
reload, not an incremental patch.**

## SDK shape: `RoomListService` two-phase load

`matrix-rust-sdk` (`matrix_sdk_ui::room_list_service`) is the production
abstraction Element X uses, and it shows the real access pattern:

- A single sliding-sync list named `all_rooms` (`ALL_ROOMS_LIST_NAME`).
- **Phase 1 — `SlidingSyncMode::Selective`:** a small range loads the first screen
  of rooms *fast* (instant launch).
- **Phase 2 — `SlidingSyncMode::Growing`:** transitions to pull the remaining
  rooms in background batches. The rust-sdk docs call this "empirically satisfying
  to provide a fast and fluid user experience for a Matrix client."
- `RoomList::entries_with_dynamic_adapters()` yields a **stream of rooms** that is
  locally sorted, filterable, and the filter can change over time — i.e. the
  app-facing UI does the ordering/filtering, matching MSC4186's "client owns the
  list."
- It's an **opinionated state machine** (`State` enum): `Init`, `SettingUp`,
  `Recovering`, `Running`, `Error`, `Terminated`. The app subscribes to `state()`
  to drive sync indicators.

## Sources

- https://github.com/matrix-org/matrix-spec-proposals/blob/kegan/sync-v3/proposals/3575-sync.md
- https://github.com/matrix-org/matrix-spec-proposals/blob/erikj/sss/proposals/4186-simplified-sliding-sync.md
- https://matrix-org.github.io/matrix-rust-sdk/matrix_sdk_ui/room_list_service/index.html
- https://matrix-org.github.io/matrix-rust-sdk/matrix_sdk/sliding_sync/index.html
- https://github.com/matrix-org/matrix-rust-sdk/issues/1911
