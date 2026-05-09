**Date:** 2026-05-09
**Status:** active
**Subject:** wasmCloud — WIT interface contracts (`wasmcloud:*`, `wrpc:*`, `wasi:*`)

# Interface Contracts: `wasmcloud:*`, `wrpc:*`, and the `wasi:*` Boundary

## Three WIT package families

A wasmCloud component sees three distinct namespaces in its imports:

| Namespace | Owner | Scope | Stability |
|---|---|---|---|
| `wasi:*` | WebAssembly CG | Universal — any CM runtime | Mixed; `wasi:http@0.2.0` stable, `wasi:keyvalue@0.2.0-draft` not |
| `wasmcloud:*` | wasmCloud project | wasmCloud-host-specific extensions | Per-package; `wasmcloud:bus@1.0.0`, `wasmcloud:secrets@0.1.0-draft`, `wasmcloud:messaging@0.2.0` |
| `wrpc:*` | Bytecode Alliance | Cross-host RPC primitives | `wrpc:rpc@0.1.0` (draft) |

The three answer different questions: **what** the component wants (`wasi:*`), **how** wasmCloud satisfies it specifically (`wasmcloud:*`), and **how** the call gets there if it crosses a host (`wrpc:*`).

## `wasi:*` — what wasmCloud uses upstream

From `wash-runtime`'s `wkg.lock` (verified 2026-05-09):

```
wasi:blobstore  0.2.0-draft
wasi:config     0.2.0-rc.1
wasi:http       0.2.0
wasi:keyvalue   0.2.0-draft
wasi:logging    0.1.0-draft
```

wasmCloud was a primary contributor to `wasi:keyvalue`. The proposal lives at `WebAssembly/wasi-keyvalue` (51 stars; updated 2026-04-02). **Verified status:** Phase 2 ("Feature Description Available"), per the upstream README. **Correction:** brief asked to verify "current upstream status" — wasi-keyvalue has *zero* tagged releases on GitHub Releases; the most recent tag is `v0.2.0-draft` (still the same 0.2.0-draft as in 2024). This is a Phase-2 proposal that has not advanced.

Champions per the upstream README: Dan Chiarlone, David Justice, Jiaxiao Zhou. Portability criteria require two implementations against open-source key-value stores and two against proprietary ones (DynamoDB / Cosmos / Firestore class) on Linux/macOS/Windows.

The shape of `wasi:keyvalue/store`:

```wit
interface store {
    variant error {
        no-such-store,
        access-denied,
        other(string)
    }
    open: func(identifier: string) -> result<bucket, error>;
    resource bucket {
        get: func(key: string) -> result<option<list<u8>>, error>;
        // ... set, delete, exists, list-keys
    }
}
```

Note the `access-denied` error variant — explicit acknowledgement that the host gates access, but with no protocol-level mechanism to *grant* access. Authority lives entirely outside the WIT.

## `wasmcloud:*` — host-specific WIT

These are the packages that say "this only makes sense on a wasmCloud host." Verified set (from `wasmCloud/go/component/wit/deps`):

- `wasmcloud:bus@1.0.0` — runtime link control.
- `wasmcloud:secrets@0.1.0-draft` — secret retrieval.
- `wasmcloud:messaging@0.2.0` — message broker abstraction.

### `wasmcloud:bus@1.0.0`

The `bus` package is small and surgical:

```wit
package wasmcloud:bus@1.0.0;

interface lattice {
  resource call-target-interface {
    constructor(namespace: string, %package: string, %interface: string);
  }

  /// Set an optional link name to use for all interfaces specified.
  /// This is advanced functionality only available within wasmcloud and,
  /// as such, is exposed here as part of the wasmcloud:bus package.
  /// This is used when you are linking multiple of the same interfaces
  /// (i.e. a keyvalue implementation for caching and another one for
  /// secrets) to a component
  set-link-name: func(name: string, interfaces: list<call-target-interface>);
}
```

Verbatim from the upstream WIT.

