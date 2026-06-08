**Date:** 2026-06-08
**Status:** active
**Subject:** Decisions for host.subscribe distilled from Matrix Sliding Sync — Validates / Avoid / Borrow

# Lessons for host.subscribe

Every bullet is targeted at Myrhiza's `host.subscribe`: a kernel-mediated
capability letting a sandboxed *interaction* component subscribe to a **window of
N gossip topics out of many**, receive per-topic state/event feeds, and aggregate
them in-sandbox into a multi-channel UI. Convergence stays per-topic in the
kernel; the subscription itself is non-deterministic and peer-local.

Read alongside [windowing.md](./windowing.md) (mechanics) and
[extensions-and-failures.md](./extensions-and-failures.md) (failure model).

## Validates

- **Windowed subscription is the right scale model.** Matrix proves in production
  that "user in thousands of rooms, K on screen" must be served by subscribing to
  a *window* of a sorted list, not the whole set. "User in 50 servers, 10 channels
  on screen" is the same problem. Cost should track the window, not the topic
  count.
- **Two-layer subscription (list-window + open-room) is real, not gold-plating.**
  Matrix needed both a cheap windowed list view *and* a richer explicit
  subscription for the open room (bigger timeline, full state). Myrhiza's UI will
  want the same split: a thin feed for the 50-channel sidebar, a fat feed for the
  one open channel. Model both from the start.
- **A renewable resume cursor beats per-message tokens.** Matrix's `pos` is a
  single, renewable, opaque cursor over a *continuous multi-topic stream* — each
  response yields the next cursor. This is exactly the gap in Myrhiza's existing
  single-use `*-submit` request-token pattern, which can't express per-message
  stream delivery. The cursor model is the resolution: one durable handle, stream
  of deliveries, not one token per message.
- **Keeping convergence out of the subscription is sound.** Sliding Sync never
  lets the client compute room ordering authority for *state* — the homeserver
  remains the source of truth; the window only selects *what to ship*. Mirrors
  Myrhiza's hard rule: the kernel runs per-topic deterministic state-apply;
  `host.subscribe` only selects which per-topic feeds reach the sandbox and never
  enters any canonical digest. `state-apply` rejecting `host.subscribe` is the
  correct analogue of "windowing is a view concern, not a state concern."
- **Explicit hydrate vs. silent teardown is the right asymmetry.** Matrix tears a
  room down silently (stop delivering, client drops it) but rehydrates with an
  explicit `initial` / `expanded_timeline` flag. Cheap teardown, unambiguous
  reload. Adopt this for channels scrolling off/on screen.
- **Per-room state selection (`required_state`) pays off.** Letting the subscriber
  declare *how much* per topic (lazy members vs. full state) is what made
  lazy-loading (MSC1227) the first scaling win. The working-set reduction was "less
  state per room" before it was "fewer rooms." MSC3575 expressed lazy members as a
  `$LAZY` sentinel state-key; MSC4186's `lazy_members` boolean is the direct
  descendant — the knob survived, the encoding got simpler.

## Avoid

- **Do not make the kernel maintain an authoritative ordered list per
  subscription.** This is MSC3575's deleted mechanism. The `SYNC`/`INSERT`/
  `DELETE`/`INVALIDATE` op stream that mutated a server-side ordered list in place
  was the single most expensive part and MSC4186 removed it: now the server sends
  a **count + changed items** and the **client orders locally**. For Myrhiza:
  deliver per-topic feeds + a cheap activity stamp; let the sandbox sort the
  sidebar. Do not build kernel-side ordered-list diffing.
- **Do not put topic ordering / "which is most active" into anything canonical.**
  Matrix kept sort order a *view* concern. Myrhiza must too: which topics are in
  the window, their delivery order, and any activity ranking are non-deterministic
  and peer-local — they must never touch a state-digest. (This is already a
  Myrhiza hard rule; Sliding Sync's history shows even the *server* shouldn't own
  this ordering, let alone the canonical state.)
