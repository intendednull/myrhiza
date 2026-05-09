**Date:** 2026-05-09
**Status:** active
**Subject:** wasmCloud — wRPC, the WIT-derived inter-component RPC protocol

# wRPC: WIT-Derived Inter-Component RPC

## Origin and stewardship

wRPC began life inside wasmCloud as the "lattice protocol" — the wire format wasmCloud hosts used to invoke each other's components over NATS. It was upstreamed to the Bytecode Alliance as `bytecodealliance/wrpc` so that other Component Model runtimes could adopt the same wire format without inheriting wasmCloud's NATS-centric operational model.

Verified facts (from `gh api repos/bytecodealliance/wrpc`, fetched 2026-05-09):

- **Org:** `bytecodealliance` (NOT `wasmCloud`).
- **Stars:** 322; **created:** 2024-01-12; **updated:** 2026-05-09.
- **License:** `Apache-2.0 WITH LLVM-exception`. **Correction:** brief said `NOASSERTION`; verified by decoding `LICENSE` and reading `license.workspace` in `Cargo.toml`. `NOASSERTION` is the GitHub default for headers GitHub's licensee detector cannot pin to an exact SPDX template — the actual text is plain Apache-2.0 with an LLVM-exception clause inherited from the Bytecode Alliance house style.
- **Default branch:** `main`. Not archived. Active.
- **Latest published crate `wrpc`:** `0.16.0` (2025-11-18); previous `0.15.0` (2025-05-23). The in-tree workspace is already at `0.17.0`, so a release is in flight but not on crates.io as of 2026-05-09.
- **Transport crates published:** `wrpc-transport 0.28.4` (2025-03-12), `wrpc-transport-nats 0.30.0` (2025-11-18), `wrpc-transport-quic 0.5.0` (2025-05-23), `wrpc-transport-web 0.2.0` (2025-05-23). Note the divergence between the in-tree workspace versions (e.g., `transport-nats 0.31.0`) and what's on crates.io.

The Bytecode Alliance home gives wrpc an explicit billing as a "Bytecode Alliance hosted project," not a wasmCloud project. The maintainer of record is Roman Volosatovs (`rvolosatovs@riseup.net`), per `workspace.package.authors`.

## What wRPC is

From the upstream README [paraphrased]: wRPC is a "component-native, transport-agnostic RPC protocol and framework based on WebAssembly Interface Types (WIT)." Its two stated use cases are out-of-tree Wasmtime plugins and distributed component communication. It is fully usable outside Wasm too, as a generic RPC framework.

The data on the wire is the [Component Model value definition encoding][cm-encoding]. There are no separate IDL types; if a function's signature is expressible in WIT, wRPC can move it across a transport. WIT bindings exist for Rust and Go; both static (codegen) and dynamic (runtime type introspection) modes are supported.

A wRPC invocation has three pieces:

1. **A WIT instance + name pair** — e.g., `wasi:http/outgoing-handler@0.2.1` + `handle`. This identifies what is being called.
2. **A parameter buffer** — the synchronous portion of the parameters, encoded with the CM value encoding.
3. **A set of asynchronous data channels**, addressed by reflective integer paths (e.g., `[0, 1, 0]` = "first parameter, second field, first element"), used for `stream<T>`, `future<T>`, and any value the caller chooses not to inline in the synchronous buffer.

The protocol version on the wire is `wrpc.0.0.1` (per `crates/transport-nats/src/lib.rs` const `PROTOCOL`). The official `SPEC.md` is also tagged `v0.0.1-draft.1`. Neither has hit a 1.0 — wRPC's wire format is still officially draft.

## Wire format and indexing

The same value can be transmitted two ways and the choice is made per-call:

- **Inline:** encode the value in the root parameter/result buffer using CM value encoding. `stream<T>` becomes `list<T>`; `future<T>` becomes a `variant { pending, ready(T) }` resolved as `ready`.
- **Asynchronous:** mark the value as `pending` in the root buffer and stream it on a separate channel keyed by its structural path.

