**Date:** 2026-05-09
**Status:** active
**Subject:** Willow — multi-tier test architecture and event-based-waits discipline

Willow's test architecture is the single most easily-lifted piece of
discipline in the corpus. The tier hierarchy, the decision tree, and
the event-based-waits primitives all translate directly to Myrhiza
with the kernel/app split applied as a relabel.

See also: [README.md](README.md), [lessons.md](lessons.md),
[determinism.md](determinism.md).

## The five-tier hierarchy

`docs/specs/2026-04-21-e2e-test-architecture-design.md` and Willow's
CLAUDE.md "Testing Strategy" / "Which test tier to use" sections
define five tiers. Each tier targets a distinct class of behaviour and
runs at a distinct cost.

### Tier 1 — Pure state-machine tests (`just test-state`)

- **Crate:** `crates/state/src/tests/{dag,materialize,sync,permissions,voting,stress}.rs`.
- **Speed:** instant (sub-second to run the full state suite). No I/O,
  no networking, no async runtime — `willow-state` is documented in
  `lib.rs:3-6` as "pure no-I/O, no-networking."
- **Coverage:** event application, permission enforcement, merge
  convergence, dedup, HLC ordering, anti-DoS caps, equivocation
  rejection, topo-sort determinism. Stress sub-suite covers 1000
  messages, 100-event replay, 3-way merge.
- **Helper:** `make_event(state, author, kind)` — constructs a signed
  event without leaving the test process.

This is **the natural Myrhiza analogue for state-apply tests**.
Myrhiza's `state-apply` is a WASM function that is a pure function of
`(prior state, event)`; tests can drive it in pure host code, no
network, no peer. Determinism property tests (cross-impl convergence)
land here.

### Tier 2 — Client library tests (`just test-client`)

- **Crate:** `crates/client/src/tests/{actions,multi_peer_sync,trust_flow,...}.rs`.
- **Speed:** fast (seconds). No browser, no real network.
- **Helper:** `test_client()` constructs a `ClientHandle` without
  networking. Multi-peer flows use `MemNetwork` (`crates/network/src/mem.rs`,
  624 lines) — a pure in-memory `Network` trait implementation that
  gossips synchronously between peers in the test process.
- **Coverage:** ClientHandle API surface (send, create, mute, trust,
  verify), derived view computation (channels, unread, presence,
  roles, connection), multi-peer sync semantics (invite + join + replay
  + reconnect + SyncBatch), governance flows.

`MemNetwork` is the load-bearing pattern. It satisfies the same
`Network` / `TopicHandle` / `BlobStore` traits as production
`IrohNetwork` so client code is unchanged between test and prod, but
gossip happens via in-process channels with no QUIC, no relay, no
wall-clock delay.

**Myrhiza needs an in-memory transport double early.** The temptation to
"only test against real iroh" must be resisted; the client-tier suite
is what unlocks fast iteration on app-runtime semantics.

### Tier 3 — Relay history tests (`just test-relay`)

- **Crate:** `crates/relay/`.
- **Coverage:** relay stores events, serves history to new peers,
  multi-peer history aggregation, offline peer recovery via relay.
- **Speed:** seconds. Real relay code path; in-process peers.

### Tier 4 — In-browser Leptos tests (`just test-browser`)

- **Crate:** `crates/web/tests/browser.rs`.
- **Toolchain:** wasm-pack + headless Firefox + geckodriver.
- **Helper:** `mount_test(|| view! { ... })` renders into the test DOM;
  `tick().await` flushes reactive effects;
  `mount_test_with_shell(TestShell::Desktop | Mobile)` for
  viewport-specific flows.
- **Coverage:** real DOM rendering, signal reactivity, event handling,
  effects, all components (sidebar, messages, input, channels,
  settings, member list, server list, connection status). Single client
  + single viewport.
- **Speed:** ~1 minute for the full suite.

For Myrhiza this tier is **per-UI-app**, not per-kernel. The kernel
itself has nothing to render. UI apps that ship via Myrhiza inherit a
"each UI app's own browser-test suite" responsibility.

### Tier 5 — Playwright E2E (`just test-e2e-ui`)

- **Folder:** `e2e/*.spec.ts` (8 spec files, ~1814 LOC at audit time).
- **Coverage:** multi-peer real-network P2P (real iroh + relay + gossip
  over the wire), cross-browser quirks (Firefox-specific behaviours),
  touch gestures (swipe, long-press, pull-down), viewport-driven
  responsive breakpoints, browser-integration paths (service worker,
  push, clipboard, navigator, browser back/forward + hash routing,
  fullscreen).
