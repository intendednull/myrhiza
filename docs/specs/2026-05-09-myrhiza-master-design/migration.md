**Date:** 2026-05-09
**Status:** draft
**Parent:** [README.md](README.md)
**Subject:** Myrhiza master design — Willow migration

# Migration: Willow → Myrhiza

## 16. Migration: Willow → Myrhiza

Willow continues to develop independently. Eventually, Willow refactors
onto Myrhiza — chat becomes one app among many on the runtime, the
Leptos web client becomes a `myrhiza-ui-leptos` instance, and Willow's
worker binaries (replay, storage, relay) become maintenance modules.

The migration is not a fork. Willow's existing codebase ships chat to
users; Myrhiza is a separate runtime project. Decisions in Willow
inform Myrhiza (the prior-art folder `prior-art/willow/` captures the
mapping); Myrhiza decisions are made fresh, re-evaluating each Willow
choice rather than blindly inheriting.

When Willow refactors onto Myrhiza, the chat product becomes
`willow-chat` — a Myrhiza app. Its state-apply contains the chat
semantics that today live in `willow-state`. Its interaction component
consumes the `myrhiza-ui-leptos` UI app. Its identity, encryption, and
permission concerns use Myrhiza primitives + modules
(IdentityScope, `myrhiza-permission-governance`,
`myrhiza-crypto-channel-key` or future `myrhiza-crypto-mls`).

**Architectural pieces enabling mechanical migration**:

- **Event-log shape (§4)** matches Willow's per-author Merkle DAG
  almost 1:1. Willow's `EventDag`, `materialize`, `HeadsSummary`,
  `PendingBuffer` map directly to Myrhiza primitives. Willow's existing
  event log is replayable through a chat-shaped `state-apply` WASM
  component. The `EventKind` enum (Willow's hard-coded chat
  semantics) becomes the chat-app's `state-apply` payload variant —
  no kernel work required.
- **Identity (§6)**: Willow's Ed25519 user keys reuse as
  `IdentityScope.long-term`. Existing chat servers become app
  instances; existing channel topic IDs translate via §4.6 formula
  with the chat-app's bundle hash + an instance seed derived from
  the existing server identity.
- **Permission model (§7)**: Willow's permission tiers (Owner,
  Admin, SyncProvider, etc.) become a `myrhiza-permission-governance`
  module that the chat app declares as a dep. Authority logic stays
  in app territory; the kernel hosts.
- **Encryption (§9)**: Willow's `seal_content` channel-key encryption
  becomes a `myrhiza-crypto-channel-key` module. Future MLS adoption
  is a module swap.
- **Browser parity (§14)**: dual-stack at v1 means Willow's existing
  Leptos web UI translates directly. The `myrhiza-ui-leptos` UI app
  is the Leptos client adapted to host other apps' interaction
  components.
- **Worker pattern (§12)**: Willow's `replay`, `storage`, `relay`
  binaries become maintenance modules. The deployment shape (operator-
  run peers configured with all maintenance modules) is preserved
  (§12.4).

**Migration timing**: target v1 (browser available from v1 ship).
Migration is *mechanical given the architecture above* — Willow's
team writes a chat-app bundle that uses Myrhiza primitives + modules,
then runs both Willow chat and Myrhiza chat side-by-side during the
cutover, then deprecates Willow chat.

**Migration mechanics specific to Willow** (event-log translation tool,
identity migration UX, channel-history-import flow) are a Willow-side
project planned separately when Willow is ready.


