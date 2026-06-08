**Date:** 2026-06-08
**Status:** active
**Subject:** Matrix Sliding Sync — windowed multi-room subscription as the scale model for host.subscribe

# Matrix Sliding Sync

Matrix is a federated real-time messaging protocol. A client syncs with its
homeserver to learn about new events across every room it is in. The original
`/sync` returned *all* rooms in one long-poll, which collapsed at scale (a user
in thousands of rooms could wait minutes on initial sync). **Sliding Sync** is
Matrix's redesign: the client subscribes to a *sliding window* of the N most
relevant rooms out of potentially thousands, scrolls that window as the user
moves through the room list, and layers explicit per-room subscriptions on top
for the room currently open.

This is the closest production analogue to Myrhiza's `host.subscribe` problem:
**a user is in 50 servers but only 10 channels are on screen.** Matrix has spent
~4 years (2021–2025) iterating the windowing, connection, and teardown model in
production, then *deleting* half of it once real usage was known. That deletion
history is the most valuable part of this corpus.

## Key facts

| Fact | Value | Source |
|---|---|---|
| Original sync | `/sync` long-poll, returns all rooms | spec |
| Lazy-loading members | MSC1227, `lazy_load_members` filter | [MSC1227](https://github.com/matrix-org/matrix-doc/issues/1227) |
| Sliding Sync proposal | **MSC3575** "Sliding Sync (aka Sync v3)", author Kegan Dougal (kegsay) | [PR #3575](https://github.com/matrix-org/matrix-spec-proposals/pull/3575) |
| Simplified Sliding Sync | **MSC4186**, author Erik Johnston | [PR #4186](https://github.com/matrix-org/matrix-spec-proposals/pull/4186) |
| Proxy implementation | `matrix-org/sliding-sync` (Go), "Sync v3" | [repo](https://github.com/matrix-org/sliding-sync) |
| Proxy status | **archived 2025-11-17**; superseded by MSC4186 | repo notice |
| Native server support | **Synapse 1.114.0** (2024-09-02): "Enable native sliding sync support (MSC3575 and MSC4186) by default" | [Synapse release](https://github.com/element-hq/synapse/releases) |
| Client SDK | `matrix-rust-sdk`: `matrix_sdk::sliding_sync` (low) + `matrix_sdk_ui::room_list_service` (high) | [docs](https://matrix-org.github.io/matrix-rust-sdk/matrix_sdk_ui/room_list_service/index.html) |
| Primary client | Element X (iOS/Android), built on matrix-rust-sdk SlidingSync; "the only Matrix 2.0 client" per the 2024-10-29 blog | [Matrix 2.0](https://matrix.org/blog/2024/10/29/matrix-2.0-is-here/) |
| rust-sdk drops MSC3575 | PR #4531 removed MSC3575 support (Element X cutover 2025-01-17) | [PR #4531](https://github.com/matrix-org/matrix-rust-sdk/pull/4531) |

## Table of contents

- [sync-evolution.md](./sync-evolution.md) — `/sync` → lazy-loading → MSC3575 → MSC4186; why each step happened and what each fixed.
- [windowing.md](./windowing.md) — lists, ranges, room subscriptions, the sticky `conn_id`/`pos` connection model, window teardown/rehydrate, the SDK's two-phase `RoomListService`.
- [extensions-and-failures.md](./extensions-and-failures.md) — the extension framework (to-device, e2ee, account-data, receipts, typing) and the failure modes (`M_UNKNOWN_POS`, connection expiry, concurrent-connection data loss).
- [lessons.md](./lessons.md) — **the decision file.** Validates / Avoid / Borrow, every bullet tied to `host.subscribe`.
- [open-problems.md](./open-problems.md) — what Sliding Sync structurally does *not* solve; Myrhiza's inherited risk list.

## Canonical reading order

1. `sync-evolution.md` — the *why* and the deletion history.
2. `windowing.md` — the *how* of windows and connections.
3. `extensions-and-failures.md` — the edges.
4. `lessons.md` — what to copy / avoid for `host.subscribe`.
5. `open-problems.md` — residual risk.

## Glossary

- **Sliding window / range** — `[start, end]` index pair (0-indexed, inclusive) into a server-sorted room list; the client sees only rooms whose rank falls in the range. Scrolling moves the range.
- **List** — a server-maintained, filtered, sorted view of rooms the client subscribes to a window of.
- **Room subscription** — an explicit by-ID subscription to one room (the open room), independent of any list window.
- **`conn_id`** — opaque per-(user, device) connection identifier; lets one device hold multiple independent sliding-sync connections.
- **`pos`** — opaque ephemeral position/resume token returned each response; the next request must echo it. Server may invalidate it at any time.
- **Sticky parameters** — config (ranges, filters, `required_state`) persists server-side across requests until explicitly changed; the client sends deltas, not full requests.
- **`required_state`** — the set of room state events the client wants per room, with wildcards and (in MSC3575) `$LAZY`/`$ME` specials; MSC4186 replaced `$LAZY` with a `lazy_members` boolean.
- **Extension** — opt-in side-channel (to-device, e2ee, receipts, typing, account-data) attached to a sliding-sync connection.
- **`M_UNKNOWN_POS`** — error the server returns to force a client to discard its `pos` and restart the connection.
