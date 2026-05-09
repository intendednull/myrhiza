**Date:** 2026-05-09
**Status:** active
**Subject:** WASM Component Model — third-party and insider critiques of the substrate

# Critiques & honest assessments

A consolidation of substantive criticism of the WebAssembly Component Model from GitHub issues, public roadmap posts, and adopters' own venting. The corpus is for Myrhiza spec authors who are betting the runtime on this substrate and need to walk in eyes-open. Where a critique is verbatim from an upstream source, it is preserved verbatim with URL.

## 1. Async story churn — Preview3 has slipped repeatedly

The single biggest open wound in the substrate. The **"Preview 3" milestone on `WebAssembly/component-model` was created 2023-08-22 by `ricochet`** (verified via `gh api repos/WebAssembly/component-model/milestones`) and is still **open** as of 2026-05-09 — a 2 year 9 month slip with no published target date (`due_on: null`). The milestone tracks 2 still-open issues.

The mechanism for landing async cuts across `Concurrency.md`, the canonical ABI for `future<T>` / `stream<T>` / `error-context`, and the entire toolchain. `Concurrency.md` is *still being edited* in 2026: PR `#643` "Add 'Component Instance Lifetime' section to Concurrency.md" merged 2026-04-30, PR `#641` "Emoji-gate synchronous future/stream read/write" merged 2026-04-27. These are not cosmetic — they are spec-level decisions about how guests express concurrency. PR `#641`'s title alone (gating sync read/write of futures *with an emoji feature flag*) is a tell that the design surface is still moving.

