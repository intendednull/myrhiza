**Date:** 2026-05-09
**Status:** active
**Subject:** Willow — Leptos web UI, dual-target discipline, UI-as-app reframe

What `willow-web` ships today, the dual-target rules that make a single
WASM build a viable browser client, and how PR #636 reframes the UI as
one app among many. Companion: [networking.md](networking.md),
[identity.md](identity.md), [crypto.md](crypto.md), [README.md](README.md).

## `willow-web` today (`crates/web/src/`)

The web client is a Leptos application built via wasm-pack / trunk.
It is the only production UI Willow ships today; the Bevy desktop
app was retired and removed from the workspace during the iroh
migration (`docs/specs/2026-03-29-iroh-migration-design.md` status
line). The component tree is substantial — 60+ components under
`crates/web/src/components/` covering chat, channel sidebar, grove
rail, drawers, profile cards, member list, command palette,
participant tiles, voice notes, file uploads, and onboarding.

Reactive primitives are Leptos signals (`Signal::derive`,
`RwSignal`, `Resource`). Per CLAUDE.md, "Reactive UI state in web"
is one of two non-actor exceptions in the state-management table
(the other is `Rc<RefCell<T>>` for non-reactive single-threaded
WASM state). Web state mutated from non-Leptos contexts goes
through a `StateActor<S>` (CLAUDE.md §State Management table).

## Async client + UI refactor (`docs/specs/2026-03-24-async-client-ui-refactor-design.md`)

Status: shipped. Replaced two layers of polling — a 16ms
`gloo_timers` interval polling `cmd_rx.try_recv()` plus a 50ms
`set_interval` polling `client.poll()` — with `futures::channel::mpsc::unbounded`
in both directions. The WASM network loop now `select!`s directly
on `cmd_rx.next()`; `ClientEventLoop` `await`s on `event_rx.next()`.

The same refactor split the monolithic `App` (~800 lines, 30 loose
signals threaded as props, 14 `Rc<RefCell<Client>>` clones) into:

- **`SharedState`** — `Rc<RefCell<...>>` wrapping all mutable state
  (`servers`, `event_state`, `chat`, `profiles`, `message_db`, `identity`,
  config, presence, voice flags, typing state).
- **`ClientHandle`** — cloneable command interface holding
  `Rc<RefCell<SharedState>>` + `UnboundedSender<NetworkCommand>`.
  Exposes mutation methods (`send_message`, `create_channel`,
  `switch_server`, `join_voice`) and read accessors.
- **`ClientEventLoop`** — async event loop that drains
  `NetworkEvent`s, applies them to state, and emits
  `ClientEvent`s for UI subscribers.

The `Rc` vs `Arc` choice is honest: the refactor "intentionally
breaks native and targets WASM only," so `SharedState` uses
`Rc<RefCell<...>>` instead of `Arc<Mutex<...>>` — re-adding native
will need a cfg-gated swap (spec §Client Split).

## UI design bundle (`docs/specs/2026-04-19-ui-design/README.md`)

The active UI target is a 22-spec multi-file bundle, target-state
not current-state. Phases run from foundation tokens through to
onboarding/settings. Concrete features the bundle nails down:

- **Three-pane desktop shell + mobile shell** with a 721px
  breakpoint (UI phase 1a/1b in `layout-primitives.md`). Grove
  rail, channel sidebar, main pane, right rail; mobile collapses
  to a bottom-tab chrome with swipe drawers and bottom sheets.
- **Command palette (Ctrl+K)** + accessibility baseline (UI phase
  1c). Focus-visible required on every interactive element;
  reduced-motion paths required for every animation.
- **7-state presence** — `here`, `away`, `whispering`, `in a call`,
  `queued`, `gone`, plus self-presence overrides
  (`presence.md`). `PresenceState::Here` is the default
  (`crates/web/src/components/message.rs:742`).
- **In-app toast stack + push notifications** with per-surface
  mute overrides (`notifications.md`).
