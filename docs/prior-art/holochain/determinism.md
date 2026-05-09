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

## Sources

- [Concepts — Validation](https://developer.holochain.org/concepts/7_validation/)
- [Concepts — Countersigning](https://developer.holochain.org/concepts/10_countersigning/)
- [Build Guide — Zomes](https://developer.holochain.org/build/zomes/)
- [Upgrade 0.5 → 0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)
