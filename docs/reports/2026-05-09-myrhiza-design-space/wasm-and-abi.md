**Date:** 2026-05-09
**Status:** brainstorming input — design-space mining, not a decision document
**Subject:** Cluster B — WASM substrate, ABI, composition, distribution, browser viability

# Myrhiza design space — WASM and ABI

This is the consolidated design-space mining for the WASM/ABI cluster, feeding the
Myrhiza master-spec brainstorming session on 2026-05-09. CLAUDE.md commits Myrhiza
to **WIT-shaped semantics as the eventual ABI** and to the four-profile model
(state-apply / state-propose / interaction / behavior). Willow PR #636 explicitly
left the **v1 ABI implementation path** unsettled. This report enumerates the
options, surfaces the load-bearing tradeoffs, and lists the questions the
brainstorming session must close.

The report is opinionated only where prior-art is dispositive; where Willow PR #636
already chose a direction, the option is named even when other options are
ruled-out, so the brainstorm can re-examine the ruling.

## Decision domains index

| # | Domain | Headline question |
|---|---|---|
| 1 | **ABI choice** | Full Component Model day-one, Extism with WIT-shaped subset, hybrid, or native-only? |
| 2 | **Cross-component composition** | Typed resource handles, opaque IDs, message-passing, or build-time composition? |
| 3 | **WIT contract design** | Coarse view-model interfaces vs fine-grained cap-discipline; versioning; resources |
| 4 | **Bundle format / app distribution** | `manifest.toml`+components+WIT vs OCI vs iroh-blobs; signing |
| 5 | **Browser viability** | jco transpile + sync submit-and-poll; ~350 KB shim floor; nested CM in browser |
| 6 | **Component instantiation / lazy-loading / hash-cache** | When; how cached; eviction; warm-up |
| 7 | **Async + concurrency model** (cross-cutting) | Preview2 sync-poll vs preview3 streams/futures; submit-and-poll vs reentrance |
| 8 | **Determinism + WASM features** | Permitted core-wasm features; floats/NaN/SIMD; fuel/epoch policy |

Domains 7 and 8 are cross-cutting consequences of choices in 1–3 but warrant their
own brainstorm slots.

---

## Domain 1 — ABI choice

CLAUDE.md commits to *WIT-shaped semantics*. The open question is the **v1
implementation backend**: do we ship full Component Model from day one, or pick a
simpler runtime now and migrate?

### Option 1A — Full Component Model day one (wit-bindgen + Wasmtime native + jco browser)

**Mechanism.** App authors run wit-bindgen (or `componentize-js` /
`componentize-py` / TinyGo wasip2) to produce a `.wasm` *component*. Native peers
embed Wasmtime 44.x with `wasmtime-wasi` 44.x; browser peers transpile components
via `jco transpile` and load the JS shim + extracted core wasm. Kernel host
imports are typed WIT interfaces under the `myrhiza:*` namespace; resources are
WIT `resource` types with `own<R>` / `borrow<R>` handles. Cross-component calls
are kernel-mediated through the typed `wasmtime::component::Linker`.

**Pros.**
- One ABI, one mental model, zero migration cost for app authors.
- Resources, borrows, world composition, futures/streams (when preview3 lands)
  all available natively. Capability-discipline at the binary layer is the
  designed-in behavior, not retrofitted.
- Tooling alignment: `wac`, `wkg`, `wasm-tools`, `wit-bindgen`, `cargo-component`
  (or `cargo build --target wasm32-wasip2`), `componentize-*` all target this
  path. Future proposals (Wasm GC, memory64, threads) integrate with CM, not
  Extism.
- Multi-language: Rust mature; JS via `componentize-js`; Python via
  `componentize-py`; Go via TinyGo wasip2; community Zig/Moonbit. Spin and
  wasmCloud-v2 prove this works in production.
- Determinism story is tractable: kernel restricts `state-apply` worlds via the
  forbidden-imports list + a wasm-validator pass.

**Cons.**
- **Heavier toolchain on day one.** Authors install `cargo-component` or
  `wasm-tools`, run a four-pass build (WIT → bindings → core wasm → component),
  and must understand WIT semver rules.
- **Browser CM is still maturing.** No native browser CM in any engine; only
  path is `jco transpile`. ~350 KB JS shim floor per app per browser peer. No
  async on the browser side until preview3 + jco preview3 transpile both ship.
- **Preview3 in flight.** WASI 0.3.0 has three RCs (latest 2026-03-15), no
  final. `Concurrency.md` is still being edited (PR #643 merged 2026-04-30,
  PR #641 emoji-gating sync future read/write merged 2026-04-27). Adopters are
  paying a dual-stack tax (cf. `pulseengine/rules_wasm_component#257`).
- **Per-language ergonomics uneven.** Rust path is mature; C++/Go/Moonbit/C#
  paths have open `wit-bindgen` bugs (#1604, #1587, #1585, #1582, #1518, #1516
  all 2026). `jco`'s resource handling has open bugs too (jco#1381, #1383
  2026-04-14).
- **Engine-bundling tax.** `componentize-js` produces a ~5 MB+ component
  bundling SpiderMonkey; `componentize-py` produces ~35 MB hello-world per
  componentize-py#98. P2P-distributable bundle sizes become a real
  constraint.
- **Build-time composition (`wac`) is alpha.** `wac` 0.10.0 (2026-04-17), 13
  releases total since 2024-04-16; open issues include O(N²) re-parsing
  (`wac#85`).

**Sources.** `prior-art/wasm-component-model/{spec,abi,wasmtime,preview-status,
critiques,open-problems,browser,languages,tooling}.md`. Spin's `componentize-*`
build path proven in production
(`prior-art/spin/sdks-and-tooling.md`). wasmCloud's CM-only post-v1.0 stance
(`prior-art/wasmcloud/architecture.md`).

**Closest precedent.** **wasmCloud-v2** and **Spin** both run full CM today.
Neither is browser-deployable; Myrhiza's browser-peer requirement is what makes
the toolchain story tighter than either of them.

### Option 1B — Extism v1 with WIT-shaped host signatures, migrate later

**Mechanism.** Use Extism (or any equivalent thin-WASM-plugin runtime) for v1.
Each host-call signature is **chosen to be WIT-expressible** (records, variants,
lists, strings, integers — no resources, no borrows, no worlds). Cross-component
composition is **kernel-brokered RPC by opaque ID only**: components hand the
kernel a string/u64 ID; the kernel resolves to a target component, marshals the
call, and returns a response. No imported resource types. No borrowed lifetimes.
No futures or streams.

Migration to full CM is a *real refactor* for app authors:

- **Resource handles replace ID lookups.** Today: `kernel.call("peer-id-12345",
  ...)`. Tomorrow: `peer.send(...)` where `peer: own<peer-handle>`.
- **Imported interfaces replace kernel-broker calls.** Today:
  `kernel.invoke("ui:list/render", &args)`. Tomorrow: `ui::list::render(args)`.
- **Borrows replace clone-and-pass.** Today: clone a payload before
  inter-component handoff. Tomorrow: pass `borrow<payload>` and let the runtime
  manage lifetime.

**Pros.**
- **Ship faster.** Extism's runtime is much smaller; integration story is days,
  not weeks. No four-pass build, no WIT toolchain on the app-author critical
  path.
- **Browser viability without jco.** Extism runs in a browser via plain core
  wasm + a small JS host. No ~350 KB jco shim floor, no preview2-shim
  dependency, no nested-CM-in-browser-WASM concern.
- **Async story sidestepped.** Submit-and-poll is the only pattern; we never
  need to integrate preview3 streams/futures on the app surface.
- **No engine-bundling cost (initially).** App authors compiling Rust → core
  wasm produce small artifacts; we don't need `componentize-js` in v1 because
  v1 doesn't claim CM-style world composition.

**Cons.**
- **Migration to full CM is not a "regenerate bindings" event.** PR #636
  acknowledges this directly: "expect to update at migration boundary, but not
  redesign your state machine." App authors will refactor every cross-component
  call site.
- **Harder to argue capability-discipline at the binary layer.** Extism does
  not have WIT resources; capability handles are opaque IDs the kernel
  validates by lookup, not by type. The "imports are the only host surface"
  invariant is maintained by convention plus kernel-side enforcement, not by
  the wasm-binary structure.
