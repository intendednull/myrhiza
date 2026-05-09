# Determinism story

Holochain's correctness model rests on validation being **deterministic and pure**: every authority that runs the validation callback for the same op must reach the same result, or the immune system breaks ([concepts/7_validation](https://developer.holochain.org/concepts/7_validation/)).

## What's enforced

- **Integrity zome callbacks run in the `hdi`-only subset.** The host functions exposed to integrity code do not include time, randomness, agent activity reads, link queries (which change over time), source-chain writes, or remote calls ([build/zomes](https://developer.holochain.org/build/zomes/)).
- **Validation can fetch dependent records via `must_get_*`.** If a dependency is missing, the result is `Unresolved`, not `Invalid` — the op is held in a limbo queue and re-validated on receipt of the missing data. This is a great pattern: validation declares its dependencies, the host parks until they're available, then retries. Avoids partial-knowledge invalid-validations.
- **Source chains are strictly ordered, hash-linked, and signed.** Forks are detectable and produce warrants.

## What isn't enforced

- **Coordinator zomes are non-deterministic by design** — they read time, generate signals, do remote calls. This is fine because their output never feeds validation; it's purely the authoring path.
- **The distinction is enforced by the host, not by the language.** A misbehaving Wasmer host could leak time into integrity. The Component Model gives you a stronger story: import sets are part of the type, statically checkable at link.
- **"Strong eventual consistency" is the formal claim, but it's eventual not synchronous.** Two agents can both produce contradictory writes, both pass validation locally, and discover the conflict only when their ops gossip into the same neighborhood. Apps are responsible for designing for that — countersigning ([concepts/10_countersigning](https://developer.holochain.org/concepts/10_countersigning/)) is the framework-provided escape hatch when atomicity is required.

## The warrant response

Validation infraction → warrant published against author → warrant gossiped → peers block the author. This is the immune-system metaphor and it works, but warrants only stabilized in 0.6 ([upgrade-holochain-0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)) — i.e. the malicious-author response only became canonical in late 2025.

## Implications for Myrhiza

Same shape applies. State-apply components must be pure, non-state-apply components are free. Two strengtheners Myrhiza gets from Component Model that Holochain doesn't have:

1. **Determinism boundary checked statically.** WIT import lists are part of the component's type. A `state-apply` component declaring an import to a non-deterministic interface fails at link, not at runtime. Holochain can only fail at runtime if a misbehaving host binds the wrong fns.
2. **Cross-peer convergence via app-exported `state-digest()`.** Holochain's source chain hashes are determined by the host (by hashing the canonical wire format). Myrhiza requires apps to export `state-digest()` because raw memory hashes diverge across allocators / wasm engines. This is more rigorous: convergence is verified at the application semantic level, not the wire-encoding level.

Countersigning is worth borrowing wholesale as the atomic-multi-party primitive. See [`lessons.md`](lessons.md).

## HDI vs HDK host functions, exhaustively

The split is enforced at the host-function-import level: integrity zomes link against `hdi`, which only exposes a deterministic subset; coordinator zomes link against `hdk` and get everything ([Dev Pulse 121](https://blog.holochain.org/integrity-and-coordination-part-ways/)).

**HDI only** (deterministic, callable from `validate` and `genesis_self_check`):

| Group | Functions |
|---|---|
| Deterministic queries | `must_get_action`, `must_get_entry`, `must_get_valid_record`, `must_get_agent_activity` |
| Info | `dna_info`, `zome_info` (DNA/zome metadata, immutable for a given DNA hash) |
| Hashing | `hash_entry`, `hash_action` (pure, no I/O) |
| Crypto verification | `verify_signature`, `verify_signature_raw` |
| Logging | `trace`/`debug`/`info`/`warn`/`error` (host-side only, doesn't affect determinism) |

**HDK only** (non-deterministic, *not* callable from validation):

| Group | Functions |
|---|---|
| Non-deterministic queries | `get`, `get_details`, `get_links`, `get_links_details`, `count_links`, `query`, `get_agent_activity`, `get_validation_receipts` |
| Writes | `create_entry`, `update_entry`, `delete_entry`, `create_link`, `delete_link`, `create_cap_grant`, `update_cap_grant`, `delete_cap_grant`, `create_cap_claim` |
| Time/randomness | `sys_time`, `random_bytes`, `schedule` |
| Signaling | `emit_signal`, `send_remote_signal` |
| Network | `call`, `call_remote`, `call_remote_signed` |
| Crypto with private keys | `sign`, `sign_ephemeral`, `ed_25519_x_salsa20_poly1305_encrypt`, `x_25519_x_salsa20_poly1305_encrypt`, `x_salsa20_poly1305_encrypt`, `create_x25519_keypair` |
| Capability | `generate_cap_secret` |
| Context | `agent_info`, `call_info` (call-site dependent → forbidden in validation) |

`sys_time` and `random_bytes` are explicitly documented as coordinator-only ([build/miscellaneous-host-functions](https://developer.holochain.org/build/miscellaneous-host-functions/)). `agent_info` and `call_info` are excluded from HDI because their results depend on which authority is currently running validation, which would defeat convergence.

## The `must_get_*` family

Every `must_get_*` function shares one behavior: **failure to retrieve does not return `Invalid` — it returns `UnresolvedDependencies`**, which the host catches and parks the op in the validation limbo, retrying when the dependency arrives ([build/must-get-host-functions](https://developer.holochain.org/build/must-get-host-functions/)).

- **`must_get_action(ActionHash)`** → `SignedActionHashed`. The action *may* itself be invalid; this only proves it exists. Used when validation only needs to compare metadata across actions (e.g. "same author?").
- **`must_get_entry(EntryHash)`** → `EntryHashed`. Entry data only, ignoring metadata that varies (deletes, updates).
- **`must_get_valid_record(ActionHash)`** → `Record`. Returns the action+entry pair *and* short-circuits to `UnresolvedDependencies` if any visible authority reports the record invalid. The **inductive validation** primitive: validate Op N against dependency M only by trusting that M was already validated by its authorities ([docs.rs](https://docs.rs/holochain_deterministic_integrity/latest/holochain_deterministic_integrity/entry/fn.must_get_valid_record.html)).
- **`must_get_agent_activity(AgentPubKey, ChainFilter)`** → `Vec<RegisterAgentActivity>`. Returns a contiguous, bounded slice of an agent's chain. Crucially, this *ignores forks* — it walks one linear history. If the agent has forked, an authority may return either branch; what matters is that within a single call, the result is one consistent slice ([PR #1483](https://github.com/holochain/holochain/pull/1483)).

### `ChainFilter`

`ChainFilter` is the bounded-slice descriptor: a starting `ActionHash` (the top), an optional `until_hash` (the bottom), and an optional `take: u32` (max items). Does not query by sequence number — walks `prev_action` links from the top hash backward, which is what makes it deterministic: every authority retrieving the same `(top, until, take)` will produce byte-identical results regardless of when they ran the query, because the chain segment is content-addressed ([PR #1483](https://github.com/holochain/holochain/pull/1483)). The trick is filtering out warrant data and validity flags — those *would* be temporally variant. You get the raw sequence and nothing else.

## Inductive validation, formalized

Stance: when validating Op N, you *may* assume the validity of any dependency you fetch via `must_get_valid_record` — that fetch will itself fail to `UnresolvedDependencies` if the authorities holding the dependency haven't reported it valid yet ([build/validation](https://developer.holochain.org/build/validation/)). Validation rules don't need to recursively re-validate the entire dependency closure; each authority validates its slice and trusts the chain. "Inductive" in the literal sense: validity of N follows from validity of N-1 plus a local rule.

## Validation receipts

After an authority validates an op, it produces a `ValidationReceipt` (signed by the authority) and:

1. Sends the receipt back to the op's author over the network ([concepts/7_validation](https://developer.holochain.org/concepts/7_validation/)).
2. Stores the op locally and serves it on `get`.
3. Gossips both the op and receipt to overlapping-arc peers for redundancy.

Authors collect receipts; reaching a (configurable) quorum of receipts is the cell's "your op is integrated" signal. `get_validation_receipts` is an HDK function that lets coordinator zomes inspect them programmatically. Receipts are themselves DHT data — they propagate via the same gossip plane, so any peer can prove an op was validated by showing the signed receipts.

## Countersigning protocol mechanics

A countersigning session ([concepts/10_countersigning](https://developer.holochain.org/concepts/10_countersigning/)):

1. **Preflight request.** Initiator builds `PreflightRequest { signing_agents, optional_signers, enzyme?, app_entry_hash, action_stub, time_window }` and sends it to all parties.
2. **Preflight response & lock.** Each participant validates feasibility, **locks their source chain** at its current head, and returns `PreflightResponse { signature, agent_state }`. Locked chains reject any non-countersigning commits.
3. **Tentative publish.** All participants assemble the full set of preflight responses, build the shared `Action`, sign it, and pre-publish to DHT authorities — these authorities act as witnesses without integrating yet.
4. **Enzyme step (if nominated).** A neutral party (the "enzyme") collects all signatures, signs over the complete signature set, and broadcasts the signed bundle. Makes signature collection an all-or-nothing op — the enzyme prevents partial commits from clever counterparties.
5. **Permanent commit.** With the full signature bundle in hand, each party commits the action to their (still-locked) chain and publishes for real.
6. **Unlock.** Source chains unlock; subsequent commits resume.
7. **Timeout / dropout.** If the time window elapses without the full set, conductors **discard the tentative action and unlock the chain as if nothing happened** — no half-committed state.

Optional signers ([Dev Pulse 122](https://blog.holochain.org/quantised-gossip-optional-countersigners/)) extend this to multi-sig with `> 50%` quorum: not every named party must sign, but a majority must. Atomicity comes from: (a) chain locks preventing concurrent writes, (b) the time window providing an upper bound on uncertainty, and (c) the enzyme's all-or-nothing signature distribution making partial-knowledge attacks costly.

## Genesis self-check and the first three actions

Every cell's source chain begins with three system actions in this order ([concepts/3_source_chain](https://developer.holochain.org/concepts/3_source_chain/), [build/genesis-self-check-callback](https://developer.holochain.org/build/genesis-self-check-callback/)):

1. **`Action::Dna`** (seq 0). Records the DNA hash. No `prev_action`. Marks the start of the chain.
2. **`Action::AgentValidationPkg`** (seq 1). Carries the *membrane proof* — application-supplied data (invite code, signature, etc.) that gates network membership.
3. **`Action::Create`** of the agent's `AgentPubKey` entry (seq 2). The agent's identity, hashed and signed.

The `genesis_self_check(data)` callback is special because it runs **before the cell joins the network**. It cannot call `must_get_*` (no DHT access yet). Its job is purely catch-typo validation on the membrane proof: verify the proof is well-formed locally so users get an immediate error rather than waiting for network rejection. The full membrane validation runs later when peers receive the `AgentValidationPkg` op via normal DHT channels.

## Sources

- [Concepts — Validation](https://developer.holochain.org/concepts/7_validation/)
- [Concepts — Countersigning](https://developer.holochain.org/concepts/10_countersigning/)
- [Concepts — Source Chain](https://developer.holochain.org/concepts/3_source_chain/)
- [Build Guide — Validation](https://developer.holochain.org/build/validation/)
- [Build Guide — must_get Host Functions](https://developer.holochain.org/build/must-get-host-functions/)
- [Build Guide — Miscellaneous Host Functions](https://developer.holochain.org/build/miscellaneous-host-functions/)
- [Build Guide — Genesis Self-Check Callback](https://developer.holochain.org/build/genesis-self-check-callback/)
- [Build Guide — Zomes](https://developer.holochain.org/build/zomes/)
- [Dev Pulse 121: Integrity and Coordination Part Ways](https://blog.holochain.org/integrity-and-coordination-part-ways/)
- [Dev Pulse 122 — Quantised Gossip & Optional Countersigners](https://blog.holochain.org/quantised-gossip-optional-countersigners/)
- [PR #1483 — must_get_agent_activity](https://github.com/holochain/holochain/pull/1483)
- [docs.rs — must_get_valid_record](https://docs.rs/holochain_deterministic_integrity/latest/holochain_deterministic_integrity/entry/fn.must_get_valid_record.html)
- [Upgrade 0.5 → 0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)