This is the "interface-typed actor" mechanism made explicit: a component can have *two* `wasi:keyvalue` imports, distinguished by link name, routed at runtime to two different providers. One link to a Redis bucket for caching, another to a Vault bucket for secrets. The component sees the same WIT type for both; the host's link table disambiguates.

### `wasmcloud:secrets@0.1.0-draft`

Two interfaces:

- `store.get(key: string) -> result<secret, secrets-error>` — returns a *resource handle*, not a value.
- `reveal.reveal(s: borrow<secret>) -> secret-value` — separate operation to extract the bytes.

Verbatim from the upstream WIT [paraphrased to fit]:

> "A secret is a resource that can only be borrowed. This allows you to pass around handles to secrets and not reveal the values until a component needs them. You need to use the reveal interface to get the value."

The split-handle design is a real ocap-flavored move. A component can hand a `borrow<secret>` to a sub-component without exposing the bytes; only callers who hold the `reveal` import can dereference. This is the closest thing in the wasmCloud WIT family to a sealed-reference pattern. It's still backed by host-side authority (the host decides who gets the `reveal` import), but the *type-level* shape encodes the principle. RFC #2190 is the design context (`https://github.com/wasmCloud/wasmCloud/issues/2190`).

For Myrhiza, this is the most directly transplantable WIT pattern in the `wasmcloud:*` family.

### `wasmcloud:messaging@0.2.0`

```wit
package wasmcloud:messaging@0.2.0;

interface types {
    record broker-message {
        subject: string,
        body: list<u8>,
        reply-to: option<string>,
    }
}

interface handler {
    handle-message: func(msg: broker-message) -> result<_, string>;
}

interface consumer {
    request: func(subject: string, body: list<u8>, timeout-ms: u32)
        -> result<broker-message, string>;
    publish: func(msg: broker-message) -> result<_, string>;
}
```

Verbatim, from `wasmCloud/messaging/wit/messaging.wit`.

Pure NATS-flavored pub/sub semantics with a `request` operation grafted on. No queue groups, no JetStream, no streams; this is the lowest common denominator. The `handler.handle-message` interface is the export side — a component declares it can receive messages, the host wires it to a subscription.

Why is this `wasmcloud:*` and not `wasi:messaging`? There is a `wasi:messaging` proposal in flight, and wasmCloud's `wasmcloud:messaging@0.2.0` predates it. The expected migration path is for components to switch to `wasi:messaging` once it stabilizes; `wasmcloud:messaging` is the bridge.

## `wrpc:*` — cross-host RPC primitives

`wrpc:rpc@0.1.0` (verified verbatim from `crates/runtime-wasmtime/wit/deps/rpc/rpc.wit`) defines:

- `interface error` — an RPC-layer error resource with `from-io-error` conversion to `wasi:io/error`.
- `interface context` — a `context` resource (constructor `context.default()`) that carries per-invocation metadata.
- `interface transport` — `incoming-channel` / `outgoing-channel` / `invocation` resources for the indexed-channel model described in [`wrpc.md`](wrpc.md).
- `interface invoker` — the actual invocation entry point:
  ```wit
  invoke: func(cx: context, instance: string, name: string,
               params: list<u8>, paths: list<list<option<u32>>>) -> invocation;
  ```

The `invoker.invoke` signature is the wRPC equivalent of "make a remote call": pass a context, an instance string, a function name, the synchronous parameter buffer, and the set of indexed paths the caller plans to stream on.

This is a *very* low-level interface. Components do not normally import `wrpc:rpc` directly; they import `wasi:keyvalue` or `wasmcloud:messaging` at the WIT layer, and `wrpc:rpc` is what the *host* uses to satisfy that import when the implementation lives on a different host. The `wrpc:rpc` package is the wRPC-layer plumbing, not an application-developer API.

## How a component author chooses