- **Loses the wider tooling ecosystem.** `wac`, `wkg`, `wit-bindgen`,
  `componentize-*` don't apply to Extism plugins; Myrhiza's tooling becomes
  Myrhiza-specific.
- **Tooling neutrality argument weakens.** A Rust app author compiling Spin
  components, wasmCloud components, and Myrhiza components today uses one
  build target (`wasm32-wasip2`); under (1B) Myrhiza diverges.
- **The migration boundary is a big PR for every app, simultaneously.**
  Coordinating "everyone migrates by date X" across an open ecosystem is
  expensive — the wasmCloud-v1→v2 reset (2026-03-22) is a worked example of
  the cost.

**Sources.** Willow PR #636 §"ABI commitments" (lines 450-477). Extism public
documentation (referenced indirectly via PR #636's framing).

**Closest precedent.** Willow's pre-runtime monolith has no WASM at all;
Extism-as-v1 is novel territory for Myrhiza. The closest analogue in the corpus
is **Holochain's preview1 + Wasmer** stance (cf. `prior-art/holochain/` per
`prior-art/wasm-component-model/preview-status.md`), which similarly trades
ecosystem alignment for shipping speed.

### Option 1C — Hybrid: CM-shaped Extism subset (a strict subset that's WIT-equivalent today)

**Mechanism.** Pick a subset of WIT that Extism *can* model losslessly (records,
variants, lists, strings, primitives), forbid the rest (resources, borrows,
streams, futures, world composition), and write Myrhiza's `myrhiza:*` host
imports **only inside that subset**. Host signatures are WIT files but the
*runtime* is Extism. App authors write WIT (so the binding generator looks like
wit-bindgen), but the produced artifact is an Extism plugin, not a CM component.

When CM matures (preview3 final, browser path stabilizes, Wasmtime LTS aligns),
*the WIT files don't change*; only the build pipeline swaps from Extism plugin
to CM component. Apps that respected the subset migrate cleanly.

**Pros.**
- **App authors write WIT from day one.** Their mental model already matches
  Myrhiza's destination ABI.
- **Migration is mechanical for apps that respected the subset.** No refactor
  of state machines or domain models.
- **Capability handles are still opaque IDs (Extism limitation), but the WIT
  file shows the *intended* resource type.** The binding generator emits an
  ID-based shim today; tomorrow it emits proper resource handles.

**Cons.**
- **The subset is severely cramped.** No resources means no capability handles
  at the type level. No borrows means hot-path memcopy. No streams/futures
  means submit-and-poll for everything. We're paying full migration tax later
  *and* full feature tax today.
- **Tooling burden.** We have to build the WIT-subset → Extism toolchain
  ourselves; nothing upstream provides it. This is novel work, not a borrow.
- **Authors will still refactor at migration.** Even with the subset, the
  semantic shift from "kernel-resolves-IDs" to "runtime-typed-resources" is
  observable in app code (lifetime annotations, ownership transfer points).

**Closest precedent.** None in the corpus. This is the engineered-by-Myrhiza
midpoint between (1A) and (1B).

### Option 1D — Native-only, no WASM in v1

**Mechanism.** Drop sandboxing in v1. Ship Myrhiza as a single Rust binary; apps
are Rust crates loaded via `dlopen` or compiled in. Add WASM in v2.

**Pros.** Ships fastest. No ABI question. No browser-CM concern.

**Cons.**
- **Capabilities are not enforceable at a binary boundary.** A native plugin
  has full process privilege.
- **Apps are not portable across kernels.** Browser peer can't load native
  plugins. iOS App Store, Android Play, signed-distribution stories are
  essentially excluded.
- **Determinism story collapses.** Native code on different platforms is not
  bit-identical; cross-peer state-apply convergence requires sandboxed,
  spec-determined execution.
- **Migration to (1A) is "rewrite every app"** — the entire premise of "apps
  are content-addressed P2P bundles" is undone.

**Sources.** Discussed implicitly in `prior-art/willow/runtime-vision.md`'s
"What changes about Willow" — workers go from trusted in-tree Rust to
"third-party-authored, attacker-influenceable WASM"; option (1D) is rejecting
that change.

**Closest precedent.** Willow today (chat-monolith). The very thing Myrhiza
exists to invert.

### Willow's current position

PR #636 §"ABI commitments" leans **(1B), Extism with WIT-shaped subset**, with
explicit migration-tax acknowledgement. Decision deferred to a "child spec on
ABI & runtime backends." Tentative, not committed.

### Myrhiza re-evaluation question

**Does Willow's lean toward (1B) survive Myrhiza's framing?** Two re-framings
shift the answer:

1. **Myrhiza has no chat-monolith to migrate from.** PR #636's "ship faster"
   pressure was about getting *Willow chat* off the runtime branch. Myrhiza
   starts from zero — there is no incumbent to ship-faster against.
2. **Browser CM is the binding constraint.** If Myrhiza accepts the ~350 KB
   jco shim floor (plus 5 MB+ for any JS-authored apps), (1A)'s costs are
   fully realized today. If Myrhiza tries to defer browser support to v2,
   (1B) looks better — at the cost of relegating browser peers to second-class
   status, which the Willow corpus calls out as unacceptable.

The brainstorming session needs to weigh: how much app-author tooling friction
on day one is acceptable to *avoid* a migration-tax day later? PR #636's
position implicitly answered "a lot" — Willow had a chat-shipping commitment.
Myrhiza answers "less" — there's no chat-shipping commitment.

### Open questions

- Which Rust-WASM target does Myrhiza standardize on? `wasm32-wasip2` (full CM)
  is one decision; `wasm32-unknown-unknown` (Extism-style) is another.
- If (1A): is `cargo-component` still the recommended Rust authoring tool, or
  is `cargo build --target wasm32-wasip2` (with `wit-bindgen` macro) the new
  path? `cargo-component` 0.21.1 was 2025-04-07 (per
  `prior-art/wasm-component-model/README.md`) — possibly stale.
- If (1B): which Extism version? Extism's own roadmap relative to CM
  alignment?
- If (1A): do we accept `componentize-js`'s 5 MB+ floor for JS apps? If not,
  is Rust the only first-class language?
- Cross-language semantics for the *same* state-apply component: do we permit
  state-apply in JS? In Python? In Go (TinyGo)? Each language toolchain has
  its own determinism risks.

---

## Domain 2 — Cross-component composition

How do components inside one Myrhiza app (e.g. state-apply + state-propose +
interaction) call each other? How do components across *different* apps cross
boundaries?

### Option 2A — Typed resource handles (full Component Model)

**Mechanism.** Each cross-component call uses WIT `resource` types with
`own<R>` / `borrow<R>` handles. Per-component i32 tables; the kernel owns
the table. A peer-handle, a state-stream subscription, a key-derivation slot —
each is a `resource`. Drop runs the destructor. Borrowed handles are
statically scoped to a single call.

**Pros.**
- **Capability-discipline at the type level.** A component cannot manufacture
  a handle; it can only receive handles from imports.
- **Revoke-on-drop is automatic.** Kernel drops the table entry; subsequent
  uses fault.
- **Borrow semantics prevent dangling references** at the call boundary.
- **Production-tested** across all CM adopters (Spin, wasmCloud-v2,
  Fastly Compute, Cosmonic).

**Cons.**
- **Requires (1A).** Extism doesn't have resources.
- **Resource-type ergonomics still have known bugs in non-Rust paths**
  (`jco#1381`, `#1383` 2026-04-14; `wit-bindgen` issues per
  `prior-art/wasm-component-model/critiques.md` §3).
- **Resource handles do not survive process restart.** Anything Myrhiza wants
  to persist (peer ID, content hash) must be a value type, not a handle —
  this is fine but worth being explicit about
  (cf. `prior-art/wasm-component-model/open-problems.md` §5).

**Sources.** `prior-art/wasm-component-model/abi.md` §Resources and handles;
`prior-art/wasmcloud/lessons.md` Borrow #11 ("provider/component split as a
process boundary that *can move*"); `prior-art/wasm-component-model/lessons.md`
Borrow §"Resource handles via per-component i32 tables".

