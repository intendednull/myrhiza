# Capability model

Capabilities in Holochain are entries written to the agent's own source chain. There's nothing to "register" globally — a `ZomeCallCapGrant` is a local commitment by the grantor.

## Three access levels

| Access | Mechanism | Threat model |
|---|---|---|
| `Unrestricted` | Anyone can call. | Public endpoint. |
| `Transferable` | Caller presents a secret. | Bearer token; secret can leak. |
| `Assigned` | Caller presents secret AND signs the call with one of N authorized pubkeys. | Strongest; binds capability to identity. |

A grant carries a `tag` (for revocation), the access level, and a `GrantedFunctions` set listing zome functions covered. By default a cell's functions are not callable by anybody but the local UI — you must commit a grant to expose anything ([build/capabilities](https://developer.holochain.org/build/capabilities/)).

There's also an implicit "author" grant: the cell's owning agent can always call its own functions from the local UI websocket, scoped per-app via a session token.

## Compared to WIT-typed handles

Holochain capabilities are coarser than what the Component Model offers:

- **Granularity is per-zome-function, not per-resource-handle.** You cannot grant "this counter, but only the increment method, and only 5 calls."
- **Type safety lives in the JSON-serialized wire payload, not at the type system level.** Mismatched arguments fail at deserialization, not at link.
- **There is no notion of capability composition** — you cannot pass a capability INTO a zome call as a typed argument the way you can pass a `resource` handle through a WIT interface.

This is a meaningful gap. A Component-Model-native runtime can express object-capability discipline at the type level (handles are non-forgeable, scope-bounded, transferable as values). Holochain has to retrofit this with secrets, signatures, and source-chain entries.

## Why this matters for Myrhiza

The capability story is one of the clearest places where Component Model gives Myrhiza a strictly stronger primitive than Holochain. Bearer tokens leak. Pubkey-bound tokens are better but still wire-format objects, not type-level constraints. Resource handles in WIT are the actual ocap discipline encoded in the type system — a guest cannot manufacture or forge them, and they're transferable as ordinary values.

See [`lessons.md`](lessons.md) for the action items.

## Sources

- [Build Guide — Capabilities](https://developer.holochain.org/build/capabilities/)
- [Holochain Gym — Capability Tokens](https://holochain-gym.github.io/developers/intermediate/capability-tokens/)