- **Do not fan a destructive, once-delivered stream across multiple handles.**
  Matrix's concurrent-`conn_id` data-loss warning: if one connection consumes a
  to-device/E2EE message, another never sees it. If Myrhiza ever delivers
  consume-once data (key material, single-delivery control messages) over
  `host.subscribe`, a re-subscribe or a second handle must not silently swallow
  it. Prefer idempotent, replayable per-topic state feeds; if a consume-once
  channel is unavoidable, make ownership explicit, not racy.
- **Do not promise resume durability.** `pos` is advisory — the server expires it
  whenever convenient and the client cold-starts. Don't design a handle whose
  contract is "the kernel will always remember exactly where you were." In a
  gossip mesh, stale-handle and missed-message are *more* common; bake cold-start
  in as the normal path, not the exception.
- **Don't over-spec v1.** MSC3575 shipped server sorts, list ops, delta tokens,
  timeline filtering, notification counts — and MSC4186 *deleted* most of it after
  real usage. Ship `host.subscribe` at the MSC4186 altitude (window + per-topic
  feed + renewable cursor + a couple of extensions). Resist building the
  maximalist version; it will be the part you delete.

## Borrow

- **The `(conn_id, pos)` connection model, adapted to a capability handle.**
  - The **subscription handle** (unforgeable across the WASM boundary) plays
    `conn_id`'s role: a per-(peer, instance) connection identity that the sandbox
    holds and the kernel keys subscription state on. It is the natural place to
    hang the capability's scope (which topics), attenuation, and revocation —
    revoking the handle is "expire the connection," and the sandbox cold-starts or
    is denied.
  - A **renewable opaque cursor** plays `pos`'s role: returned with each delivery
    batch, echoed on the next poll, invalidatable by the kernel at will, with
    cold-start as the guaranteed fallback. This replaces single-use `*-submit`
    tokens for stream delivery while preserving the "kernel re-enters via an
    exported handler" submit-and-poll shape — the handler just receives a batch +
    next cursor instead of a one-shot completion.
- **Window-as-range, scroll-as-range-update.** Represent the on-screen set as a
  range over a subscriber-defined ordering and move it by sending a delta. Sticky
  parameters (the kernel remembers the current window; the sandbox sends only
  changes) keep the boundary chatter-free.
- **Batched multi-topic deltas per poll.** One delivery carries changes across all
  windowed topics (plus extensions), not one stream per topic. Fewer boundary
  crossings, natural backpressure via the poll cadence and `timeout`.
- **`required_state`-style per-topic hydration depth.** Let the manifest/subscription
  declare how much per topic (recent tail vs. fuller state), with a lazy default.
  This is the attenuation knob and the bandwidth knob at once.
- **The `initial` / `expanded` rehydrate flags.** When a topic re-enters the window
  or its depth grows, flag the delivery as a full (re)hydrate so the sandbox knows
  to replace, not patch.
- **The extensions pattern for orthogonal data.** Keep the core capability narrow
  (windowed per-topic state) and express orthogonal concerns (presence-like
  signals, control messages, key material) as explicitly-enabled, individually
  scoped extensions on the same handle — not crammed into the core feed, not a
  separate capability each.
- **The `RoomListService` two-phase load as the in-sandbox UX target.** Selective
  (load the visible few fast) → Growing (backfill the rest in background), with a
  small explicit state machine (`Init`/`SettingUp`/`Recovering`/`Running`/
  `Error`/`Terminated`) the UI can render as a sync indicator. This is the proven
  shape for "instant launch" over a large topic set, and it lives entirely in the
  sandbox over a `host.subscribe` feed.

## Sources

- https://github.com/matrix-org/matrix-spec-proposals/blob/kegan/sync-v3/proposals/3575-sync.md
- https://github.com/matrix-org/matrix-spec-proposals/blob/erikj/sss/proposals/4186-simplified-sliding-sync.md
- https://matrix-org.github.io/matrix-rust-sdk/matrix_sdk_ui/room_list_service/index.html
- https://matrix.org/blog/2024/11/14/moving-to-native-sliding-sync/
- https://matrix.org/blog/2024/10/29/matrix-2.0-is-here/