**Closest precedent.** Wasmtime / Spin / wasmCloud-v2.

### Option 2B — Opaque IDs (Extism / kernel-broker)

**Mechanism.** Components pass `string` or `u64` IDs to the kernel; the kernel
resolves the ID to a target component via lookup tables. No type-level
distinction between owned and borrowed; lifetime is "valid until the kernel
revokes it, which happens whenever the kernel decides."

**Pros.**
- **Compatible with Extism** (so compatible with (1B) / (1C)).
- **Conceptually simple.** A handle is a string; a component can stash
  it, log it, send it.

**Cons.**
- **No type-level capability-discipline.** A component holding a string can
  pass it to another component, even if it shouldn't. The kernel must
  authorize per-call, not per-binding.
- **Revocation is kernel-policy, not type-system-enforced.** Bugs in
  authorization logic become security bugs.
- **Cannot express borrow.** Every cross-component handoff is by-value
  (clone). For large payloads this is real overhead.
- **Migration cost to (2A) is the (1B)→(1A) refactor cost** — it is not a
  binding regeneration.

**Sources.** Willow PR #636 §"Constraints we accept" line 530-532 ("Opaque
IDs, not typed resource handles, between components. Until wit-bindgen unifies
imported and exported resource types, components pass string/u64 IDs and the
kernel resolves them.").

**Closest precedent.** Willow PR #636's tentative v1 stance.

### Option 2C — Shared memory (rejected per PR #636)

**Mechanism.** Two components map a shared memory region; cross-component
data is read/written directly.

**Pros.** Fastest possible inter-component handoff.

**Cons.**
- **Annihilates the capability boundary.** Two components sharing memory can
  observe each other's state mutations.
- **Annihilates determinism across peers.** Memory-layout assumptions leak
  across the boundary.
- **Forbidden by the Component Model substrate** itself: shared-nothing
  linkage is the substrate's signature decision (cf.
  `prior-art/wasm-component-model/abi.md` §Shared-nothing linkage).

**Sources.** `prior-art/wasm-component-model/abi.md` §"Components vs core
modules" + §"Shared-nothing linkage"; PR #636 §"Inter-component composition"
("There is no direct memory-shared linkage between components").

**Closest precedent.** Multiple core modules *inside* a single component
(allowed). The boundary is at the component edge.

**Status.** Rejected by both Willow PR #636 and the substrate. Listed for
completeness only.

### Option 2D — Message-passing only (Spritely / Erlang-shape)

**Mechanism.** No direct calls between components. All cross-component
communication is via typed messages routed through the kernel — the same
shape as Erlang processes or Spritely Goblins actors. No resources at all;
all references are sealed-mailbox-shaped IDs.

**Pros.**
- **Capability-discipline by message authentication, not type system.**
  Aligns with OCapN / CapTP traditions.
- **Map cleanly onto async/futures** — every call is naturally async,
  no submit-and-poll bolting.
- **Erlang-style fault isolation:** one component crashing does not block
  another mid-call.

**Cons.**
- **Hostile to Component Model semantics.** WIT functions look synchronous
  by default (preview2 sync, preview3 async-but-typed-as-functions); there's
  no native message-passing primitive.
- **Performance overhead** — every call is a queue + dispatch, even when
  components are colocated. The 30,000 RPS in-process number from
  wasmCloud-v2 (`prior-art/wasmcloud/architecture.md`) is achievable only
  for direct linker calls.
- **Adds an entire orthogonal abstraction** to learn, on top of WIT.