The "indexing" rules from `SPEC.md`:

- Record fields indexed by WIT declaration order.
- Tuple members indexed by declaration order.
- Variant members indexed by declaration order.
- List elements indexed by appearance order.
- Stream elements indexed by appearance order.

`stream<T>` chunks are carried as a sequence of `list<T>` packets on the indexed channel, terminated by an empty list. `future<T>` resolves with a single `T` on its channel. Resources are encoded as opaque `list<u8>` blobs whose semantics are application-defined.

The reflective-path scheme is the heart of wRPC's claim to feel "native" to WIT: any future expansion of the WIT type system that introduces nested asynchrony can be addressed without changing the wire protocol, because the path scheme is recursive on the type structure.

## Transports

The transport is pluggable; the protocol does not bake in any particular network technology.

| Transport | Crate | Notes |
|---|---|---|
| NATS.io | `wrpc-transport-nats` | wasmCloud-default; uses NATS subject hierarchy for indexing |
| TCP | `wrpc-transport` (`net` feature) | one TCP stream per invocation, framed with a length-prefixed header |
| Unix Domain Sockets | `wrpc-transport` (`net` feature) | same framing as TCP |
| QUIC | `wrpc-transport-quic` | streams map to QUIC streams natively |
| WebTransport | `wrpc-transport-web` | for browser-side components |

NATS is the wasmCloud-default — and the only transport with a published-on-crates.io wRPC implementation that's used in production today. The other transports are real but lower-traffic.

### NATS subject scheme

For NATS, an invocation lifecycle (paraphrased from `SPEC.md`):

1. Server subscribes on `[<prefix>.]?wrpc.0.0.1.<wit-instance>.<wit-function>`.
2. Client publishes on that subject with the encoded parameters and a reply inbox `R_c`.
3. Server publishes an empty packet on `R_c` carrying its own reply inbox `R_s`.
4. Client streams pending parameters on `R_s.params.<path>`; server streams pending results on `R_c.results.<path>`.

Concrete example subjects from `SPEC.md`:
- `MBGL42DW...UZM.wrpc.0.0.1.wasi:http/outgoing-handler.handle`
- `default.wrpc.0.0.1.wasi:http/types@0.2.0.fields`
- `custom.wrpc.0.0.1.wasi:http/types@0.2.0.fields`

The leading token before `wrpc.0.0.1` is the lattice prefix; in wasmCloud v1 it was a base32-encoded host key, providing the only address-level isolation between multiple lattices on a shared NATS cluster. There is no protocol-level encryption; transport security is delegated entirely to the underlying transport (NATS auth, TLS, etc.).

## Linker::func_wrap vs wRPC

A useful frame: Wasmtime's `Linker::func_wrap` is host-local; wRPC is host-to-host.

In Wasmtime, when a component imports `wasi:keyvalue/store.get`, the runtime calls a function pointer registered via `Linker::func_wrap`. The pointer can do anything the host wants (call into Rust state, dispatch to a plugin, etc.). The component sees a normal WIT import; the call is a Rust function call with WIT lowering/lifting at the boundary.

wRPC plugs into the same seam. The wRPC runtime (`wrpc-runtime-wasmtime`) registers Linker entries that, instead of dispatching to a local function pointer, encode the call onto a transport, await a response, and lift the result back into the component's memory. From the component's perspective there is no difference — it imported `wasi:keyvalue/store.get` and got a value back.

This is why wasmCloud was able to make distributed calls "transparent" in v1: components didn't know whether their `wasi:keyvalue` import was satisfied by an in-process plugin or by a remote NATS-mediated provider. **It is also why wasmCloud has now reversed that policy in v2** — see *Implications* below.

## Async and streaming

wRPC's WIT support extends to the in-flight-but-unstandardized native async types `stream<T>` and `future<T>`. The README says: "wRPC fully supports the unreleased native [WIT] `stream` and `future` data types along with all currently released WIT functionality."

Mechanics:

