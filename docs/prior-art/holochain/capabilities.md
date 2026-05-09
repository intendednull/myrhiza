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

## Grant lifecycle in detail

A grant is a `ZomeCallCapGrant` system entry with three components: a **tag** (string, not unique, used for later querying/revocation), an **access level** (`Unrestricted` / `Transferable { secret }` / `Assigned { secret, assignees }`), and a **`GrantedFunctions`** set (either `All` or a list of `(ZomeName, FunctionName)` pairs) ([build/capabilities](https://developer.holochain.org/build/capabilities/)).

```
Grantor side                                Grantee side
============                                ============
generate_cap_secret() -> CapSecret
create_cap_grant(ZomeCallCapGrant {
  tag: "alice-friends",
  access: Transferable { secret },
  functions: { ("messaging","send_msg") }
})  -> writes Action::Create + Entry::CapGrant
                                            create_cap_claim(CapClaim {
                                              tag, grantor, secret
                                            }) -> stored on grantee's chain

                          [later, at call time]

call_remote(grantor, zome, fn, secret, payload)
   -> grantor's conductor:
        1. authenticate signature on call envelope
        2. query author's chain for unrevoked CapGrant
           matching (zome, fn) and secret
        3. for Assigned: also check caller pubkey ∈ assignees
        4. invoke zome function in ribosome
```

Lookup at call time happens on every invocation: the conductor queries the source chain for `CapGrant` entries whose `(zome, fn)` covers the requested call and whose secret matches. Grants live forever in the source chain (append-only), so revocation is **another** action: `delete_cap_grant(grant_action_hash)` writes an `Action::Delete` referencing the grant. The cap check skips deleted grants. The `tag` is convenience metadata for querying — Holochain doesn't enforce uniqueness — and it's the field UIs typically display for "what have I granted."

## The secret-exchange dance

There is **no framework primitive for delivering the cap secret to the grantee.** The grantor `generate_cap_secret()`s, commits the grant, then must hand the secret to Bob via some out-of-band channel: a remote signal, a different (already-`Unrestricted`) zome function, a DM in another app, a QR code, etc. The grantee then commits a `CapClaim` to remember `(tag, grantor, secret)` for later use.

This is a real footgun. Bearer secrets in remote signals are visible to both endpoints' cells (and on the wire, though Kitsune2 transport is encrypted peer-to-peer). For `Transferable`, anyone who learns the secret can call. `Assigned` mitigates by additionally requiring the call envelope to be signed by an enumerated pubkey, but the secret is still in plaintext on both chains. There has been [community discussion](https://github.com/holochain/holochain/issues/4708) about ergonomics here, but no built-in primitive for sealed delivery has shipped — left to apps.

## `call_remote` at the wire level

`call_remote(target_agent, zome, fn, cap_secret, payload)` from a coordinator zome:

1. Caller cell builds a `ZomeCall` envelope `{ cell_id, zome, fn, payload, cap_secret, provenance, signature, nonce, expires_at }`.
2. Lair signs the envelope hash on behalf of the caller agent.
3. Kitsune2 routes the message to the `target_agent`'s peer over iroh/tx5 transport (encrypted in transit).
4. The receiving conductor verifies the envelope signature, looks up an unrevoked `CapGrant` on the target cell's chain, then dispatches into the ribosome under the target agent's identity — "Alice's cell doing the work means everything that happens — reads and writes, signals — happens from Alice's perspective" ([concepts/8_calls_capabilities](https://developer.holochain.org/concepts/8_calls_capabilities/)).
5. Return value is msgpack-serialized and shipped back over the same connection.

For local zome-to-zome (`call(...)` within the same cell or sibling cells of the same agent), the conductor short-circuits: matching pubkeys means the **implicit author grant** applies, no explicit `CapGrant` needed.

## Granularity ceiling

Holochain has never landed finer-grained caps. The smallest unit is a `(zome, fn)` pair. You cannot:

- Restrict a grant to specific argument shapes ("only `transfer({ to: bob, amount: <= 10 })`").
- Hand out a per-resource handle ("this counter only, increment method only, expires in 5 calls").
- Compose grants — there's no `cap.attenuate()`.
- Pass a grant *into* a zome call as a typed value the way you pass a `resource` handle through a WIT interface.

Proposals for finer-grained personal security have been [floated](https://medium.com/holochain/bridging-and-laying-the-groundwork-for-fine-grained-personal-security-ddee29f4e196) but nothing matching ocap-style attenuation has shipped. The ceiling is structural: caps are source-chain entries (data), not type-system objects, so the runtime can only check membership of a `(zome, fn)` set — it has no way to type-check or attenuate a value that doesn't exist as a typed handle in the first place.

## Agent-activity reads

`get_agent_activity(agent, ChainQueryFilter, ActivityRequest)` is a coordinator-only call — there's no explicit cap mechanism gating "I want to see what Alice has been doing." Agent activity authorities (peers near Alice's key) serve activity reads to anyone who asks, subject only to: (a) public-entry visibility rules (private entries are excluded from agent activity payloads), and (b) whether the requesting peer has been blocked by a warrant. There is no per-zome or per-action-type filter on visibility — once an action is published as `RegisterAgentActivity`, its existence is public. By design (agent activity is the auditing surface) but worth flagging: there's no equivalent to "redact action sequence 47 from public agent activity."

## Sources

- [Build Guide — Capabilities](https://developer.holochain.org/build/capabilities/)
- [Concepts — Calls & Capabilities](https://developer.holochain.org/concepts/8_calls_capabilities/)
- [Holochain Gym — Capability Tokens](https://holochain-gym.github.io/developers/intermediate/capability-tokens/)
- [Issue #4708 — easier cap claim/grant queries](https://github.com/holochain/holochain/issues/4708)
- [Bridging and Fine-Grained Personal Security](https://medium.com/holochain/bridging-and-laying-the-groundwork-for-fine-grained-personal-security-ddee29f4e196)