**Sources.** Cross-reference `prior-art/spritely-ocapn/` (folder exists in
the corpus per `prior-art/wasmcloud/lessons.md`'s neighbour links); not
directly read for this report. Erlang/OTP listed in PR #636 §"Lineage and
influences" line 567.

**Closest precedent.** Spritely Goblins, Erlang/OTP. The Component Model
ecosystem has not converged on this shape.

### Willow's current position

PR #636 explicitly rules out (2C) and tentatively commits to (2B) for v1
(opaque IDs), with (2A) as the migration target. (2D) is not considered.

### Myrhiza re-evaluation question

**Does (2B) buy enough simplicity to justify the migration tax?** The
counter-argument: if the kernel-broker for (2B) is doing all the
authorization work that (2A) would do via type-system, the kernel code is
the same complexity either way. The simplification is *only* in the
runtime backend (Extism vs Wasmtime), which is Domain 1, not Domain 2.

**Does (2D) deserve a serious look?** If the kernel is brokering every
call regardless (CLAUDE.md: "Capabilities are the only host surface" + PR
#636's "all cross-component calls go through the kernel"), the difference
between "WIT typed call brokered through kernel" and "typed message
brokered through kernel" is small. (2D) might absorb the async+streams
question (Domain 7) more cleanly.

### Open questions

- If (2A): how does the kernel reissue handles after restart? PR #636's
  shape: "no handle survives a process restart on either end" — does this
  cause issues for long-lived peers?
- If (2A): can a `borrow<R>` cross peer boundaries (i.e. does Myrhiza
  support a remote borrow), or is borrow strictly intra-peer?
- If (2B): what is the canonical ID format? UUID? content hash? bech32m
  with HRPs (Willow precedent — `prior-art/willow/lessons.md` §Validates
  "bech32m-with-HRP identifiers")?
- Cross-app composition: does a component in app A get to call into app B?
  PR #636 says "components compose by importing each other's exposed
  interfaces, mediated by the kernel" — but the manifest authority story
  for cross-app calls is unspecified.

---

## Domain 3 — WIT contract design

Given a WIT contract, the design choices: interface granularity, versioning,
resource conventions, world composition.

### Option 3A — Coarse-grained, view-model-returning interfaces

**Mechanism.** Interaction components export interfaces that return *view
models* in per-surface units: one channel timeline, one member list, one
composer state. Returns are **version-tagged** so the host can skip
recomposition on no-op state changes; large lists are paged.

PR #636 §"Constraints we accept" (lines 482-490): "No tight inner-loop
callbacks across component boundaries. Interaction components return view
models in per-surface units (e.g. one channel timeline, one member list,
one composer state) — not per-element callbacks, but also not 'the whole
app's view.'"

**Pros.**
- **Submit-and-poll friendly.** Each call is one round-trip; results are
  cacheable.
- **Aligned with the no-reentrance constraint** (CM open-problem #11).
- **Diffable at the host level** — the UI can compute element-level diffs
  *without* the component participating.

**Cons.**
- **Less responsive UI.** Coarse units may force re-render of more than
  needed.
- **Pagination is mandatory for large surfaces** (timelines, member rosters)
  — adds protocol complexity.
- **Custom diffing strategy** must be specified (PR #636 defers this to the
  WIT-interfaces child spec).

**Closest precedent.** Holochain zomes (return materialized views, not
streamed updates). Spin's request-handler model is the same shape (HTTP
responds with a complete view, not a stream of patches).

### Option 3B — Fine-grained capability-discipline (Spritely / OCapN-style)

**Mechanism.** Many small interfaces, each granting a narrow privilege.
A component imports specifically the narrowest interface it needs:
`ui:list-append` rather than `ui:panel`; `chat:read-message` rather than
`chat:full-state`.

**Pros.**
- **Tight least-privilege.** A component can only do what its imports list
  permits, at fine granularity.
- **Composable across apps.** A "translation utility" component imports
  `chat:message` (read) without needing `chat:write`.

**Cons.**
- **Many-interfaces explosion.** A typical app might import 30+ interfaces.
  Manifest review becomes harder, not easier.
- **Performance penalty.** Many small calls instead of one coarse call.
- **Conflicts with CM open-problem #7 (`wac` is alpha).** Composing many
  small interfaces is exactly the case where wac performance matters.

**Closest precedent.** Spritely Goblins; OCapN.

### Option 3C — Hybrid: coarse view-model interfaces + fine capability-checked
proxies for privileged operations

**Mechanism.** The default UI app exports coarse `ui:*` interfaces (panel,
list, message). But **`ui:*` calls that proxy privileged platform surfaces
are capability-checked per call, not just per import-binding** (PR #636
lines 326-335). Clipboard writes, file pickers, top-level navigation, push
registration, and similar — each call is gated by the calling component's
manifest, not the UI app's broad surface.

**Pros.**
- **Privileged surfaces get fine-grained discipline; non-privileged
  surfaces get coarse ergonomics.**
- **Aligns with the "default UI app is in the TCB for its own chrome but
  not for arbitrary callers' intents" stance** in PR #636 §"UI is an app".

**Cons.**
- **Two-tier interface design.** Authors must understand both shapes.
- **Per-call capability check has runtime cost.** Each privileged call is a
  kernel re-entry.

**Closest precedent.** PR #636's commitment shape; not seen elsewhere in
the corpus.

### WIT versioning sub-domain

Three options:

- **(3-V-A) Strict-pin (kernel rejects loads requiring downgrade).** PR
  #636 implicit; see also `prior-art/wasm-component-model/open-problems.md`
  §8 ("Reject loads that would require a downgrade. The kernel does not
  implement WIT-style downgrade. Myrhiza's WIT package versions are
  semver-strict at the kernel boundary.").
- **(3-V-B) Silent downgrade (CM default).** Per CM #609, "linked
  interfaces may be downgraded to match what is in the host." Adopted by
  default if Myrhiza doesn't specify otherwise. Footgun.
- **(3-V-C) Adapter components for ABI migration.** When a kernel WIT
  package bumps major version, ship an adapter component that lifts
  old-version guests onto the new ABI. Bounded migration cost. Cf.
  `prior-art/wasm-component-model/lessons.md` Borrow §"Adapter components
  for ABI migration".

PR #636 silently assumes (3-V-A) by inheriting the CM ecosystem's "be
explicit" tendency. The brainstorm should make this explicit.

### Resource and handle conventions sub-domain

If (1A)+(2A): handles are WIT `resource`s with `own<R>` / `borrow<R>`
flavours. The conventions to settle:

- Per-app handle namespace: do two apps installing keys under the same
  opaque handle on one peer collide, namespace per-app, or kernel-arbitrate?
  (PR #636 open question, line 656-658.)
- Handle persistence: do *any* handles outlive a kernel restart? Defaults
  to no per CM open-problem #5; should there be exceptions (e.g. peer-id)?
- Handle introspection: should the kernel expose a `myrhiza:debug/inventory`
  interface for "what handles does this component currently hold"? cf.
  `prior-art/wasmcloud/lessons.md` Borrow #10 (host inventory).

### World composition sub-domain

Each Myrhiza profile is a WIT `world`:
`myrhiza:profiles/state-apply@1.0.0`,
`myrhiza:profiles/state-propose@1.0.0`,
`myrhiza:profiles/interaction@1.0.0`,
`myrhiza:profiles/behavior@1.0.0`. Each world's import list is the
**closed** set of permitted host imports (CM open-problem #3).

App-defined world composition: an app's state component is a component
implementing `myrhiza:profiles/state-apply` (kernel-imposed) plus the app's
*own* domain interfaces (e.g. `chat:state`).

### Sources

`prior-art/willow/runtime-vision.md` §"Submit-and-poll for async" + §"UI is
an app"; `prior-art/wasm-component-model/spec.md` §"Imports vs exports vs
worlds"; `prior-art/wasm-component-model/lessons.md` Borrow §"The world /
interface split"; `prior-art/wasmcloud/lessons.md` Borrow #11.

### Closest precedent

(3A) is implicit in the Spin / wasmCloud / Holochain world-shape (return
view models per-handler). (3B) is unique to Spritely Goblins / OCapN. (3C)
is novel to PR #636.

### Willow's current position

(3A) coarse + (3C) fine-checks-on-privileged-surfaces. Diff/paging strategy
deferred to child spec.

### Myrhiza re-evaluation question

**Is the per-call capability check on `ui:*` privileged proxies a kernel
primitive or an app-side discipline?** PR #636 phrases it as load-bearing
architecture — which means the kernel needs a primitive for "the calling
component's manifest, not the calling component's host", which means
capability tokens carry caller identity. This is a **non-trivial spec**.

### Open questions

- WIT package naming convention for Myrhiza profiles vs apps. PR #636 hints
  `myrhiza:profiles/state-apply`; settle.
- Should `myrhiza:tracing` be defined now (per CM open-problem #12) or
  defer to wasi-otel stabilization?
- Does Myrhiza enforce strict-pin (3-V-A) at install or at first-call?
  Install-time is deterministic; first-call has weaker guarantees but
  catches dynamic-load scenarios.

---

## Domain 4 — Bundle format / app distribution

How do app bundles look on the wire? How are they signed? How are they
hosted?

### Option 4A — `manifest.toml` + components + WIT files (PR #636 shape)

**Mechanism.** An app bundle is a directory:

```
chat-server/
├── manifest.toml              version, hashes, capabilities, interfaces
├── state.wasm                 deterministic component
├── interaction.wasm           UI-coupled component
├── behavior-discord-bridge.wasm
└── schema.wit                 interface contract
```

Hash-pinned, signed by author, fetched by hash via iroh-blobs.

**Pros.**
- **Inspector-friendly.** A user can untar, read the manifest, look at the
  WIT contracts.
- **Tool-neutral.** No special CLI to package.
- **Aligns with the locked-app pattern** Spin uses (`prior-art/spin/
  triggers-and-components.md` §"Distribution model"): manifest + N
  content-addressed component blobs.

**Cons.**
- **Signature of multiple files** is more complex than one artifact.
- **Less aligned with OCI tooling** that expects single artifacts.

### Option 4B — OCI artifacts (Spin / wasmCloud / `wkg` shape)

**Mechanism.** Bundle is a single OCI artifact: `manifest.json` references
content-addressed layers (one per component, one per asset). Pushed to any
OCI registry (GHCR, Docker Hub, ECR). `wkg` is the BA-stewarded tool. Per
the CNCF TAG Runtime WASM OCI artifact layout.

**Pros.**
- **Existing tooling.** `wkg`, cosign signing, OCI mirroring, vulnerability
  scanning, SBOM tooling all apply for free.
- **Production-validated** by Spin and wasmCloud-v1.
- **Multi-language language registry conventions** (`spin:redis@3.0.0`)
  exist.

**Cons.**
- **Not P2P-native.** OCI assumes a centralized registry-of-registries.
  iroh-blobs is the Myrhiza distribution channel.
- **OCI-on-GHCR is fragile** (cf. `prior-art/wasm-component-model/
  critiques.md` §4 — Wasmtime's own security release CI was blocked by
  GHCR-as-WIT-registry instability).
- **No canonical Bytecode-Alliance-blessed registry** exists; "use OCI,
  host of your choice" is the answer.

### Option 4C — iroh-blobs hash-pinned (PR #636 commits to this)

**Mechanism.** App bundle is a content-addressed blob set on iroh-blobs.
Hash is the identity. Peers fetch from any peer holding the blobs. Manifest
is one of the blobs.

**Pros.**
- **P2P-native.** No central registry. Fits Myrhiza's peer-symmetric
  posture.
- **Same content = same hash.** Cross-distribution interop is by hash
  equality (cf. `prior-art/wasm-component-model/lessons.md` Borrow §"The
  OCI content-addressed registry convention" — the Myrhiza bundle on
  iroh-blobs is bit-identical to a hypothetical OCI bundle).
- **iroh is already a load-bearing dependency** for transport (see
  Willow PR #636 §"What stays the same").

**Cons.**
- **No `wkg` analogue.** Myrhiza must build its own dependency-resolution
  tooling.
- **Discovery problem.** "Where do I find an app I want to install?" has
  no answer beyond "ask a peer who has it."
- **Single-vendor dependency.** Iroh is itself pre-1.0, single-steward
  (n0).

### Hybrid 4-A+C — Bundle is a directory, distributed via iroh-blobs, OCI-shape-aligned

**Mechanism.** Bundles are *bit-identical* to what an OCI artifact would be,
but distributed canonically via iroh-blobs. A peer can also push the same
bytes to an OCI registry, and other peers can pull from there. The wire
schema aligns with the [CNCF TAG Runtime WASM OCI artifact layout].

**Pros.**
- **Best of both.** P2P canonical; OCI mirror possible.
- **Existing OCI tooling for signing/SBOM/scan applies** (cosign, etc.).

**Cons.**
- **Two distribution mechanisms to specify.** More surface area.

**Sources.** `prior-art/spin/triggers-and-components.md` §"Distribution
model"; `prior-art/wasm-component-model/lessons.md` Borrow §"OCI
content-addressed registry"; `prior-art/wasmcloud/lessons.md` Validates
§"OCI-as-component-registry"; PR #636 §"Apps as bundles of components".

### Signing model sub-domain

PR #636: signed by app author. Three sub-options:

- **(4-S-A) JWS over manifest.** wasmCloud-v1's "claims" pattern
  (`prior-art/wasmcloud/lessons.md` Validates §"Signed metadata travels
  with the bundle"). Issuer, capabilities granted, version. Embedded in
  the OCI artifact; verified before instantiation.
- **(4-S-B) Sigstore / cosign over the OCI artifact.** Standard OCI
  tooling; ties to Sigstore's keyless / Fulcio identity model.
- **(4-S-C) Ed25519 detached signature in the manifest.** Author's pubkey
  in the manifest; signature over the manifest hash. Aligns with Willow's
  Ed25519-everywhere stance (`prior-art/willow/lessons.md` Validates
  §"Ed25519 identity").

### Closest precedent

PR #636's stack is **(4A directory shape) + (4C iroh-blobs canonical
distribution) + (4-S-A or 4-S-C signing)**. The
sources name OCI as a wire-format alignment target without committing to
OCI-as-distribution.

### Willow's current position

Bundles are content-addressed iroh-blobs sets, hash-pinned, signed by
author. Manifest is `manifest.toml` (TOML, not JSON). OCI alignment is
implicit but not committed.

### Myrhiza re-evaluation question

**Should Myrhiza commit to OCI wire-format alignment as a hard
constraint?** Aligning means we can interop with the WASM-component-OCI
ecosystem (Spin's `spin registry pull`, wasmCloud's deployment pipelines,
existing cosign signing chains, vulnerability scanners). Not aligning gives
us freedom to optimize the wire format for iroh-blobs's chunking
characteristics.

### Open questions

- Does the Myrhiza CLI ship with OCI push/pull, or is iroh-only the v1
  story?
- What's the dependency-resolution tool? `wkg`-equivalent on iroh-blobs?
- How does an app *find* its dependencies (e.g. a shared library
  component) — manifest references by hash, by name+version, or both?
- Discovery: how does a user find an app to install? PR #636 doesn't
  solve this; out of scope for this cluster.

---

## Domain 5 — Browser viability

Myrhiza's commitment: browser peers are first-class. The substrate's
browser story is jco-transpile only.

### Option 5A — jco transpile + sync submit-and-poll, accept the constraints

**Mechanism.** Every component is `jco transpile`d. Each transpile produces
core wasm + JS shim + TS declarations. Browser peer loads the JS shim,
which lifts/lowers via `TextEncoder` / `DataView` / `realloc` calls into
the component's allocator. Async surfaces (gossip, blob fetch, HTTP) are
exposed as **submit-and-poll** WIT host imports: component calls
`broadcast(payload) -> request-token`; kernel later re-enters via
`on-completion(token, result)`.

**Pros.**
- **Works today.** jco 1.19.0 (2026-04-22) is stable; preview2-shim 0.17.9
  (2026-04-17) is actively maintained.
- **Same WIT contract as native.** Apps target one ABI; the kernel ships
  two implementations (Wasmtime + jco-transpiled-with-Myrhiza-host-shim).
- **Determinism preserved** for state-apply (jco is a faithful CABI
  implementation; canonical-ABI behavior is fully specified).

**Cons.**
- **~350 KB JS shim floor per app per browser peer.** Real download cost,
  especially with five apps active.
- **No async on the browser side until preview3 + jco preview3 transpile
  ship.** As of 2026-05-08, jco preview3 transpile mappings are *being
  added this week* (jco PR #1455). Adoption is months out, optimistically.
- **No `wasi:sockets`, no `wasi:filesystem` real FS.** The kernel must
  implement `myrhiza:peer` and `myrhiza:storage` via WebSockets / WebRTC /
  IndexedDB / SubtleCrypto in JS. This **shim is the browser kernel** and
  must remain in lockstep with the wasm-side kernel.
- **Bundle size for JS-authored apps is dire** — `componentize-js` produces
  ~5 MB minimum per JS app.

### Option 5B — Browser uses Extism instead, native uses CM

**Mechanism.** Two ABIs: native peers run Wasmtime + CM; browser peers run
Extism + a Myrhiza-custom subset. Apps ship two builds.

**Pros.**
- **Smaller browser shim** (Extism's runtime is much smaller than jco's).
- **Sidestep preview3 timing.** Extism doesn't need streams/futures.

**Cons.**
- **Two ABIs to maintain.** App authors test against both. Tooling
  doubles.
- **Defeats the "one ABI for app authors" PR #636 goal.**
- **Browser peers can't load the same component bytes as native peers.**
  Hash-equality between bundles breaks at the browser/native boundary.

### Option 5C — Browser is read-only / interaction-only; state-apply only on native

**Mechanism.** Browser peers run *only* interaction profile components.
State-apply runs on native peers. Browser peers fetch materialized state
from a native worker peer over the network.

**Pros.**
- **Sidesteps determinism in the browser.** No need for state-apply in
  jco.
- **Smaller browser bundle.** Only interaction components ship.

**Cons.**
- **Inverts CLAUDE.md's stance.** Browser peers become second-class,
  protocol-asymmetric.
- **Tight coupling between browser peer and a "trusted" worker peer.**
  Single point of failure.
- **Doesn't actually shrink the browser shim much.** The interaction
  profile still pulls in the host import surface for `ui:*` /
  `host.broadcast` / `host.subscribe` etc.

### Option 5D — Native-only browser peer (WebView wrapping Wasmtime)

**Mechanism.** Browser peers ship as a Tauri/Electron-style native app
wrapping Wasmtime. No real browser peers; the "browser" is just a
WebView UI layer on a native runtime.

**Pros.**
- **One runtime, full CM.** No jco constraints.

**Cons.**
- **No actual browser deployment.** Users install a desktop/mobile binary.
  iOS App Store + Android Play approval required for distribution.
- **Defeats the "browser as peer" CLAUDE.md commitment.**

### Nested-WASM-in-browser-WASM concern

PR #636 doesn't directly address this, but it surfaces if the Leptos UI app
itself is a WASM component, *and* it loads other WASM components (chat
state-apply, behavior bridges, etc.) into its WASM context.

The current Willow web client is `wasm-pack`-built Leptos — a single
core-wasm module loaded by the browser. Under PR #636, the UI app becomes
a CM component too (or at minimum imports `ui:*` from other apps' WASM
components). Nested-CM-in-browser-WASM is an open question:

- **Can a jco-transpiled component instantiate another jco-transpiled
  component inside its own JS shim?** Probably yes (the shim uses
  `WebAssembly.instantiate`), but the kernel-mediation pattern Myrhiza
  needs (every cross-component call goes through the kernel) requires the
  kernel to *be* in JS, not in another WASM component.
- **The browser kernel is JS** (per Domain 5A). Components are jco
  shims. The kernel sees JS proxy objects; cross-component calls go
  through the JS kernel. This works but means the browser-side kernel
  is **not** the same code as the native kernel — it's a JS
  reimplementation against the same WIT contract.

### Sources

`prior-art/wasm-component-model/browser.md`; `prior-art/wasm-component-model/
preview-status.md`; `prior-art/wasm-component-model/open-problems.md` §10;
`prior-art/wasm-component-model/critiques.md` §7; PR #636 §"Constraints we
accept" line 491-504 ("Sync ABI at v1, with kernel-side async bridged via
tokens"); PR #636 §"Constraints we accept" line 533-535 ("Two runtime
backends in the kernel. wasmtime native, jco-transpiled web. Same host
interface so app authors target one ABI.").

### Closest precedent

(5A) is the **only** browser path the corpus knows. Spin and wasmCloud are
explicitly server-only (`prior-art/wasmcloud/lessons.md` Avoid §"Server-only
deployment"). Fastly Compute uses jco internally for edge but not for
browser delivery.

### Willow's current position

(5A) firm. "Two runtime backends in the kernel. wasmtime native,
jco-transpiled web. Same host interface so app authors target one ABI."

### Myrhiza re-evaluation question

**Is the ~350 KB JS shim floor per app acceptable?** A user running five
apps (a chat, a wiki, a kanban, a poll, a code editor) downloads ~1.75 MB
of jco shim *plus* the actual app component bytes *plus* the
preview2-shim bundle. Total first-load is multiple MB. Compare this to
what Slack (Electron-bundled, ~200 MB, but cached locally as an installed
app) does — Myrhiza's first-load over slow WiFi is comparable.

The question is: does the design accept this cost as the price of
browser deployment, or does (5D) become less unattractive when sized?

### Open questions

- Does the kernel's JS-side implementation track wasm-side semantics
  rigorously, or is wasm-side authoritative and JS-side a "best effort"
  shim?
- jco's preview3 timeline — block on it for native async, or sustainably
  ship preview2-only on browser even after native moves to preview3?
- Service worker integration: can the browser kernel run in a service
  worker for offline operation?

---

## Domain 6 — Component instantiation, lazy-loading, hash-cache

When does a component instantiate? How is it cached? How is it evicted?

### Option 6A — Instantiate on subscribe (state-apply) + first-use (others)

**Mechanism.** PR #636: "State components materialize as soon as the peer
subscribes to a topic (so it can apply incoming events); other components
instantiate on demand. Worker-computed snapshots can carry peers through
the warm-up so the UI stays responsive even before all interaction
components have downloaded."

**Pros.**
- **State-apply is hot when needed** (events arriving = component must
  apply).
- **Interaction / behavior load lazily**, so a peer in five apps doesn't
  pay startup cost for all of them.
- **Worker snapshots carry warm-up.**

**Cons.**
- **First-use latency for interaction components.** UI shows loading state
  until the component is instantiated.
- **Cache invalidation when component is updated** — see the snapshot
  portability open question.

### Option 6B — Instantiate eagerly per-app on install

**Mechanism.** Installing an app instantiates all its components at once.

**Pros.**
- **No first-use latency.**

**Cons.**
- **High startup cost.** A peer in five apps pays five-app instantiation
  cost on every kernel start.
- **RAM cost.** Every component instance holds linear memory and resource
  table.

### Hash-cache eviction policy sub-domain

Three sub-options:

- **(6-C-A) LRU on instance count.** Bound the number of *instantiated*
  components; evict least-recently-used. PR #636 implicit.
- **(6-C-B) RAM-budgeted.** Bound total wasm linear-memory usage; evict
  largest-and-oldest first.
- **(6-C-C) Per-app guarantees.** Each app gets a guaranteed instance
  slot; eviction is across-app only when global pressure forces it.

### Snapshot custody and warm-up sub-domain

PR #636: "Worker-computed snapshots can carry peers through the warm-up so
the UI stays responsive."

- Worker peers compute deterministic snapshots and gossip them.
- Joining peers can install the snapshot and skip event-by-event replay.
- Snapshot portability across **component-version upgrades** is an open
  question (PR #636 line 660-661).

### Sources

PR #636 §"Apps as bundles of components" (lines 96-122) on lazy-load +
hash-cache; §"Open questions deferred to child specs" line 660-661 on
snapshot portability; `prior-art/wasm-component-model/wasmtime.md` (not
read in this report directly) on `Module::serialize` /
`Engine::precompile_component` for AOT caching.

### Closest precedent

Spin's `Component` is loaded once (AOT-precompiled where possible) and
shared across `Store` instances; per-event `Store` is created and dropped
(`prior-art/spin/architecture.md` §"Component lifecycle"). Spin's "instance
reuse" via `InstanceReuseConfig` is one warm-pool model.

wasmCloud-v2's pooling allocator (`prior-art/wasmcloud/architecture.md` —
"Builds a Wasmtime `Engine` (with the pooling allocator and configurable
WASIp3 support)") is another data point.

### Willow's current position

(6A) instantiation pattern + worker-snapshot warm-up. Eviction policy and
RAM budget unspecified.

### Open questions

- Default RAM budget per instance?
- Default fuel budget per state-apply call (open question per PR #636 line
  647)?
- Pre-check fuel budget — same as apply, or separate (PR #636 line
  653-655)?
- Snapshot migration when component-version changes (PR #636 line 660-661):
  invalidate, schema-migrate, or app-defined?
- Multi-peer behavior coordination (PR #636 line 663-668): leader election
  in the kernel or in the app?

---

## Domain 7 — Async + concurrency model (cross-cutting)

Cross-cuts ABI choice (preview2 vs preview3 affects the WIT surface) and
browser viability (jco doesn't support async). Pulled out for clarity.

### Option 7A — Sync ABI v1, kernel-side async bridged via submit-and-poll
tokens (PR #636 commitment)

**Mechanism.** All WIT host imports are sync. Inherently-async surfaces
return a `request-token`; the kernel re-enters the component via an
exported `on-completion(token, result)` handler.

**Pros.**
- **Browser-compatible.** No async on the JS side needed.
- **State-apply is sync by definition** — no async overhead even on
  native.
- **Preview3 not a blocker.** Myrhiza apps don't depend on `stream<T>` /
  `future<T>` for v1.

**Cons.**
- **Ergonomic cost.** Apps cannot use `async`/`await` flow control; SDK
  macros must hide token-juggling.
- **No native back-pressure semantics** — the kernel manages the token
  lifetime; apps can't `select!` on multiple in-flight tokens cleanly.

### Option 7B — Preview3 native async (when it ships)

**Mechanism.** Use `stream<T>` / `future<T>` / `error-context` directly.
Apps `await` future returns; streams are cancellable.

**Pros.**
- **Better ergonomics** for apps.
- **Standard.** Aligned with the substrate's destination.

**Cons.**
- **Browser support not yet shipped.** jco preview3 transpile mappings are
  being added 2026-05-08 (`jco#1455`).
- **WASI 0.3.0 final is not landed.** Three RCs cut; `Concurrency.md`
  still being edited.
- **Adopters paying dual-stack tax** (rules_wasm_component#257).

### Option 7C — Preview2 + custom Myrhiza-shaped streams

**Mechanism.** Use `wasi:io/streams` and `pollable` for byte streams.
Wrap higher-level concepts (event streams, peer subscriptions) in
Myrhiza-typed streams that are *defined* against poll semantics today
but explicitly marked for migration to preview3 streams when they ship.

**Pros.**
- **Use what works today.** `wasi:io/streams` is stable.
- **Migration path is clear** when preview3 lands.

**Cons.**
- **Boilerplate-heavy.** Apps explicitly compose `pollable`s and call
  `poll`.
- **Submit-and-poll is still needed** for non-stream async surfaces
  (HTTP, blob fetch).

### Sources

PR #636 §"Constraints we accept" (lines 491-504); `prior-art/wasm-component-
model/preview-status.md`; `prior-art/wasm-component-model/critiques.md` §1
+ §8; `prior-art/wasm-component-model/open-problems.md` §1 + §6.

### Willow's current position

(7A) firm. PR #636: "Sync ABI at v1, with kernel-side async bridged via
tokens. Browser jco does not support async. State `apply` is sync by
definition."

### Open questions

- Is submit-and-poll the *only* model, or do we permit native preview3
  async on the native backend with browser using submit-and-poll? (Two
  ABIs again — see Domain 5B.)
- What does cancellation look like under submit-and-poll? Token-cancel
  RPC? Drop-the-handler-to-cancel?

---

## Domain 8 — Determinism + WASM features (cross-cutting)

Determinism for state-apply requires choosing which core-wasm features are
permitted. Cross-cuts ABI choice (full CM gives access to features but
also exposes them).

### Permitted core-wasm features

**Permit for v1:** `multi-value`, `bulk-memory`, `reference-types`,
`tail-calls`. SIMD permitted only with NaN canonicalization (see
WasmEdge#4819 on f64x2 NaN payload divergence).

**Forbid for v1:** `gc` (per CM #525, pre-proposal), `threads` (per CM
open-problem #9), `memory64` (per CM #22, open since 2022), `exception-
handling`. Floats permitted but with strong recommendation to ban in v1
state-apply per PR #636 §"Determinism, in detail".

### Fuel + epoch policy

PR #636: state-apply uses **fuel** (deterministic instruction-count
budget). Other profiles use **epoch interruption** (cooperative
time-slicing).

`prior-art/wasm-component-model/lessons.md` Borrow §"Fuel + epoch metering
combined" supports this split:

- Fuel for state-apply: deterministic budget that's part of the consensus
  invariant — every peer applies the event with the same fuel limit,
  terminates at the same instruction count.
- Epoch for interaction/behavior: wall-clock time-slicing for fairness,
  no consensus implication.

Cost: fuel has ~10–20% perf hit
(`bytecodealliance/wasmtime#4109`).

### Determinism validator

Per `prior-art/wasm-component-model/open-problems.md` §4: determinism
comes from a kernel-side validator, not from the substrate. Kernel
restricts state-apply components to:

- **Forbidden-imports list:** no `wasi:clocks/wall-clock`, no
  `wasi:random`, no `wasi:sockets`, no SIMD floats unless canonicalized,
  no threads.
- **Wasm-validator pass:** rejects any component using non-deterministic
  instructions.

The validator is a separate offline-runnable function over the component
binary (CM lessons.md Borrow §"Validator as a separate pass").

### Open questions

- Default fuel budget per state-apply call?
- NaN canonicalization on/off — `wasmtime`'s `wasm_nan_canonicalization` +
  `cranelift_nan_canonicalization` flags. Both required for SIMD to be
  deterministic.
- Cross-engine determinism: wasmtime vs jco. The `prior-art/wasm-component-
  model/lessons.md` "skepticism check" on this is sharp: "in practice,
  only Wasmtime is fully spec-conforming; jco's preview2-shim and Wasmer's
  wai are partial implementations. Cross-engine bit-identical execution is
  closer to 'one engine + a verifier' than to 'many engines.'"

---

## Cross-domain interactions

Where choices in one domain force choices in another:

| Combo | Interaction |
|---|---|
| **(1A) full CM + (5A) jco transpile** | Forces ~350 KB shim floor per app per browser peer; forces preview2 baseline until jco preview3 transpile stabilizes. |
| **(1B) Extism + (2A) typed resources** | **Impossible.** Extism doesn't support resources. Choosing (1B) forces (2B) opaque IDs. |
| **(1A) full CM + (2B) opaque IDs** | Possible (use `string`/`u64` types instead of resources) but throws away the type-level capability discipline (1A) was supposed to enable. |
| **(2A) resources + (3-V-A) strict-pin** | Resource definition changes are a major bump; existing handles in materialized state become invalid on kernel upgrade. Adapter components (3-V-C) become load-bearing. |
| **(1A) + (4A) bundle directory + (4C) iroh distribution** | Wire-format alignment with OCI is possible but optional; OCI tooling (cosign, scanners) does not directly apply to iroh-blob-bundles. |
| **(5A) jco + (7A) submit-and-poll** | Mutually reinforcing: no async on browser, every async surface uses tokens. (7B) preview3 is gated on jco preview3. |
| **(3A) coarse view-models + (7A) submit-and-poll** | Each view-model fetch is one round-trip; pagination shapes the protocol. Compatible. |
| **(8 fuel for state-apply) + (1A) full CM** | wasmtime's fuel mechanism is a CM feature; Extism does not have fuel by default — a determinism-validated Extism would need custom integration. (1B) makes (8) harder. |
| **Pre-check = apply (CLAUDE.md) + (1A)** | Forces state-apply to be a CM export the kernel can call in dry-run mode. Pre-check fuel budget is a separate spec question (PR #636 line 653-655). |
| **(6A) lazy-load + (4C) iroh-blobs** | Component bytes are fetched on first use; iroh-blobs's chunked transfer is the bottleneck. Pre-fetch on subscribe is the warmup. |

---

## Brainstorming question list, sorted by criticality

The brainstorm should close these in order. Higher-criticality questions
gate downstream work.

### CRITICAL — gates everything else

1. **(Domain 1) v1 ABI backend: full CM (1A), Extism (1B), hybrid (1C),
   or native-only (1D)?** Each forces downstream choices in Domain 2,
   tooling burden, and migration cost. *Willow lean was (1B); does
   Myrhiza inherit?*
2. **(Domain 5) Browser viability: accept jco shim costs (5A), maintain
   two ABIs (5B), demote browser to interaction-only (5C), or drop
   real-browser support (5D)?** *PR #636 commits to (5A); this is one
   of the two re-evaluation questions.*

### HIGH — needed before WIT design

3. **(Domain 2) Cross-component composition: typed resources (2A),
   opaque IDs (2B), or message-passing (2D)?** Forced partly by Domain 1.
   But (2D) is novel and might absorb the async question.
4. **(Domain 7) Async: sync+submit-and-poll (7A), preview3 (7B), or
   preview2-with-Myrhiza-streams (7C)?** *PR #636 commits to (7A); aligned
   with (5A).*
5. **(Domain 3) Default WIT package naming and versioning policy.**
   `myrhiza:profiles/state-apply@1.0.0`? Strict-pin (3-V-A) at install?
6. **(Domain 4) Bundle distribution: directory + iroh-blobs (4A+4C),
   OCI alignment, or both?** *PR #636 commits to iroh-blobs canonical;
   OCI alignment is the open question.*

### MEDIUM — orthogonal but needed for v1

7. **(Domain 4) Signing model: JWS/claims (4-S-A), Sigstore (4-S-B), or
   detached Ed25519 (4-S-C)?**
8. **(Domain 8) Determinism validator scope: which core-wasm features
   permitted; NaN canonicalization on/off; floats permitted in
   state-apply or banned?**
9. **(Domain 6) Default fuel and RAM budgets per profile per app.**
10. **(Domain 6) Snapshot portability across component-version upgrades.**
11. **(Domain 3) Resource handle conventions: per-app namespace, kernel
    introspection, persistence semantics.**
12. **(Domain 5) Service worker integration for offline browser peers.**

### LOW — future-work, deferrable

13. Dependency-resolution tool (Myrhiza's `wkg` analogue).
14. App discovery — how does a user find an app to install?
15. Hot reload (PR #636 defers to v2).
16. Cross-app authority composition (PR #636 defers to v2).
17. Multi-peer behavior coordination primitives in kernel vs app.
18. `wac`-style build-time composition vs kernel-time composition for
    multi-component apps.

---

## Specific issues surfaced

### Extism's hard limitations are a real refactor at migration

PR #636 explicitly acknowledges: "expect to update at migration boundary,
but not redesign your state machine." But the changes are substantial —
*every cross-component call site* changes:

- **Resource handles replace ID lookups.** Type-level capability
  discipline replaces kernel-side ID validation.
- **Imported interfaces replace kernel-broker calls.** Compile-time linking
  replaces runtime dispatch.
- **Borrows replace clone-and-pass.** Lifetime annotations on the call
  site.

This is not "regenerate bindings." App authors writing (1B) today are
writing v1 code that v2 will refactor. Whether the v1 ship-date win pays
for the v2 refactor cost is the (1A)-vs-(1B) question.

### Browser CM maturity is the binding constraint

jco 1.19.0 (2026-04-22) ships preview2; preview3 transpile mappings *being
added this week* (jco#1455). preview2-shim 0.17.9 (2026-04-17). No native
browser CM in any engine. If Myrhiza demands browser parity with native,
preview3 is gated on jco preview3 stability — which is at least months
out from the 2026-05-08 PR opening.

For Myrhiza's `interaction` profile, this means the real choice is:
**preview2 + submit-and-poll for v1**, with no native async on browser
side, period. (Native peers could in principle use preview3 today behind
Wasmtime's `unsafe_async` flag, but maintaining two profile semantics is
exactly the dual-stack tax we want to avoid.)

### wasmCloud's v1→v2 reset is a lesson Myrhiza should absorb

The wasmCloud v1→v2 reset (2026-03-22) retired:
- **Lattice** (NATS topic-prefix tenancy)
- **Capability providers** (out-of-process WASM provider model)
- **Link definitions** (runtime-mutable component bindings)
- **wadm** (declarative reconciler)
- **`wasmcloud:secrets`** (envelope-encryption custom interface)

Reasons cited (`prior-art/wasmcloud/architecture.md`): "the transparency was
paying for ~6× throughput overhead on the in-process case and was worth
giving up." The v2 stance: explicit-when-distributed beats transparent-with-
overhead.

Myrhiza inherits two corrected lessons:

1. **Don't make distributed RPC transparent at the component-call level.**
   Components in the same workload should call each other in-process via
   the linker (CM open-problem #11 reentrance forbids host-mediated
   reentrance for `state-apply`, but cross-component-within-an-app is fine).
   Components on different peers should use an *explicit* WIT-typed
   transport.
2. **Don't pivot architecture mid-product.** wasmCloud lost
   `wasmcloud:secrets` to operational simplicity; Myrhiza has no
   orchestrator to delegate to, and the v1 wasmCloud:secrets design
   (envelope encryption, per-component scoping, no key material in
   component memory) is precisely the precedent Myrhiza needs.

### Spin's factor architecture maps to Myrhiza profiles only roughly

Spin's *factor* (SIP-021) is per-host-capability — `factor-key-value`,
`factor-outbound-http`, `factor-llm`. SIP-023 is per-key configuration
inheritance (`inherit_configuration: true | false | [keys]`).

Myrhiza's *profile* is per-component-class — `state-apply`,
`state-propose`, `interaction`, `behavior`. Different abstraction:

- A Spin factor is a host-side capability provider (kernel half).
- A Myrhiza profile is a guest-side determinism/import-set declaration.

The Myrhiza analogue of a Spin factor is the **kernel's host import
implementation** (e.g. `kernel.peer.broadcast` is satisfied by a
network-factor that handles iroh gossip). The factor pattern *inside the
kernel* is borrowable — `init` (linker setup), `configure_app` (manifest
validation), `prepare` (per-instance state) — but it's a kernel-internal
pattern, not part of the app contract.

SIP-023 is more directly relevant: when a Myrhiza app has sub-component
dependencies (per-app composition), per-key capability inheritance is the
right shape. *True/false inherit-all is a footgun*; per-key inherit is
least-privilege.

### OCI vs iroh-blobs: what we lose by skipping OCI

If Myrhiza commits to iroh-blobs canonical distribution (per PR #636) and
*does not* align wire-format with OCI, we lose:

- **`cosign` signing.** Sigstore-compatible signing of OCI artifacts is
  the de-facto WASM-component signing tool.
- **Vulnerability scanners.** Trivy, Grype, others scan OCI artifacts.
- **SBOM tooling.** Standard tools generate SBOMs from OCI artifacts.
- **Mirroring.** GHCR, Docker Hub, Harbor mirror to / from each other.
- **The `wkg` dependency-resolution tool.** Reading
  `~/.config/wasm-pkg/config.toml` and resolving package names to OCI
  references is BA-stewarded.
- **Cross-WASM-runtime sharing.** A Spin component and a Myrhiza app
  could share the same upstream OCI-published shared-WIT-package
  if wire-formats align.

If we *do* align wire-format (without committing to OCI as a distribution
mechanism, just as a wire schema), most of these survive.

### `wkg` analogue for Myrhiza

`wkg` (`bytecodealliance/wasm-pkg-tools` 0.15.0, 2026-02-06) is the BA
package-resolution tool. Reads config; resolves `package = "ns:name@version"`
to an OCI reference; fetches; verifies hash.

A Myrhiza analogue would:
- Read the bundle's `manifest.toml` for declared dependencies.
- Resolve each dependency to an iroh-blobs hash (via a local cache, a
  shared peer's hash table, or — controversially — an OCI mirror).
- Fetch the dependency components.
- Verify hashes match.

This is Myrhiza-novel work. No upstream tool currently resolves
WASM-component dependencies via iroh-blobs.

### `wac` build-time vs runtime composition

`wac` is the BA-stewarded **build-time** WASM composition tool. It wires
component A's exports into component B's imports and produces a single
artifact.

Myrhiza is **runtime-composition by design** — the kernel brokers every
cross-component call (PR #636 §"Inter-component composition"). The kernel
*is* the composer.

Build-time composition still has a use case: **packaging a multi-component
app as a single deliverable**. If an app has state.wasm, interaction.wasm,
and one shared utility component, `wac` could compose them into a
single .wasm at build time. Trade-offs:

- **Pro:** Single artifact distribution; one hash; smaller manifest.
- **Con:** Loses per-component lazy-loading (if the bundle is one .wasm,
  the kernel can't selectively instantiate state-only).
- **Con:** Loses kernel mediation of intra-app cross-component calls
  (compose-time wiring bypasses the kernel).
- **Con:** `wac` is alpha (CM open-problem #7) with O(N²) re-parsing
  performance.

PR #636's implicit answer: **don't compose at build time**. Each component
ships as a separate .wasm; the kernel composes at load time. This loses
single-artifact convenience but gains lazy-load + kernel-mediation
properties Myrhiza needs.

---

## Sources

### Prior-art folders read for this report

- `/mnt/storage/projects/myrhiza/docs/prior-art/wasm-component-model/{README,abi,lessons,browser,preview-status,critiques,open-problems,spec}.md`
- `/mnt/storage/projects/myrhiza/docs/prior-art/wasmcloud/{README,architecture,lessons,wrpc,capability-model}.md`
- `/mnt/storage/projects/myrhiza/docs/prior-art/spin/{README,architecture,lessons,sdks-and-tooling,triggers-and-components}.md`
- `/mnt/storage/projects/myrhiza/docs/prior-art/willow/{runtime-vision,lessons,apps,ui}.md`

### Primary specs

- `/tmp/willow-pr-636.diff` — Willow PR #636 master runtime spec (843
  lines).
- `/mnt/storage/projects/myrhiza/CLAUDE.md` — locked Myrhiza decisions.

### Upstream verified facts (key ones)

- WebAssembly Component Model HEAD `669d494` (2026-05-07).
- WASI 0.2.11 (2026-04-07); 0.3.0 RCs only (latest 2026-03-15).
- Wasmtime 44.0.1 (2026-04-30); LTS 36.0.9 (2026-05-05).
- jco 1.19.0 (2026-04-22); preview2-shim 0.17.9 (2026-04-17); jco#1455
  preview3 transpile mappings opened 2026-05-08.
- componentize-js 0.20.0 (2026-04-14); componentize-py 0.23.0 (2026-04-15).
- wasm-tools 1.248.0 (2026-04-28); wac-cli 0.10.0 (2026-04-17); wkg 0.15.0
  (2026-02-06).
- Spin v4.0.0 (2026-04-20).
- wasmCloud v2.1.0 (2026-05-07); v2.0.0 reset (2026-03-22).
- Preview3 milestone open since 2023-08-22 (still open as of 2026-05-09).

### Key issues, PRs, and verbatim quotes

All cross-referenced via the prior-art critique and open-problem files.
The most load-bearing for this cluster:

- CM #412 (reentrance/callbacks); CM #525 (Wasm GC pre-proposal); CM #609
  (silent downgrade); CM #648 / #638 (resource dtor settling); CM #525
  (Concurrency.md still being edited).
- jco#1383 / #1381 (resource ergonomics in JS).
- wit-bindgen #1604 / #1587 / #1585 / #1582 / #1518 / #1516 (per-language
  ergonomics).
- componentize-py #98 (35 MB hello-world).
- ComponentizeJS #291 (5 MB+ component sizes).
- spinframework/spin#3485 (preview3 RC version-tracking churn).
- pulseengine/rules_wasm_component#257 (preview2/preview3 dual-stack).
- denoland/deno#31314, oven-sh/bun#24867 (browser CM gap).
- WebAssembly/WASI#886 (OCI/GHCR registry instability).
- bytecodealliance/wasmtime#4109 (fuel perf hit).
- wasmCloud#5020 (v2 K8s pivot friction); #4953 (resource string-cmp bug).

---

*End of cluster B mining. The brainstorming session that consumes this
should aim to close at minimum questions 1, 2, and 5 from the criticality
list — those gate every subsequent spec.*