- `stream<u8>` parameter → caller can stream on the indexed channel as the producer drives chunks; receiver drains the channel.
- `future<T>` return → callee writes one value to the indexed channel; caller awaits.
- Bidirectional streams in a single invocation are supported because every transport must be multiplexed (per the spec, "wRPC transports MUST allow for bidirectional concurrent transfer of multiple data streams"). NATS gets this for free; TCP/UDS use the framed-stream specification.

Compared to **WASI Preview 3 native async** (the ongoing effort to bake `stream`/`future` into the Component Model itself, currently three RCs cut Jan/Feb/Mar 2026 with no final): wRPC is *transporting* the same logical types over the wire. P3 standardizes how those types behave at the *language/runtime* level. The two are complementary — once P3 is final, a wRPC invocation of a P3 component will lower P3-async values to wRPC-async channels.

## Identity and authentication

wRPC has no built-in authentication, no built-in caller identity, and no capability-style authority discipline.

What it has:

- **WIT instance/name routing** — calls are typed by interface, but there is no mechanism in the protocol for the callee to verify "this caller may invoke this interface."
- **Transport-level auth** — NATS auth (NKey, JWT, mTLS), QUIC TLS, etc. The wRPC layer trusts whatever the transport tells it.
- **Out-of-band lattice prefixes** — the per-lattice subject prefix on NATS is a coarse-grained tenant boundary, not a per-call authorization signal.

There is no equivalent of an ocap "swiss number" or bearer-style unguessable reference. There is no per-call attenuation. There is no "the callee receives a sealed envelope only it can open." Authority over which WIT instances a component may import is enforced by the *host* (via Linker construction) and by the *configuration* of which subject prefixes a wRPC client is allowed to publish to — both of which are out-of-band relative to wRPC.

This is the point in the brief that needs to be flat: **wRPC is interface-typed, not capability-typed in the ocap sense.** It is closer in spirit to gRPC + a service mesh than to CapTP. Trust comes from mesh policy, not from possession of an unguessable reference.

For a runtime like Myrhiza that wants ocap discipline on cross-peer calls, wRPC's wire format is reusable but its authority story is not — see `[wRPC vs Spritely CapTP](../spritely-ocapn/captp-and-ocapn.md)` for the contrast.

## WIT package versioning

Versioning shows up at two layers:

1. **Subject naming.** When the WIT interface is versioned (e.g., `wasi:http/outgoing-handler@0.2.1`), the version becomes part of the subject. A `0.2.0` server and a `0.2.1` client publish on different subjects and never collide.
2. **Encoding compatibility.** The CM value encoding is structural; if two interfaces are bit-for-bit type-compatible, the encoding is interoperable. WIT's own semver rules govern when this is true.

There is no negotiation handshake. A client either knows the exact instance + version it wants, or it doesn't. Discovery (which versions are available on a lattice) is delegated to a higher layer — in wasmCloud v1, that was wadm. In Myrhiza terms, this means version mismatch shows up as a "no subscriber" error, not as a typed protocol-level failure.

## The Spin connection

The brief said Spin uses wRPC under the hood. **Correction:** verified false. A code search across `fermyon/spin` (6.4k stars, active, 2026-05) and the `spinframework/spin` mirror finds zero references to `wrpc`, `bytecodealliance/wrpc`, or any wRPC transport crate. Spin's inter-component story uses a different mechanism (Spin's own component composition + native Wasmtime Linker imports for service chassis). If wRPC adoption beyond wasmCloud was ever real, it has not landed in Spin's mainline. The README's framing as a Bytecode Alliance project should be read as aspirational — "available for any CM runtime to adopt" — not as describing actual cross-runtime usage.

Notable third-party users found via crates.io and code search:

- `LFDT-Nightstream/Starstream` — uses `wrpc-transport`.
- `lyric-project/lyric-runtime` — uses wRPC for distributed execution.
- `cosmonic-labs/*` — Cosmonic (the commercial wasmCloud sponsor) uses wRPC.
- `sevki/cloudflare-wrpc` — experimental Cloudflare-Durable-Objects wRPC transport.