**Verbatim, from the WASI P3 template breakage thread on Spin** ([spinframework/spin#3485](https://github.com/spinframework/spin/issues/3485), 2026-04-27):

> *"Cannot get `http-rust-wasip3-unstable` working with Spin 4.0 […] component imports instance `wasi:http/types@0.3.0-rc-2026-01-06`, but a matching implementation was not found in the linker"*

The `0.3.0-rc-2026-01-06` versioning literally encodes the date of the RC. Adopters are tracking moving targets by their commit dates because the spec is not yet pinned to a stable version.

**Status as of 2026-05-09:** still applies. WASI repos are at `v0.3.0-rc-2026-03-15` (latest of three RCs cut 2026-01-06, -02-09, -03-15) for the seven coordinated subsystems; no `v0.3.0` final. Wasmtime ships preview3 behind a flag (`unsafe_async`-gated). jco PR `#1455` "feat: add preview3 transpile mappings" was opened 2026-05-08 — *yesterday* relative to this doc — with the comment "I'm not sure if this is what we want here -- adding p3 mappings unconditionally?" The toolchain is still figuring out whether p3 is on or off by default.

The async-as-WIT-feature debate has effectively been settled (futures and streams are first-class WIT types per `WIT.md` and `Concurrency.md`), but the *callback-based* alternative continues to surface; see [issue #412 "Support to invoke user defined callbacks inside WASM component from wasmtime"](https://github.com/WebAssembly/component-model/issues/412) (open since 2024-11-11, labeled `pre-1.0`):

> *"Programs like eBPF usually hire a callback function to let underlying framework to invoke it when a certain kind of event happened, to support this in WASM component model, it would be very useful to enable developers coding WASM component with callbacks. Current WASM component model doesn't have an appropriate keyword/primitive to support that, and wasmtime doesn't support reentrance to WASM component."*

Reentrance is still a hole in the substrate; psibase's plugin-host architecture review ([gofractally/psibase#1703](https://github.com/gofractally/psibase/issues/1703), 2026-02-12) cites the Bytecode Alliance Zulip directly:

> *"There are unfortunately no good options for callbacks in the component model yet."*

## 2. WIT toolchain immaturity for non-Rust languages

componentize-js and componentize-py bundle whole language engines into the produced component. SpiderMonkey-via-componentize-js produces tens of MB; CPython-via-componentize-py the same. **Verbatim from [bytecodealliance/componentize-py#98](https://github.com/bytecodealliance/componentize-py/issues/98) "Feature request: Smaller wasm modules / binaries" (2024-07-16):**

> *"is it possible to produce a Hello World example that's much smaller than 35MB (and to reduce the size of the wasmtime host side bindings from 30MB)?"*

The maintainer's reply (`dicej`, same thread, 2024-07-16):

> *"Yeah, the size is annoying, I agree. You can reduce it somewhat using e.g. `wasm-tools strip --all` […] Otherwise, I don't know of any great options for significantly reducing the component size and still keep it"*

Same complaint surfaces on the JS side. [bytecodealliance/ComponentizeJS#291 "Component Size & Performance: JCO/SpiderMonkey vs QuickJS"](https://github.com/bytecodealliance/ComponentizeJS/issues/291) (2025-09-08):

> *"I did certainly notice the chunky component size of my JS components whenever wasmtime had to recompile them. I didn't think too much of it, thinking that's just the price to pay for bundling an interpreter. However, eventually I built a rust WASI component bundling QuickJS (via the rquickjs crate), whi[ch]…"*

The maintainer's reply (`tschneidereit`, same thread):

> *"the key reason for choosing SpiderMonkey instead of QuickJS is that in my opinion QuickJS, while an impressive piece of technology for what it is, is not a good basis for a production JS runtime."*

So the size is a deliberate tradeoff for "production JS"; the cost is a baseline ~5 MB+ on every JS component. **Status: still applies.** The componentize-py issue has been open since 2024-07; no fundamental remediation has shipped.

The toolchain is also fragile per-language. Sample of currently-open `wit-bindgen` issues (`gh api 'repos/bytecodealliance/wit-bindgen/issues?state=open'`, 2026-05-09): `#1604` "Optimize the size of generated bindings" (2026-04-24, "The wasip2 crate is 1.3MB and the wasip3 crate is 1.6MB with both crates containing generated bindings that are well over 10k lines of code. There is a lot of redundancy in those bindings"); `#1587` "Bug report: Generated Moonbit binding bug" (2026-04-14); `#1585` "Bug report: Generated C++ glue code bug" (2026-04-13); `#1582` "go - WIT tuple type generated code uses unkeyed fields" (2026-04-08); `#1518` "moonbit bindgen `s8`/`s16` lift corrupts values"; `#1516` "markdown bindgen generates invalid HTML". The Rust path is mature; the C++/Go/Moonbit/C# paths are not.

## 3. Resource type ergonomics — per-language

[bytecodealliance/jco#1383](https://github.com/bytecodealliance/jco/issues/1383) "Bug resport: The encapsulation path from jco's borrow<resource> to Python objects is defective" (2026-04-14) and [#1381](https://github.com/bytecodealliance/jco/issues/1381) "Bug report: An error when deleting from the resource table causes the next access to obtain an incorrect handle" (2026-04-14) are both open. Resource handle bugs are still being filed in 2026 against the most-used non-Rust transpile path.

The spec itself is still settling resource semantics: [#648](https://github.com/WebAssembly/component-model/issues/648) "Why is it `(dtor (func n))` instead of `(dtor (core func n))`?" (2026-05-05), [#638](https://github.com/WebAssembly/component-model/issues/638) "Make resource `dtor` type explicit" (closed 2026-04-16). Open since 2023: [bytecodealliance/wit-bindgen#586](https://github.com/bytecodealliance/wit-bindgen/issues/586) "Prototyping handles and resources using a bindings-based implementation".

**Status: still applies.** Rust path is good; JS/Python/C# resource ergonomics are a known sharp edge in 2026.

## 4. OCI-as-registry friction — no canonical component registry

The `wkg` tool plus the OCI artifact convention (defined in `bytecodealliance/wasm-pkg-tools`) is the de-facto answer, but it is not universally adopted and has operational rough edges.

**Verbatim from [WebAssembly/WASI#886 "Hosting WITs via OCI on GHCR is flaky"](https://github.com/WebAssembly/WASI/issues/886) (2026-02-24, filed by an `alexcrichton`-tier wasmtime maintainer after a security release):**

> *"Today we did a security release of Wasmtime which involves doing a lot of CI all at once. We had lots of flaky failures due to `wkg` being unable to download WITs, such as: `WARN oci_client::token_cache: Invalid bearer token error=Error(InvalidToken)`"*

The Wasmtime team's *own CI for a security release* was blocked by GHCR-as-WIT-registry instability. [WebAssembly/WASI#873](https://github.com/WebAssembly/WASI/issues/873) "`wkg wit fetch` Not authorized" (2026-01-20) reports the same shape. [bytecodealliance/wasm-pkg-tools#198](https://github.com/bytecodealliance/wasm-pkg-tools/pull/198) (2026-03-18), `#149`, `#145` are all about wkg-OCI rough edges.

There is also no governance body running a canonical *Bytecode-Alliance-blessed* component registry analogous to crates.io / npmjs.com. The convention is "use OCI, the host of your choice."

**Status: still applies.** OCI-on-GHCR is fragile; no canonical neutral registry exists.

## 5. Multiple core-wasm features required by adopters

The Component Model itself does not depend on Wasm GC, exception handling, threads, or memory64 — but components written *in* Java/Kotlin/Scala/C# need GC; C++ components want exceptions; multi-threaded components want threads; large-heap components want memory64.

**Wasm GC + Component Model integration is still pre-proposal.** [WebAssembly/component-model#525 "Pre-Proposal: Wasm GC Support in the Canonical ABI"](https://github.com/WebAssembly/component-model/issues/525) (2025-06-03):

> *"This issue proposes extensions to the Component Model's Canonical ABI for Wasm GC support and describes some of the motivation for particular choices. I am in the process of implementing these extensions in `wasm-tools` and `wasmtime`. My goals are to kick off discussion of how best to integrate GC and the canonical ABI, build consensus, and eventually get these extensions merged into the component model spec itself."*

So as of 2026-05-09, GC-integration is still "kick off discussion" — almost a year after the pre-proposal. Memory64 has been an open question on the CM since [#22](https://github.com/WebAssembly/component-model/issues/22) (2022-04-12, still open). Scala-JS reports "Scala.js Wasm backend suitable for standalone Wasm VMs" as enhancement (scala-js/scala-js#4991, open). dart2wasm "Support non-JS wasm runtimes" (dart-lang/sdk#53884, open 2026-02-04).

**Status: still applies.** Practical GC-language support inside CM is not solved in 2026.

## 6. Spec velocity — "almost stable" for years

The CM has been "almost ready for 1.0" since at least 2023. The repo uses no GitHub releases (only proposal-phase tags); spec versioning is by phase number, not semver. Activity is still high — `gh api repos/WebAssembly/component-model/commits` shows 10+ commits in the past 30 days (2026-04-09 → 2026-05-07), several of them substantive (`9fdc3cc` "Use `<core:funcidx>` in resource type definitions", `aec7010` "Rebase CABI onto explicit stack-switching interface"). The spec is still moving in 2026.

The **"Preview 3" milestone created 2023-08-22, still open in 2026-05-09**, is the cleanest single artifact of the velocity problem. **Status: still applies.**

## 7. Browser support — no native CM, jco-transpile is the only path

No browser vendor has shipped native Component Model support. The path to "WASM component runs in a browser" is `jco transpile`, which lifts the component to a JS+WASM-core bundle. This is a hidden cost that doesn't show up in "WASM is everywhere" marketing.

**Verbatim from [denoland/deno#31314 "Support WASM components and WIT files for richer types"](https://github.com/denoland/deno/issues/31314) (2025-11-16):**

> *"I would like to be able to import WASM components directly from Deno and get rich type support (including complex object types). Currently it's not supported by Deno, and I can't add it myself due to Deno's lack of custom loader support."*

Same shape on bun (oven-sh/bun#24867, open). Even the Deno-and-bun runtimes (whose ergonomic affinity to Component Model is high) cannot natively load components in 2026.

**Status: still applies.** jco-transpile is the only browser/JS-runtime path. There is no W3C process actively shipping CM as a browser feature.

## 8. WASI preview2 → preview3 ABI break

Preview2 is poll-driven (`pollable` resources, `wasi:io/poll`, sync `read`/`write`). Preview3 is async-native (`stream<T>`, `future<T>`, `error-context`). This is not an additive change; it's a different ABI shape. Components written against preview2 do not Just Work on preview3 hosts.

**Verbatim from [leptos-rs/leptos_wasi#18 "WASI Preview 3 support"](https://github.com/leptos-rs/leptos_wasi/issues/18) (2026-05-04):**

> *"WASI Preview 3 is out in Wasmtime and in Spin canary. P3 simplifies and optimises a lot of the async bits around HTTP requests. It would be good to make a P3-native version of leptos-wasi, particularly because the async entry point is required if applications are to use async APIs (e.g. async database requests instead of blocking ones). I am not sure of the scope of changes, though - it's been too long since I looked at this code. The streaming model is somewhat different from WASI P2 (goodbye ResponseOutparam, hello request-response), but hopefully not so divergent as to require major rewri[te]"*

That last line — *"hopefully not so divergent as to require major rewrite"* — is the venting. **Status: still applies.** Specific repos that have to write full p2+p3 dual-stack support: `pulseengine/rules_wasm_component#257` "feat: Multi-Bundle Support for WASI Preview2/Preview3 Coexistence" (closed 2026-03-23). The dual-stack tax is real and being paid in production code today.

## 9. Observability primitives missing

There is no standardized tracing / metrics / log WIT yet. [WebAssembly/WASI#646 "Proposal: wasi-otel"](https://github.com/WebAssembly/WASI/issues/646) (2025-03-12) is still open:

> *"WASI Otel exposes an OpenTelemetry interface to Wasm components to allow them to collect trace, metric, and log signals […] This work was originally being pursued under the banner of WASI Observe. Throughout that process the contributors felt that it was best t[o…]"*

So observability is in proposal stage, not in the preview2 set, not in the preview3 set. **Status: still applies.** Adopters who want tracing today either bring their own (component-internal) OpenTelemetry library or build a private host import for it.

## 10. Wasmer's separate trajectory

Wasmer has historically run its own WAI/WIT-adjacent toolchain rather than adopting the Bytecode Alliance Component Model directly. [wasmerio/wai#36 "WebAssembly Component Model + WASI"](https://github.com/wasmerio/wai/issues/36) (2023-03-10, still open):

> *"Is there currently a way of running a WebAssembly component with WASMER that uses WASI? The `wai-component` tool seems to be able to generate a component from a core WebAssembly module and a corresponding `.wai` interface file. What I didn't find is an example demonstrating how WASMER, embedded into Rust, can load a `.wai` file and the WebAssembly component and run functions exposed in the `.wai` definition."*

The issue has been open with no answer for over three years. Wasmer's `wai` is a fork-with-different-spelling of `wit`; the two ecosystems do not interop transparently. In 2026 most CM activity is concentrated in the Bytecode Alliance / Wasmtime sphere; Wasmer is effectively a separate runtime trajectory.

**Status: still applies, but mostly settled into "two ecosystems."** Myrhiza picks Wasmtime; the Wasmer split is a fact about the ecosystem, not a contested debate.

## 11. Performance — component overhead, linker time, cold start

Component overhead vs core-wasm overhead is real. `wit-bindgen`-generated bindings can be 10k+ lines of code per crate ([wit-bindgen#1604](https://github.com/bytecodealliance/wit-bindgen/issues/1604), 2026-04-24): "the wasip2 crate is 1.3MB and the wasip3 crate is 1.6MB with both crates containing generated bindings that are well over 10k lines of code. There is a lot of redundancy in those bindings."

Linker time matters when composing many components. [bytecodealliance/wac#85 "`wac_types::Package` not having its own `wac_types::Types` requires re-parsing packages on every composition"](https://github.com/bytecodealliance/wac/issues/85) (2024-04-18) flags O(N²) re-parsing as a known issue.

Cold-start cost includes engine initialization. Wasmtime's epoch-interruption mechanism ([wasmtime#12990 "MMU-based epoch interruption"](https://github.com/bytecodealliance/wasmtime/issues/12990), 2026-04-08) is being re-engineered for less overhead. Fuel metering has a documented "rather significant performance hit" per [wasmtime#4109 "Slacked fuel metering"](https://github.com/bytecodealliance/wasmtime/issues/4109) (open since 2022-05-07):

> *"wasmtime right now has the fuel mechanism. It allows precise control of how many instructions are executed before suspending execution, at least at a basic block granularity. The price is a rather significant performance hit."*

**Status: still applies.** The substrate has working perf knobs (Pulley, Winch, epoch interruption, fuel) but combining "deterministic + bounded + fast" remains a knob-tuning exercise per use case.

## 12. WIT semver is convention, not enforced

[#609 "Adding a note to WIT.md about WIT interface version interop & host downgrades"](https://github.com/WebAssembly/component-model/issues/609) (2026-02-11):

> *"Linked interfaces may be downgraded to match what is in the host (i.e. `ns:pkg/iface@0.2.1` being downgraded to `ns:pkg/iface@0.2.0`) […] adding functions to an existing interface (even with `@since`) *could* become a breaking change, because guests cannot predict whether hosts will have coverage or not."*

**Status: still applies.** The host can silently downgrade a guest's required version; there is no compile-time enforcement that the version a guest requested is the version it gets. Compatibility is by convention plus link-time fallback, not by typecheck. See also [#540 "Incorrect references to SemVer"](https://github.com/WebAssembly/component-model/issues/540), [#534 "Interface version / compatibilty changes"](https://github.com/WebAssembly/component-model/issues/534).

## Implications for Myrhiza

The substrate is correct for Myrhiza's bet, but **none of these critiques is fully resolved as of 2026-05-09**. Specs that ride on the substrate must:

- Commit to **preview2 today** with an explicit migration plan to preview3, accepting the dual-stack tax. (See [`spec.md`](spec.md) for the preview2 set.)
- Treat Wasm-GC support as **out of scope for state-apply components in v1** — Rust/C/Zig only, no Java/Kotlin. Revisit when [#525](https://github.com/WebAssembly/component-model/issues/525) lands.
- Specify the **exact wkg / OCI registry topology Myrhiza uses** — do not depend on a "canonical" registry that does not exist.
- Pin **wasmtime exact version** (`v44.0.1` as of 2026-04-30) and **wasm-tools exact version** (`v1.248.0` as of 2026-04-28) per spec; tolerate upstream churn explicitly.
- Treat **WIT semver downgrade** as a real correctness hazard for the kernel: the kernel must reject a load if the host's interface version is below what the guest world required, not silently downgrade.
- Build **observability primitives ourselves** (Myrhiza-defined `myrhiza:tracing` WIT package) until wasi-otel stabilizes; do not block on upstream.
- Accept **callbacks-into-guest is a hole** — Myrhiza's authority verdict / pre-check / state-apply call shapes must not require host→guest reentrance from inside a guest call.

For neighbors carrying parallel critiques: [Iroh's critiques file](../iroh/critiques.md) (relay centralization, pre-1.0 churn) is the closest-shaped analogue; [Holochain's critiques file](../holochain/critiques.md) on toolchain immaturity outside the Rust path; [Agoric's critiques file](../agoric-endo/critiques.md) on flagship-app entanglement and kernel substrate choice.

## Sources

- `https://github.com/WebAssembly/component-model/milestone/1` — Preview 3 milestone, created 2023-08-22, still open as of 2026-05-09.
- `https://github.com/WebAssembly/component-model/issues/412`, `#525`, `#540`, `#534`, `#609`, `#641`, `#643`, `#648` — all verified via `gh api`.
- `https://github.com/bytecodealliance/componentize-py/issues/98` — verbatim quote from 2024-07-16.
- `https://github.com/bytecodealliance/ComponentizeJS/issues/291` — verbatim from 2025-09-08.
- `https://github.com/bytecodealliance/wit-bindgen/issues/1604`, `#1587`, `#1585`, `#1582`, `#586` — wit-bindgen ergonomics issues.
- `https://github.com/bytecodealliance/jco/issues/1381`, `#1383`, `#500`, `#668`, pull `#1455` — jco resource and async issues.
- `https://github.com/WebAssembly/WASI/issues/873`, `#886`, `#646` — WASI registry and observability issues.
- `https://github.com/spinframework/spin/issues/3485` — verbatim p3 template breakage.
- `https://github.com/leptos-rs/leptos_wasi/issues/18` — verbatim p3 migration venting.
- `https://github.com/denoland/deno/issues/31314`, `oven-sh/bun#24867` — browser-host CM gap.
- `https://github.com/wasmerio/wai/issues/36` — Wasmer separate trajectory.
- `https://github.com/bytecodealliance/wasmtime/issues/4109`, `#12990` — wasmtime perf knobs.
- `https://github.com/gofractally/psibase/issues/1703` — adopter critique citing BA Zulip on callbacks.
- All issue dates and quote text verified via `gh api repos/.../issues/N` between 2026-05-09T18:00:00Z and 2026-05-09T19:00:00Z.