**`wasi:keyvalue@0.2.0-draft` vs `wasmcloud:keyvalue`:** there is no `wasmcloud:keyvalue`. The brief asked the component author to choose between them; in practice the only WIT a wasmCloud component imports for KV is the upstream `wasi:keyvalue`. wasmCloud-specific KV behaviors (NATS-backed buckets, Redis-backed buckets, Vault-backed secret-flavored buckets) are providers/plugins behind that one import.

The actual choices a wasmCloud component author makes:

| Capability | What you import | Why |
|---|---|---|
| Key-value | `wasi:keyvalue@0.2.0-draft` | Upstream proposal; runtime substitutes implementation |
| Messaging (today) | `wasmcloud:messaging@0.2.0` | `wasi:messaging` not yet stable enough |
| Secrets | `wasmcloud:secrets@0.1.0-draft` | No upstream `wasi:secrets` proposal yet |
| Multiple instances of same KV | `wasi:keyvalue` × 2 + `wasmcloud:bus.set-link-name` | Disambiguate by link name |
| HTTP | `wasi:http@0.2.0` | Stable upstream |
| Config | `wasi:config@0.2.0-rc.1` | Upstream RC |
| Logging | `wasi:logging@0.1.0-draft` | Upstream draft |

Pros of staying on `wasi:*`: portability across runtimes (Wasmtime, Spin, Hyperlight). Cons: drafts move; you may need to update.

Pros of `wasmcloud:*`: features available *now* (e.g., `set-link-name` for multi-instance routing has no upstream equivalent). Cons: lock-in to wasmCloud-shaped semantics. The `wasmcloud:*` family is in `wasmCloud/wasmCloud` repo's `wit/deps` and travels with the runtime.

## The "interface-typed actor" pattern

Quoting directly from the wasmCloud v2 announcement (2026-03-23) [verified verbatim where quoted]:

> "In v1, a component that imported `wasi:keyvalue` would have its call automatically routed over NATS via wRPC. This was convenient, but implicit, and often surprising."

The pattern that distinguished wasmCloud (v1 era) from Spin: components were addressed *by interface*, not *by name*. A call to `wasi:keyvalue/store.get(key)` did not name a target component; it named an interface. The lattice routed the call to *whichever provider on the lattice implemented that interface for that link*. Multiple providers could implement the same interface with different link names.

The link table looked roughly like:

```
component A  --(link-name=cache)-->    wasi:keyvalue --> redis-provider
component A  --(link-name=secrets)-->  wasi:keyvalue --> vault-provider
component B  --(link-name=default)-->  wasi:keyvalue --> nats-provider
```

A and B both import `wasi:keyvalue/store`. A imports it twice with different link names. The link is a *runtime-mutable* declarative wiring: change the link, change which provider satisfies the import, no recompilation.

This was wasmCloud's signature contribution to the CM application model. It is also exactly the pattern that **v2 walked back from**.

## Link definitions

In v1, link definitions were stored in NATS JetStream as part of lattice state. A wadm OAM manifest [paraphrased from migration docs] would express:

```yaml
spec:
  components:
    - name: my-component
      type: component
      properties:
        image: oci://...
      traits:
        - type: link
          properties:
            target: my-keyvalue-provider
            namespace: wasi
            package: keyvalue
            interfaces: [store, atomic]
            name: cache  # the link name
```

The host watched JetStream for link changes and rewired its Wasmtime Linker on the fly.

In **v2**, the link concept survives but is reframed. Per the v2 announcement: capability providers are gone, replaced by in-process *host plugins* and out-of-process *services*. The link table is now between components and plugins/services, expressed as Kubernetes CRDs (`WorkloadDeployment` etc.), not as NATS-stored OAM. The "runtime-mutable" property is now mediated by `kubectl apply`, not by a lattice control message.

## `wrpc:*` is what's *left* of distributed wasmCloud

After v2's collapse-to-in-process, the place where wRPC still lives in mainline wasmCloud is the **explicit cross-host case**: when an operator deliberately wires a component's import to a remote provider over NATS, the host instantiates a `wrpc:rpc.invoker` to satisfy the import. The default for a fresh v2 deployment is no wRPC at all — everything is in-process, plugin-mediated, microsecond-grade.