The user base outside wasmCloud is small and mostly research/experiment grade.

## Performance

There are no published benchmarks for wRPC overhead vs in-process Linker calls that I could verify. The wasmCloud v2 announcement (2026-03-23) gives the qualitative comparison directly:

> "In v1, a component that imported `wasi:keyvalue` would have its call automatically routed over NATS via wRPC. ... A call that felt like nanoseconds in your mental model was actually subject to transport failure, message loss, and network latency."
> — wasmCloud v2.0 is here, 2026-03-23 [paraphrased: "in-process calls happen in nanoseconds by default" in v2]

That matches first-principles intuition: an in-process Linker call is a function pointer + WIT lowering, on the order of nanoseconds. A wRPC-over-NATS call is a network round-trip plus NATS broker hop plus serialization, on the order of hundreds of microseconds at best, milliseconds typically. The wasmCloud team's decision in v2 to make wRPC explicit rather than implicit is a direct response to operators being burned by this gap.

The wRPC repo has a `benches/` directory with a `reactor` benchmark, but no published numbers and no comparative baseline.

## Implications for Myrhiza

The honest summary: **wRPC's wire format is good prior art; its authority model is not what Myrhiza needs.**

What Myrhiza could borrow:

- **The "WIT calls all the way down" idea.** Cross-peer component invocation in Myrhiza could lower to a WIT-typed wire format rather than a custom event schema. This means peers exchange typed function calls, not opaque bytes-with-schemas. The CM value encoding is mature and already has Rust + Go bindings.
- **The reflective-indexing scheme for streaming.** When Myrhiza needs to send `stream<event>` between peers, the wRPC indexing scheme is a reasonable starting point that doesn't bake in any particular transport.
- **The transport-pluggable architecture.** Myrhiza's P2P transport will likely be Iroh-flavored (QUIC + relays). The wRPC transport trait (`wrpc-transport::Invoke` / `Serve`) is a reasonable shape for what Myrhiza's equivalent should look like — one that doesn't assume NATS, doesn't assume a central broker, and doesn't assume any particular discovery mechanism.

What Myrhiza must not borrow:

- **The "interface-typed but not authority-typed" model.** Myrhiza's design treats capabilities as the only host surface. If a peer can call `myrhiza:state/apply.handle-event` on another peer just by knowing the interface name, the capability model is dead. Cross-peer calls in Myrhiza must carry a *verifiable authorizing token* (signature over `(caller-peer-id, interface, args, timestamp)` at minimum), and the callee's `state-apply` must authenticate that token before applying anything. This is closer to OCapN/CapTP than to wRPC.
- **The lattice-prefix-as-tenancy assumption.** Myrhiza is peer-symmetric P2P. There is no central NATS, no shared subject space. The "everyone with publish access can reach every interface" model wRPC inherits from NATS does not translate.
- **The `wrpc.0.0.1` draft status.** If Myrhiza adopts a wRPC-shaped wire format, the spec must be pinned and versioned independently. wRPC itself has been in 0.0.x since 2024 and shows no sign of stabilizing.

The companion file `interfaces.md` covers what Myrhiza should and shouldn't reuse from the `wasmcloud:*` and `wrpc:*` WIT package families.

[cm-encoding]: https://github.com/WebAssembly/component-model/blob/main/design/mvp/Binary.md#-value-definitions

## See also

- Companion files: [`architecture.md`](architecture.md), [`capability-model.md`](capability-model.md), [`interfaces.md`](interfaces.md), [`history.md`](history.md), [`comparisons.md`](comparisons.md), [`open-problems.md`](open-problems.md), [`lessons.md`](lessons.md).
- Prior-art neighbors: [WASM Component Model](../wasm-component-model/), [wRPC vs Spritely CapTP](../spritely-ocapn/captp-and-ocapn.md).
- Upstream: [bytecodealliance/wrpc](https://github.com/bytecodealliance/wrpc), [SPEC.md](https://github.com/bytecodealliance/wrpc/blob/main/SPEC.md).