- **Helper:** `setupTwoPeers`, `sendMessage`, `setupTwoPeersJoined`
  in `e2e/helpers/`. Real Playwright `browser` fixture for multi-peer.
- **Target speed:** <3 min after Phase B migration; ~40 min before
  Phase A speedups (audited and rebuilt).

## The decision tree

Willow CLAUDE.md "Which test tier to use" verbatim:

1. **State-machine logic only?** (event application, permissions,
   merge, dedup, HLC) → Rust state crate test.
2. **Client API + derivation, no DOM?** (mutations, view signals,
   ClientHandle methods) → Rust client crate test.
3. **Multi-peer sync semantics?** → Rust client crate test with
   `MemNetwork` (unless validating real iroh/QUIC behaviour
   specifically).
4. **DOM rendering or event dispatch?**
   - Single client + single viewport → wasm-pack browser test.
   - Multi-client or multi-viewport → Playwright.
5. **Cross-browser quirk coverage?** → Playwright.
6. **Touch / gesture / mobile-shell media query?** → Playwright
   mobile-chrome.
7. **Service worker, push, or navigator APIs?** → Playwright.

**Default to the lowest tier covering behaviour.**

**Rewrite trigger.** Playwright test fails because selector/helper
drifts — not because behaviour broke — test at wrong tier. Migrate it
down on the same commit.

This decision tree, with one substitution (state-tier becomes
state-apply-WASM-component-tier), is **directly liftable to Myrhiza**.

## Event-based waits — the antipattern Willow paid for

`docs/specs/2026-04-27-event-based-waits-design.md` (614 lines) is the
definitive treatment. The audit found:

- **53** `waitForTimeout(ms)` calls in helpers and specs (200ms–2000ms
  each).
- **71** `{ timeout: <ms> }` overrides on assertions, including 23 of
  30s, 8 of 60s, 8 of 120s.
- **3** polling loops sleeping 300ms gating on UI visibility.
- **0** uses of `waitForFunction`, `expect.poll`, `waitForResponse`, or
  any app-emitted event.

Per Playwright's own guidance, replacing `waitForTimeout` removes ~45%
of flake. The suite's wall-clock was dominated by sleeps that succeeded
long before they expired.

The fix is **three categories of wait, three tools**:

| Bucket | What you wait for | Tool |
|---|---|---|
| **State convergence** | Peer B applied event H from peer A | Push (`__willowEvent`) for ordered events; pull (`expect.poll(snapshot)`) for "eventually X" |
| **DOM / animation settle** | Drawer slide, dropdown fade, modal open | `data-state="<phase>"` attribute flipped on `transitionend` |
| **Real durations** | longPress 600ms, debounce, HLC drift | `page.clock.runFor('600ms')` |

Anti-patterns explicitly forbidden:

- `page.waitForTimeout(ms)` — banned by ESLint rule.
- `waitForLoadState('networkidle')` — unsafe for gossip apps.
- `expect(await locator.isVisible()).toBe(true)` — defeats auto-retry.
- Setting up `waitForResponse` *after* the trigger — race.

### `WillowTestHooks` WASM API

A cargo feature `test-hooks` (off in production) exports a
`WillowTestHooks` JS-visible struct with three `Promise`-returning
methods: `snapshot()`, `heads()`, `event_count()`, `last_event()`. The
production build pays no cost: no exported symbols, no event
subscription, no `window.__willow`. CI symbol-leak check
(`! grep WillowTestHooks dist/*.wasm`) enforces.

A push-side dispatcher subscribes to `client.subscribe_events()` and
dispatches every wire-visible `ClientEvent` to a Playwright
`exposeBinding('__willowEvent', …)` callback, with a per-page buffer
and overflow detection.

The TypeScript `Peer` wrapper (`e2e/test-hooks.ts`) provides
`nextEvent(predicate)`, `snapshot()`, `heads()`, `eventCount()`,
`waitUntilHeadsEqual(other)` — the canonical multi-peer assertions.

### `data-state` lifecycle attribute

Five animated UI elements (mobile drawer, grove drawer, confirm dialog,
bottom sheet, tab bar) plus the action-sheet overlay carry a
`data-state="<phase>"` attribute that flips on `transitionend` (with
reduced-motion fallback). Tests assert
`expect(el).toHaveAttribute('data-state', 'open')` — driven by the CSS
transition itself, auto-retried by Playwright's web-first assertions.

### `page.clock` for real durations

Native Playwright since 1.45. Patches `Date`, `setTimeout`,
`setInterval`, `requestAnimationFrame` — covers `js_sys::Date::now()`
calls inside WASM. Used for `longPress`, debounce timers, HLC drift
simulation.