This means `wrpc:*` interfaces are now **opt-in plumbing for explicit distribution**, not the default substrate. That is a good outcome for honesty (the v1 model lied about latency) and a worse outcome for "transparent distribution" (which never really worked anyway).

## Implications for Myrhiza

### What to take

- **The split-handle secret pattern** (`wasmcloud:secrets`). Resource `secret` + separate `reveal` interface is a real ocap-flavored design. Myrhiza's equivalent for sealed cross-peer references should use the same shape: a borrowable resource handle whose dereference is a separate authority-gated operation.
- **The `set-link-name` idea, generalized.** A Myrhiza component that imports `myrhiza:state/store` may need to import it twice with different "names" — one bound to a local-only store, one bound to a peer-replicated store. The wasmCloud bus pattern is the right primitive.
- **The `wrpc:rpc` interface shape.** Even though wRPC's authority story is wrong for Myrhiza, the *shape* of `interface invoker` (context + instance + name + params + paths) is a clean abstraction for "make a typed cross-peer call with optional streaming." Myrhiza's equivalent — call it `myrhiza:rpc` — should look similar but add a *capability token* to the parameter list.

### What to leave

- **Implicit cross-host routing as the default.** This is the v1 sin that v2 explicitly walked back. Myrhiza's cross-peer calls must be *visibly cross-peer* in the WIT: a different interface, or at minimum a ceremony that makes the latency tier obvious. State-apply must never silently round-trip to a remote peer.
- **NATS-shaped pub/sub as the messaging primitive.** `wasmcloud:messaging` is fine as a wasmCloud-internal abstraction but its data model assumes a centralized broker. Myrhiza is peer-symmetric. The right messaging primitive here is closer to OCapN's `deliver` / `deliver-only` than to NATS subjects.
- **String-keyed link tables as the wiring mechanism.** Runtime-mutable WIT-import wiring works in wasmCloud because the lattice is centrally administrable. In Myrhiza, the equivalent must be either (a) per-peer local config (each peer wires its own imports), or (b) governed by signed capability tokens, never by a shared mutable namespace.

### The Myrhiza package family

A reasonable starting layout:

| Package | Analog | Purpose |
|---|---|---|
| `myrhiza:state` | (none — Myrhiza-specific) | `state-apply`, `state-propose` interface signatures |
| `myrhiza:bus` | `wasmcloud:bus` | Local link-name disambiguation; per-peer |
| `myrhiza:capability` | `wasmcloud:secrets`-flavored | Sealed-handle pattern for capability tokens |
| `myrhiza:rpc` | `wrpc:rpc` | Cross-peer typed call with capability-gated authorization |
| `myrhiza:peer` | (none) | Peer identity, signing, key-rotation |

`wasi:*` imports apply to Myrhiza components unchanged (`wasi:http`, `wasi:io`, `wasi:clocks`, `wasi:random`, `wasi:filesystem` for sandboxed disk). The ABI surface that matters is the `myrhiza:*` family — design it deliberately, document each addition in a spec, and treat each new host import as a determinism / sandboxing decision.

## See also

- [`wrpc.md`](wrpc.md) — the protocol underneath `wrpc:rpc`.
- Companion files: [`architecture.md`](architecture.md), [`capability-model.md`](capability-model.md), [`wrpc.md`](wrpc.md), [`comparisons.md`](comparisons.md), [`lessons.md`](lessons.md).
- Prior-art neighbors: [WASM Component Model](../wasm-component-model/), [Spritely OCapN](../spritely-ocapn/captp-and-ocapn.md).
- Upstream: [wasmCloud/wasmCloud](https://github.com/wasmCloud/wasmCloud), [wasi-keyvalue](https://github.com/WebAssembly/wasi-keyvalue), [bytecodealliance/wrpc](https://github.com/bytecodealliance/wrpc).
