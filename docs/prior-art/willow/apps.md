**Date:** 2026-05-09
**Status:** active
**Subject:** Willow — what ships as an "app" today (one chat application, baked-in) vs PR #636's many-apps reframe

Willow today is a single chat application baked into 15 production crates.
There is no "app" abstraction; chat semantics are hard-coded into
`willow-state`, `willow-web`, and the worker binaries. PR #636 reframes the
whole codebase as one app among many on a generic kernel.

See also: [actors.md](actors.md), [workers.md](workers.md),
[state-machine.md](state-machine.md), [README.md](README.md).

## Today: one chat app, baked-in

`crates/state/src/event.rs:280-468` defines `EventKind` — 22 chat-shaped
variants:

- **Server lifecycle**: `CreateServer { name }` (genesis).
- **Governance**: `Propose { action }`, `Vote { proposal, accept }`.
- **Permissions**: `GrantPermission`, `RevokePermission` (where
  `Permission::SyncProvider` authorizes workers).
- **Server structure**: `CreateChannel`, `DeleteChannel`, `RenameChannel`,
  `ChannelRevive`, `CreateRole`, `DeleteRole`, `SetPermission`, `AssignRole`.
- **Chat**: `Message`, `FileMessage`, `EditMessage`, `DeleteMessage`, `Reaction`.
- **Identity**: `SetProfile`, `UpdateProfile`.
- **Encryption**: `RotateChannelKey { channel_id, encrypted_keys }`.
- **Pinning**: `PinMessage`, `UnpinMessage`.
- **Server metadata**: `RenameServer`, `SetServerDescription`.
- **Per-identity mute**: `MuteChannel`, `MuteGrove`.

`ServerState` (`crates/state/src/lib.rs:53`, materialize logic in
`materialize.rs`) represents one chat server. Pure function:
`apply_event(prior_state, event) -> next_state`. No I/O, no actors —
the event-sourced authority is a deterministic Rust function that has
chat hard-coded into it.

### Multi-server via the Grove model

A single peer joins multiple chat servers via the Grove model.
`ClientHandle` (`crates/client/src/lib.rs:270-348`) carries a
`server_registry_addr: Addr<StateActor<ServerRegistry>>` and a
`servers: HashMap<String, ServerContext>` keyed by server ID.
`MuteScope::Grove` is per-author whole-grove muting that silences toasts
across every server. Each server has its own gossip topic, its own
`EventDag`, its own materialized `ServerState`. The "Grove" is the
user-facing aggregate; the kernel-side abstraction is just a registry of
`ServerState`s with a chosen-active one.

Per-domain `StateActor`s under `ClientHandle` (lines 281-310) split state
along chat-shaped axes: `event_state_addr: StateActor<ServerState>`,
`server_registry_addr`, `chat_meta_addr` (current channel, peers, dedup),
`profile_state_addr`, `network_meta_addr`, `voice_state_addr`,
`presence_meta_addr`, `queue_meta_addr` (per-peer outbound tracking),
`dag_addr` (the per-author Merkle DAG), `persistence_addr` (`rusqlite`).
These are not generic. `VoiceState`, `ChatMeta`, `ProfileState` exist
because Willow ships chat. Voice in particular shows the bake-in: own
actor, own WebRTC closures (`crates/web/src/voice.rs`), own `Rc<RefCell>`
exemptions in the lock-discipline spec.

### UI bound to chat semantics

`crates/web/` (Leptos) imports chat types directly: components, voice.rs,
audio.rs, notifications.rs, upload_state.rs, profile/, palette_recents.rs,
reaction_recency.rs. No abstraction for "rendering an arbitrary app's
view"; it imports `ChatMessage`, `ServerState`, `Channel` from
`willow-state`.

The MCP agent (`crates/agent/src/`) exposes `ClientHandle` to LLM agents
via tools, resources, notifications (`auth.rs`, `notifications.rs`,
`resources.rs`, `scopes.rs`, `server.rs`, `tools.rs`). First-class peer
with own Ed25519 identity, but bound to the chat domain.

Workers are chat-specific too: `willow-replay`/`willow-storage` both call
`apply_event` against `ServerState` (`crates/replay/src/role.rs:11-15`
imports `EventKind`, `ServerState`, `Snapshot` directly). The
`WorkerRole::on_event(&Event)` signature takes `willow_state::Event` —
the chat event type.

## PR #636: one app among many

PR #636 §"What changes about Willow" (diff lines 421-444):

> **`willow-state` splits.** A payload-agnostic kernel half (events, DAG,
> sync primitives, HLC) stays as kernel. The chat-specific half
> (`EventKind`, `ServerState`, `apply_event`, `required_permission`)
> becomes the `chat-server` app.
>
> **The web client becomes the default UI app.** Its bindings to chat
> semantics route through the kernel and the chat-server interaction
> component rather than through direct Rust imports of chat types.

An **app** in PR #636's framing (diff lines 96-122) is a content-addressed
bundle on iroh-blobs:

```
chat-server/
├── manifest.toml                     (version, hashes, capabilities, interfaces)
├── state.wasm                        (deterministic; required by materializing peers)
├── interaction.wasm                  (typed view + commands; loaded if peer has UI)
├── behavior-discord-bridge.wasm      (optional; loaded by peers with capability)
└── schema.wit                        (interface contract)
```

