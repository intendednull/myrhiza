**Date:** 2026-06-08
**Status:** active
**Subject:** Matrix sync evolution — `/sync` → lazy-loading → MSC3575 → MSC4186, and why each step happened

# Sync evolution

The value here is the *trajectory*: a full-state long-poll that didn't scale, two
incremental patches, a maximalist redesign (MSC3575), and then a deliberate
*pruning* of that redesign (MSC4186) once production usage was known. Myrhiza is
designing `host.subscribe` at roughly the MSC3575 moment — this history is a
preview of which features will and won't survive contact with real apps.

## Stage 0 — `/sync` v2 (full-state long-poll)

The original endpoint: client long-polls `/sync`, server returns **every room**
the user is in, plus a `since` token for the next incremental poll. Simple and
stateless on the client side (the `since` token carries all resumption state),
but it scales with total account size, not with what the user is looking at:

- All rooms are always returned; the client cannot say "I only care about 10."
- Initial sync on large accounts "can take minutes to perform" (spec motivation);
  real reports cite a 2,200-room account pulling 100,000+ events and `/sync`
  taking up to ~400 seconds, and Element initial syncs up to ~20 minutes.
- Incremental syncs are **unbounded** — the cost of catching up grows with how
  long the user was offline, not with how much they care to see now.
- Clients "cannot opt-out of extraneous data such as receipts."

The structural flaw: **sync cost is tied to account size, not to the working
set.** Every later stage attacks exactly this.

## Stage 1 — Lazy-loading members (MSC1227)

First incremental fix. Membership events dominate room state in big rooms (every
join/leave ever). With `lazy_load_members` set on the sync filter, the server
returns membership events **only for senders of events in the returned timeline**,
not the whole member list. Adds `include_redundant_members` to control whether
unchanged memberships are re-sent ("redundant membership events").

Lesson: the first scaling win wasn't "fewer rooms," it was "less *state per
room*." Sliding Sync's `required_state` lazy-member selection is the direct
descendant — `$LAZY` in MSC3575, then a `lazy_members` boolean in MSC4186.

## Stage 2 — MSC3575, "Sliding Sync (aka Sync v3)"

The redesign (author: Kegan Dougal / kegsay). Three structural moves:

1. **Window, don't dump.** The client subscribes to **lists** (filtered, sorted
   server-side room views) and requests only a **range** `[0, 99]` of each. Sync
   cost becomes proportional to the *window*, not the account. See
   [windowing.md](./windowing.md).
2. **Stateful, sticky connection.** Parameters persist server-side across
   requests keyed by `conn_id`; the client sends deltas and resumes with `pos`.
3. **Current state, not historical.** `required_state` returns the room's *latest*
   state, so the server caches one snapshot per room instead of reconstructing
   "state before timeline" per request.

Delivered via a standalone **proxy** (`matrix-org/sliding-sync`, Go, "Sync v3")
that sat in front of an unmodified homeserver and translated v2 `/sync` into the
new API. This decoupled client iteration from homeserver release cycles — a
pragmatic shipping vehicle, not the end state.

## Stage 3 — MSC4186, "Simplified Sliding Sync"

After real-world use, MSC3575 was judged too complex on the server side. MSC4186
(author: Erik Johnston) is "paring back that API based on real world use cases
and usages." What got cut is the signal:

- **Server-driven list ops removed.** MSC3575 streamed `SYNC`/`INSERT`/`DELETE`/
  `INVALIDATE` operations to mutate the client's list in place (see
  [windowing.md](./windowing.md)). MSC4186 drops this: the response just carries
  `lists[name].count` (total matching rooms) plus changed rooms, and the **client
  re-derives ordering locally**. Maintaining a server-authoritative ordered list
  per connection was the expensive part.
- **Timeline filtering removed** — the spec notes it "does not support timeline
  filtering, which is heavily used by e.g. bots."
- **`notification_count` / `highlight_count` removed** from the room result.
- **Moved params from URL into POST body** (CORS / size).
- **`required_state` format and `lists` stickiness** reworked.

MSC4186 keeps: ranges/windows, room subscriptions, `conn_id`/`pos`,
`required_state` with lazy members, the extensions framework.

## Where it landed

- **Synapse 1.114.0** (2024-09-02) shipped native support: changelog reads
  "Enable native sliding sync support (MSC3575 and MSC4186) by default." This
  removed the need for the proxy.
- The proxy repo was **archived 2025-11-17**: "MSC3575 … has been superseded by
  MSC4186 … The proxy is no longer being worked on and is now archived."
- **matrix-rust-sdk** removed MSC3575 support (PR #4531); Element X cut over fully
  to native Simplified Sliding Sync on **2025-01-17**.
- Matrix 2.0 (2024-10-29) markets the result as "instant login, instant launch,
  and instant sync," with Element X as the only Matrix 2.0 client (per the
  2024-10-29 blog).
- Even the simplified server impl shipped a security advisory: **CVE-2024-53867**
  (GHSA-56w4-5538-8v8h, CVSS 3.1) — Synapse Sliding Sync leaked partial room
  *state* changes to users no longer in a room, fixed in Synapse 1.120.1. A
  reminder that a newly-built subscription surface is its own attack surface; the
  "narrow the surface" lesson cuts both ways.

## The one-sentence takeaway for Myrhiza

The maximalist design (server maintains and streams an authoritative ordered list
per connection) was the part that got deleted; the survivor is **server sends a
count + changed items, client owns ordering and aggregation.** Design
`host.subscribe` at the MSC4186 altitude, not the MSC3575 one.

## Sources

- https://github.com/matrix-org/matrix-spec-proposals/blob/kegan/sync-v3/proposals/3575-sync.md
- https://github.com/matrix-org/matrix-spec-proposals/blob/erikj/sss/proposals/4186-simplified-sliding-sync.md
- https://github.com/matrix-org/matrix-spec-proposals/pull/3575
- https://github.com/matrix-org/matrix-spec-proposals/pull/4186
- https://github.com/matrix-org/matrix-doc/issues/1227
- https://matrix.org/blog/2024/11/14/moving-to-native-sliding-sync/
- https://github.com/matrix-org/sliding-sync
- https://github.com/element-hq/synapse/releases
- https://matrix.org/blog/2024/10/29/matrix-2.0-is-here/
- https://github.com/element-hq/synapse/security/advisories/GHSA-56w4-5538-8v8h
- https://nvd.nist.gov/vuln/detail/CVE-2024-53867