### Ratchet harness

`just test-e2e-flake N=10` runs the suite N times to detect remaining
flake; CI gate ratchets the allowed failure count downward over time.
Implemented in PR4 of the event-based-waits sequence.

## Multi-peer E2E patterns

`e2e/helpers/peers.ts` exposes:

- `setupTwoPeers()` — fresh-start + welcome + invite + join settle for
  two browser contexts.
- `setupTwoPeersJoined()` — variant that skips the welcome screen.
- `sendMessage(page, text)` / `expectMessage(page, text)` — paired send
  + observe assertions.
- `Peer` class wrapping `WillowTestHooks` + Playwright page.

The pattern Willow paid for: each `setupTwoPeers` call is ~25% of the
old wall-clock. Phase A in the test-architecture spec replaces the
`waitForTimeout(3000)` post-join settle with a deterministic
`locator().waitFor()` on the first `.channel-item`.

## Test infrastructure crates

- **`crates/network/src/mem.rs`** (624 lines) — the `MemNetwork`
  in-memory transport double. Implements the `Network` trait family
  with in-process gossip via `tokio::sync::broadcast` channels.
  **Note:** explicitly does NOT compile to wasm32 (uses
  `tokio::sync::broadcast`); a wasm-buildable variant is the
  test-hooks-vs-test-utils distinction.
- **`crates/state/src/tests/`** — six test files covering DAG, sync,
  materialize, permissions, voting, stress.
- **`crates/client/src/tests/`** — eleven test files; `actions.rs`,
  `multi_peer_sync.rs`, `trust_flow.rs`, `governance.rs`, etc.

## What Myrhiza lifts directly

- **The five-tier hierarchy** (state / client / relay / browser /
  Playwright) with state→state-apply-WASM and client→runtime-host
  re-labels.
- **The decision tree** verbatim, including the rewrite trigger.
- **An in-memory transport double early** — Myrhiza's analogue of
  `MemNetwork` is non-negotiable. Build it before the test suite
  calcifies.
- **Event-based-waits architecture** — `__willowEvent`-equivalent
  push channel, `expect.poll`-style pull, `page.clock` for real
  durations. Forbid `waitForTimeout` with ESLint rule from day one.
- **`data-state` lifecycle attribute** for animated UI elements.
- **Ratchet harness** for measuring flake rate over time.
- **`test-hooks` cargo feature distinct from `test-utils`** — the
  narrow read-only feature stays wasm-buildable; the broader test
  helpers stay native-only.

## What Myrhiza re-evaluates

- **State-apply WASM tests in pure host code** — running deterministic
  WASM components in a test wasmtime + a fake event stream is
  cheaper than booting any client. Myrhiza's tier-1 looks more like
  "execute the state-apply component against fixture events" than
  "execute Rust state code."
- **Cross-impl convergence tests at tier 1.** Determinism's load-bearing
  property — two implementations of the same `state-apply` reach
  bit-identical state digests — is naturally a state-tier test, but
  needs a second runtime backend.
- **The Playwright tier may not exist for the Myrhiza kernel itself.**
  Per-app UI suites live in app repos; Myrhiza's kernel testing tops
  out at multi-peer sync (client-tier with `MemNetwork`-equivalent).

## Repo

- GitHub: [github.com/intendednull/willow](https://github.com/intendednull/willow)

## Sources

- `/mnt/storage/projects/willow/docs/specs/2026-04-21-e2e-test-architecture-design.md`
  (142 lines).
- `/mnt/storage/projects/willow/docs/specs/2026-04-27-event-based-waits-design.md`
  (614 lines).
- `/mnt/storage/projects/willow/docs/specs/2026-04-13-test-architecture.md`
  (714 lines, predecessor).
- `/mnt/storage/projects/willow/docs/plans/2026-04-27-event-based-waits-pr1-test-hooks-foundation.md`
  + `pr2-peer-wrapper.md`, `pr3-data-state-lifecycle.md`,
  `pr4-ratchet-flake-harness.md` — execution plans.
- `/mnt/storage/projects/willow/CLAUDE.md` — "Testing Strategy" +
  "Which test tier to use" sections.
- `/mnt/storage/projects/willow/crates/state/src/tests/` — six test
  files.
- `/mnt/storage/projects/willow/crates/client/src/tests/` — eleven
  test files.
- `/mnt/storage/projects/willow/crates/network/src/mem.rs` (624
  lines) — the `MemNetwork` test double.
- `/mnt/storage/projects/willow/e2e/helpers/` and
  `/mnt/storage/projects/willow/e2e/test-hooks.ts`.