Apps can ship any subset of the four component profiles
([actors.md](actors.md) covers state-apply / state-propose; full table at
diff lines 73-93): state-only (pure semantics), interaction-only
(alternative UI for someone else's state app), state+interaction, or any
combination. A peer fetches by hash and instantiates only what it needs.

### UI app catalog (PR #636 lines 124-159)

The Leptos web client becomes `willow-ui-leptos`. Honest framing: a real
UI requires a broad and unstable capability surface (DOM + focus/IME,
clipboard, file pickers, navigation, viewport/media queries, push,
IndexedDB, service workers, drag-and-drop) that the kernel doesn't
abstract. The default UI app is **privileged** to bind a broad
browser-shaped capability surface. Other UI apps each implement the
`ui:*` WIT interfaces in their own idiom.

Plausible UI apps (lines 147-154): **`willow-ui-tui`** (terminal,
ratatui, chat-shaped subset of `ui:*`); **`willow-ui-mcp`** (today's
`willow-agent` becomes this); **`willow-ui-mobile-native`** (Compose /
SwiftUI, far-future); **`willow-ui-dioxus`** (once Dioxus Blitz is
mature).

App authors target the WIT contract; their interaction components work
against any UI app exporting the imported interfaces. UI apps that don't
export an interface (e.g. TUI without `ui:rich-card`) cause graceful
degradation, not breakage. **Custom-pixel surfaces** (whiteboard, code
editor, network-graph viz, 3D voice room) are an explicit out-of-band
escape hatch — sandboxed iframes on web, platform-specific elsewhere —
not part of the `ui:*` contract.

### MVP demo apps + acceptance criteria

PR #636 §"MVP, in spirit" (diff lines 575-598) candidate proof-points:
**tiny shared-counter app** (~50 lines state, ~100 lines interaction;
deliberately irrelevant to chat); **single-channel chat that doesn't
reuse `ServerState`**; **real-time poll**. The framing cares about *not
chat* more than which not-chat — proving the kernel doesn't know what
chat is.

Six concrete acceptance criteria (lines 577-593):

1. Kernel loads + instantiates a WASM state component from an iroh-blobs
   bundle.
2. Multi-peer convergence — same component bytes across peers converge
   to the same state hash.
3. UI app loads interaction component, projects view, submits command,
   observes resulting state change.
4. Two app instances coexist on one peer without event-crossing —
   different state component, different topic, no leakage.
5. Capability declarations actually gate access — a component cannot
   import an interface its manifest does not declare.
6. Behavior component runs on a designated peer, observes events, logs
   them. Emitting events under kernel-custodied behavior identity is
   the next milestone after MVP, blocked on capability + identity-custody
   child specs.

## For Myrhiza

Myrhiza is the realization of PR #636's destination: chat is one app
running on the runtime, the runtime is the host project. Direct
consequences:

- **`EventKind` becomes app-defined.** Each app's state component owns
  its event variants. The kernel sees only `Event { author, prev, deps,
  payload, sig }` with `payload: Vec<u8>` — opaque bytes the kernel
  doesn't decode.
- **`ServerState` doesn't exist at the kernel layer.** Each app's state
  component owns its materialized state in linear memory and exports a
  `state-digest()` for cross-peer convergence checking (PR #636 lines
  252-263) — the kernel hashes a canonical encoding the app produces,
  *not* WASM linear memory which would diverge trivially due to
  allocator/padding/iteration-order.
- **The Grove model generalizes to "apps a peer has joined."** Today's
  `server_registry_addr: StateActor<ServerRegistry>` becomes
  `app_registry_addr: StateActor<AppRegistry>` — a registry of (app
  bundle hash, topic, materialized state component instance). One peer
  running five apps across five topics; instances coexist without
  event-crossing per MVP criterion #4.
- **The MCP agent becomes `willow-ui-mcp`**, an interaction-only app
  exposing whatever interfaces the user's installed apps want surfaced
  to LLMs. Today's bake-in (importing `ChatMessage` directly) routes
  through the chat-server interaction component.
- **Capability prompts at install** — installing an app prompts a
  capability summary the kernel renders ("this app wants to broadcast
  events on topic X, store ≤ 1 MB locally, send HTTP requests to
  discord.com"). Granted at instantiate time; cannot escalate without
  re-prompting.

The bake-in inventory above (chat-shaped per-domain `StateActor`s under
`ClientHandle`, chat-typed worker `WorkerRole` impls, chat-importing web
crate) is also the work-list for the chat-server-migration child spec
PR #636 names at line 636-637 ("much later"). Each chat-shaped construct
is something Myrhiza has to either reframe into the chat-server app
bundle or replace with a generic kernel primitive. The split runs roughly
along the `EventKind` / `ServerState` boundary in `willow-state`, with the
rest of the codebase consuming whichever side it depends on.

## Repo

- GitHub: [github.com/intendednull/willow](https://github.com/intendednull/willow)

## Sources

- `crates/state/src/event.rs:280-468` — `EventKind`, all 22 chat variants
- `crates/state/src/lib.rs:53` — `ServerState` re-export
- `crates/state/src/materialize.rs` — chat-specific `apply_event`
- `crates/client/src/lib.rs:270-348` — `ClientHandle` with chat-shaped
  per-domain `StateActor`s
- `crates/web/src/lib.rs` — Leptos web UI bound to chat types
- `crates/agent/src/{lib,main}.rs` — MCP agent exposing `ClientHandle`
- `crates/replay/src/role.rs:11-15` — workers importing chat-specific
  `ServerState` / `EventKind`
- PR #636 (`/tmp/willow-pr-636.diff`) lines 32-94 — "from WASM plugin
  system to P2P app runtime" reframe + runtime profiles
- PR #636 lines 96-122 — apps as bundles of components
- PR #636 lines 124-171 — "UI is an app", default UI app's capability
  privilege, UI app catalog
- PR #636 lines 421-444 — `willow-state` splits, web becomes default UI
  app, workers become generic peer hosts
- PR #636 lines 575-598 — MVP acceptance criteria + demo-app candidates
- PR #636 lines 636-637 — `chat-server` migration deferred to child spec
