**Date:** 2026-05-09
**Status:** active
**Subject:** Willow — authority discipline and the pre-check-equals-apply mechanic

Willow's load-bearing authority insight: there is exactly one place that
decides whether an author may emit a given event, and that place is
called both before signing (pre-check) and during replay (apply). The
two cannot drift because they are the same code.

See also: [state-machine.md](state-machine.md),
[determinism.md](determinism.md), [glossary.md](glossary.md),
[README.md](README.md).

## Single-authority discipline (shipped)

`willow-state` is the single source of truth
(`docs/specs/2026-04-12-state-authority-and-mutations.md` §"Single
source of truth"). All authority checks live in two places, both inside
`willow-state`:

- `check_permission(state, author, kind)` — pure read-only check
  (`crates/state/src/materialize.rs:116-153`).
- `required_permission(kind) -> Option<Permission>` — the EventKind →
  Permission table (`materialize.rs:297-346`).

`apply_event` calls `check_permission` first
(`materialize.rs:163-165`). If the check fails, the event is `Rejected`
— the DAG retains it (because the chain has already committed by the
time apply runs on a remote peer) but the state does not advance.

**No other crate may enforce trust.** Client, UI, worker, agent — all
hold a `ServerState` and consult it through the same predicates
(`is_admin`, `has_permission`). UI gating, MCP-tool gating, and
worker-side filtering are all defense-in-depth on top of the canonical
check.

## The pre-check-equals-apply mechanic

The same `check_permission` runs:

- Before signing a local event (`ManagedDag::create_and_insert` first
  pre-checks; if the author lacks permission, no event is signed, no
  seq advances). See `state-authority-and-mutations.md` §"Local mutation
  flow".
- During replay of a remote event, inside `apply_event`.

This is not "shared logic by convention" — it is **literally the same
function called twice**. Pre-check fails closed: a rejected pre-check
prevents the event from ever entering the DAG, so the per-author chain
does not get stuck on a dead event that no peer would accept.

This is the load-bearing mechanic Myrhiza inherits and has lifted
verbatim into its CLAUDE.md ("Pre-check is mechanically the same WASM
function as `state-apply`, called by the kernel in dry-run mode. Not a
convention.").

## Permission tiers (shipped, chat-specific)

`state-authority-and-mutations.md` §"Permission tiers" defines six tiers,
each implemented in `apply_event` / `check_permission`:

| Tier | Events | Check |
|------|--------|-------|
| Governance (vote) | `Propose`, `Vote` | `is_admin` + threshold |
| Admin-only (direct) | `GrantPermission`, `RevokePermission`, `RenameServer`, `SetServerDescription` | `is_admin` |
| Permission-gated | `Message`/`FileMessage`/`EditMessage`/`DeleteMessage`/`Reaction` (`SendMessages`); `Create/Delete/RenameChannel`, `RotateChannelKey` (`ManageChannels`); `CreateRole`/`DeleteRole`/`SetPermission`/`AssignRole` (`ManageRoles`) | `has_permission` |
| Member-only | `SetProfile`, `UpdateProfile`, `PinMessage`, `UnpinMessage`, `ChannelRevive` | `state.members.contains_key(&author)` (gate lives in `apply_mutation`, defense-in-depth — issue #177) |
| Per-identity preference | `MuteChannel`, `MuteGrove` | none — preferences are not shared state |
| Genesis | `CreateServer` | structural (must be first event) |

Admin status is **not** a `Permission` variant — it lives in
`ServerState.admins: BTreeSet<EndpointId>`. This separation is
structural: there is no `GrantPermission { permission: Admin }` shape,
so escalation-via-direct-grant is impossible by construction
(`event.rs:50-52` comment, "Does NOT include admin status").

The `_ => None` arm in `required_permission` is intentional but
treacherous — bug #109 was a new variant landing in the catch-all and
getting zero enforcement. The fix shipped is a comment block listing
every variant that returns `None` and *why* (`materialize.rs:315-344`).
Each must be checked elsewhere (governance block, admin block, member
gate) or be unrestricted by design.

## Owner-rooted-with-governance (shipped)

The genesis author becomes the sole initial admin
(`server.rs:106-143`). The owner is the **root of trust**:
`check_and_apply_proposal` lets the genesis author push governance
actions through unilaterally, bypassing the vote threshold
(`materialize.rs:213-218`). This makes the owner non-removable — there
is also a "prevent 0-admin state" guard in `RevokeAdmin` / `KickMember`
that refuses to remove the last admin (`materialize.rs:240-256`).

All other privilege changes go through `Propose` → `Vote` →
auto-apply-on-threshold. Vote thresholds: `Majority` (default),
`Unanimous`, `Count(n)` (`event.rs:230-239`). When the threshold is
met, `apply_proposed_action` materializes the result
(`materialize.rs:229-262`).

`Vote` events must include their `proposal` hash in `deps` or `prev`
(`dag.rs:223-230`). This is enforced at insert time — without this
structural link, topo-sort could place a vote before its proposal,
breaking the threshold logic.

## Trust model (shipped + designed)

- **Identity = Ed25519 keypair.** Private keys never leave the local
  machine. Every event is signature-verified at insert
  (`dag.rs:163`, `event.rs:548-577`).
- **Invite trust lists are suggestions, not guarantees.** A joining
  peer's invite carries the inviter's view of who the admins were; the
  joiner verifies state by gossiping with multiple peers and adopting
  the majority-agreed DAG (per the design discussion in
  `per-author-merkle-dag-state-design.md`).
- **Relays are not authoritative.** A relay can only sync history if it
  holds the `SyncProvider` permission, granted explicitly via
  `GrantPermission` (`event.rs:55-56`). PR #636 §"Constraints we
  accept" doubles down: "Relays are gossip-driven, not state-driven.
  The relay never inspects app payloads, never materializes state,
  and never runs WASM."
- **`SyncProvider` does not confer write authority** — it only marks a
  peer trusted to *serve* history. Forging events still requires the
  author's signing key.

## What changes under PR #636

PR #636 §"What stays the same about Willow" commits the
pre-check-equals-apply mechanic into the runtime explicitly:

> Today's centralized `required_permission()` table runs in trusted
> in-process Rust; under the runtime the kernel calls into the app's
> state component to ask "may this author emit this event under the
> current state?" before signing. … pre-check is not "shared logic by
> convention" — it is mechanically the same WASM function as `apply`'s
> authority verdict, called by the kernel in dry-run mode against a
> hypothetical post-state.

PR #636 §"Constraints we accept" pins the failure mode:

> Pre-check fails closed. When the kernel's dry-run pre-check panics,
> exhausts fuel, traps, or loops up to the deterministic budget, the
> user-action that triggered it is rejected and the event is *not*
> signed.

The stated reason is exactly the one above: rejected events accumulate
in the per-author DAG and cannot be removed without breaking the chain.

## What Myrhiza inherits

**Lifts directly** (mechanism, not policy):

- **Pre-check-equals-apply.** One WASM export per app, called by the
  kernel both before signing and during replay. Already canonical in
  Myrhiza CLAUDE.md.
- **Single-authority discipline.** Per app, exactly one place owns the
  authority verdict. The kernel does not encode any cross-app authority
  primitive (PR #636 §"Open questions" lists "cross-app authority
  composition" as deferred to v2).
- **Pre-check fails closed.** Panics, fuel exhaustion, traps → reject.
- **Vote-binding-via-deps.** Any structural causal binding (a vote
  references its proposal) belongs in the app's DAG-insert validation.

**Lifts conceptually** (general patterns Myrhiza apps may choose):

- Owner-rooted-with-governance (one valid pattern; Myrhiza apps can
  pick this, flat membership, capability-secure delegation, or
  whatever).
- Defense-in-depth handler-local checks even when the central table
  lists a variant as unrestricted (issue #177 lesson).
- Two-tier admin-vs-permission split (admin status is structurally
  distinct from a `Permission` variant).

**Does not lift** (chat-specific):

- The `Permission` enum's specific variants.
- The `ProposedAction` / `VoteThreshold` shape.
- The owner-override carve-out for governance.
- The chat-tuned tier table.

These move into the per-app `state-apply` component. The kernel does
not know what "admin" or "permission" mean — it only knows that an app
exports an authority predicate that the kernel calls.

**Re-evaluates**:

- Whether the kernel offers any common-helper for vote-threshold logic
  (probably no — Myrhiza apps with no governance shouldn't pay for it).
- Whether `SyncProvider`-equivalent (relay trust grant) is an app
  concern or a kernel-level capability granted at the topic-membership
  layer. PR #636's relay-is-dumb stance suggests app-level.

## Repo

- GitHub: [github.com/intendednull/willow](https://github.com/intendednull/willow)

## Sources

- `crates/state/src/event.rs:50-77, 217-239` — `Permission` enum,
  `ProposedAction`, `VoteThreshold`, the structural admin/non-admin
  split.
- `crates/state/src/materialize.rs:116-153, 161-202, 213-218,
  240-256, 297-346` — `check_permission`, `apply_event`,
  owner-override, 0-admin guard, `required_permission` table and
  catch-all comment.
- `crates/state/src/server.rs:106-202` — admin set, governance fields,
  `is_admin` / `has_permission` / `is_sync_provider`.
- `crates/state/src/dag.rs:163, 223-230` — insert-time signature
  check, vote-must-link-proposal structural rule.
- `docs/specs/2026-04-12-state-authority-and-mutations.md` (full file,
  151 lines) — the canonical authority spec.
- `docs/specs/2026-04-01-per-author-merkle-dag-state-design.md`
  §"Server Identity and Genesis Event", §"Governance Model".
- PR #636 `docs/specs/2026-04-27-willow-runtime/README.md` §"Capability
  model", §"What stays the same about Willow", §"Constraints we
  accept".