- **17-field profile cards** — crest banner, pronouns, handle,
  nickname, bio, tagline, pinned fragment, shared groves, and
  more (`profile-card.md`); desktop popover + mobile bottom sheet.
- **Local on-device search** with encrypted-at-rest index +
  scope ladder + `/`, `⌘F`, palette entry points
  (`local-search.md`).
- **Reactions / pins** with permission-gated pin action
  (`reactions-pins.md`).
- **File uploads** with drag-and-drop + paste-to-upload + voice
  notes (`files-inline.md`).
- **Whisper mode**, **device handoff**, **ephemeral channels**,
  **call experience** (grove / grid / focus layouts) — phases 4
  and 5.

The bundle's design language is named explicitly: serifs for
display (Fraunces), sans for body (IBM Plex Sans), mono for
crypto artefacts (JetBrains Mono); palette is bark + moss on
ink-on-deep-bark dark; "willowPulse" / "leafFall" /
"willow-pop-in" motion (bundle README §"Design language at a
glance"). Trust-first-but-calm is the load-bearing design
principle: "Crypto state is always visible — never loud."

The terminology map is a UX vocabulary swap, not a code rename:
`server → grove`, `dm → letter`, `private side-channel → whisper`,
`offline queue → sync queue`, `device transfer → handoff`,
`ephemeral channel → ephemeral`. Internal identifiers (`server`,
`channel`, `dm`, `peer`, `event`) are unchanged (bundle README
§Terminology map).

## Dual-target discipline (per CLAUDE.md §Dual-Target Support)

All lib crates compile both native and `wasm32-unknown-unknown`:

- **No `std::fs`** in lib crates — gate with
  `#[cfg(not(target_arch = "wasm32"))]`.
- **No `std::time::SystemTime`** — use `js_sys::Date::now()` on
  WASM.
- **No `std::thread`** or **tokio** in lib crates — native-only.
- **RNG:** `getrandom` needs `js` (v0.2) / `wasm_js` (v0.3)
  features on WASM; UUID workspace dep includes `js` for v4
  generation.
- **Network:** "iroh handles WASM transport differences internally,
  so most `#[cfg(target_arch = "wasm32")]` gates for networking no
  longer needed" (CLAUDE.md, post-iroh-migration).

Native uses `Arc<Mutex<...>>`; browser uses `Rc<RefCell<...>>`
behind a cfg gate where the type lives in a web-only crate. The
crate-level rule "all types must be `Send + Sync`" applies to lib
crates; `willow-web` is exempt.

## PR #636 — UI-as-app reframe

PR #636 reframes the Leptos web client as **the default UI app**
(lines 124-159). Other apps' interaction components import
`ui:*` interfaces — `ui:panel`, `ui:list`, `ui:message`,
`ui:form`, `ui:menu` — that the UI app exports. The honest
framing in the spec: "a real UI on any platform requires a broad
and unstable capability surface (DOM + focus/IME, clipboard,
file pickers, navigation, viewport/media queries, push, IndexedDB,
service workers, drag-and-drop on web)" — the kernel does not try
to abstract that surface. The default UI app is privileged to
bind the broad browser capability set; it is shipped in-tree as
one app, but is not architecturally identical to a third-party
interaction-only utility (lines 130-138).

`ui:*` is **an interaction contract for app-to-UI integration,
not a portable UI substrate** (lines 139-146). Each UI app
implements the contract in its own idiom; reusing one set of
interaction components across UIs is the goal but each UI app is
a substantial standalone project. Plausible alternative UI apps
(lines 148-159):

- `willow-ui-tui` — terminal, ratatui rendering, the chat-shaped
  subset of `ui:*`.
- `willow-ui-mcp` — agent host, structured-data rendering for an LLM.
- `willow-ui-mobile-native` — Compose / SwiftUI shell, far-future.
- `willow-ui-dioxus` — once Dioxus Blitz matures, candidate
  replacement for Leptos.

UI apps that do not export an interface (e.g. a TUI without
`ui:rich-card`) cause graceful degradation, not breakage.

**Custom-pixel surfaces** (whiteboard, code editor, network-graph
viz, 3D voice room) ship as sandboxed iframes embedded by the
default UI app, communicating through a postMessage protocol
that is itself a kernel-mediated capability (lines 161-171).
Bevy is ruled out as the *primary* substrate but kept as a
far-future GPU-driven escape hatch once its web tooling matures
(2027-2028 timeframe). The escape hatch is browser/native shaped
on purpose; a TUI host or MCP host renders these as "unavailable
on this surface" rather than attempting fallback.

The capability-checking commitment is sharp: **`ui:*` calls that
proxy privileged platform surfaces are capability-checked per
call, not just per import-binding** (lines 326-335). Clipboard
writes, file pickers, top-level navigation, push registration,
and similar — each call is gated by the *calling component's*
manifest, not the UI app's broad surface. This prevents a
malicious or compromised interaction component composed inside
the UI app from socially-engineering the UI into doing things
the calling component was never granted. The UI app is in the
TCB for its own chrome and DOM; it is **not** in the TCB for
arbitrary callers' intents.

## Lift-into-Myrhiza notes

- **Leptos-as-default-UI-app is the Myrhiza commitment.** The
  60+ components under `crates/web/src/components/` are a real
  reservoir of UX work (presence, profile cards, command palette,
  reactions, file uploads, voice notes). Lifting them as the
  default UI app's implementation of `ui:*` is direct — they
  already speak Leptos signals natively. The surface they bind
  becomes the default UI app's privileged capability set.
- **`ui:*` contract design** is a deferred child spec (PR #636
  line 609 lists it under "Child specs (planned) — WIT
  interfaces"). Myrhiza must own writing it; nothing in current
  Willow constrains the WIT signatures yet.
- **Per-call capability checking on `ui:*` proxies** is a
  load-bearing architectural commitment — not an implementation
  detail. Don't elide.
- **Custom-pixel iframe escape hatch** is the explicit out-of-band
  channel for whiteboard/code-editor/3D-voice surfaces. Lift the
  postMessage-as-kernel-mediated-capability framing.
- **Dual-target discipline at the kernel layer** — PR #636 commits
  the kernel to compile to both native (wasmtime) and WASM
  (jco-transpiled). Lib-crate dual-target rules from Willow's
  CLAUDE.md transfer; the trickier piece is platform-specific
  backends behind kernel-internal traits (MLS engine, persistent
  key storage, full-fat blob store), where confirming each
  survives jco transpilation is part of the deferred
  crypto-and-key-custody child spec (PR #636 lines 354-364).

## Repo

- GitHub: [github.com/intendednull/willow](https://github.com/intendednull/willow)

## Sources

- Willow repo: `/mnt/storage/projects/willow`
- `crates/web/src/` — `app.rs`, `state.rs`, `event_processing.rs`, components subtree (60+ files)
- `crates/web/src/components/` — message, chat, channel_sidebar, command_palette, profile_card, profile_popover, mobile_shell, grove_drawer, grove_rail, member_list, etc.
- `docs/specs/2026-03-24-async-client-ui-refactor-design.md` — async channel refactor + Client split (`SharedState` / `ClientHandle` / `ClientEventLoop`)
- `docs/specs/2026-04-19-ui-design/README.md` — 22-spec target-UX bundle, phases, terminology map
- `docs/specs/2026-04-19-ui-design/foundation.md`, `layout-primitives.md`, `presence.md`, `notifications.md`, `profile-card.md`, `local-search.md`, `reactions-pins.md`, `files-inline.md`, `whisper-mode.md`, `device-handoff.md`
- PR #636 §"UI is an app" (lines 124-171), §"Capability model" (lines 326-335), §"What stays the same about Willow" (dual-target survival, lines 354-364)
- `willow CLAUDE.md` § State Management, § Dual-Target Support, § Code Conventions
