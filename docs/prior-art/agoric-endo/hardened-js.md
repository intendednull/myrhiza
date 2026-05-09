**Date:** 2026-05-09
**Status:** active
**Subject:** Agoric/Endo — Hardened JavaScript: SES, lockdown, compartments, harden

# Hardened JavaScript

"Hardened JavaScript" is the security substrate Agoric runs on. It is a layered
construction on top of standard ECMAScript: the SES shim freezes the JavaScript
realm at startup, `harden()` recursively freezes object surfaces afterward, and
`Compartment` provides per-tenant globals + module loaders that share the frozen
intrinsics. There is no OS process, no V8 isolate, and no WASM sandbox involved.
Isolation is purely linguistic.

This is the most production-hardened ocap stack in JS, but the threat model is
narrower than people often assume. Read [Limitations](#limitations) before
modeling Myrhiza on this.

## SES (Secure ECMAScript)

SES is a subset / reinterpretation of ECMAScript in which the realm's intrinsics
are deeply frozen. It ships as the [`ses`](https://www.npmjs.com/package/ses)
npm package — a runtime shim, not a separate engine. Hosts apply it by calling
`lockdown()` once at process start.

- `ses` 2.0.0 — published 2026-04-17
- `ses` 1.15.0 — published 2026-02-26 (the version MetaMask Snaps still pins; see [`./apps.md`](./apps.md))
- `ses` 1.10.0 — published 2024-11-13
- `ses` 1.0.0 — published 2023-12-12

After `lockdown()` runs, the world is different in three ways:

1. **Frozen primordials.** `Object.prototype`, `Array.prototype`,
   `Function.prototype`, `String.prototype`, `Promise.prototype`, etc. and the
   constructors themselves are transitively frozen. Prototype pollution and
   monkey-patching of built-ins are eliminated by construction.
2. **Tamed sources of ambient authority.** `Math.random()`, `Date.now()`, and
   `new Date()` (no-argument form) throw `TypeError` by default. Locale-sensitive
   methods get deterministic shims. `console`, error stacks, and regex behavior
   are tamed to remove side channels and to make stacks non-leaky across trust
   boundaries.
3. **Safe `eval` / `Function`.** Direct `eval` is rejected by a regex-based
   censor (the shim cannot fully emulate the optimization the spec gives direct
   `eval`). The censor will [false-positive on legitimate code containing
   `<!--`, `-->`, or certain `import(...)` shapes](https://github.com/endojs/endo/blob/master/packages/ses/docs/guide.md);
   that is a known sharp edge of the regex approach.

The intrinsics are *shared* across compartments rather than re-created per
compartment. This is deliberate: it preserves "identity continuity" so that an
`Array` constructed in one compartment is `instanceof Array` in another, and
data can flow across compartment boundaries without re-marshalling primitives.

### `lockdown()` taming options

`lockdown()` accepts knobs that trade safety for compatibility, e.g.
`errorTaming`, `dateTaming`, `mathTaming`, `regExpTaming`, `localeTaming`,
`consoleTaming`, `stackFiltering`, `overrideTaming`. The defaults are the safe
choice. Hosts that need (e.g.) real `Date.now()` must opt in explicitly and
accept that their guests can now observe wall-clock time.

## `harden()`

`harden(value)` recursively walks the own-property graph and applies
`Object.freeze`. It stops at already-hardened values. The semantics:

- A hardened object's *surface* is immutable: properties cannot be added,
  reassigned, or reconfigured. Methods can still be called.
- Hardening is *transitive across own properties* but does not pierce closures.
  A hardened object whose method closes over a mutable `Map` keeps that `Map`
  mutable. ([SES guide](https://github.com/endojs/endo/blob/master/packages/ses/docs/guide.md):
  "being hardened doesn't preclude an object from having access to mutable
  state.")
- `harden()` is the precondition for crossing a trust boundary. The marshal
  layer (see [./capabilities.md](./capabilities.md)) refuses to serialize
  un-hardened passables.

A near-future TC39 proposal called **Stabilize** (Stage 1, no shipping date)
adds three "integrity traits" — *Fixed*, *Overridable*, *Non-trapping* — and
notably makes hardened proxies stop trapping to their handler, closing a class
of reentrancy attacks on Proxy targets. `harden()` will adopt this when it
lands. Today, hardening a Proxy does not stop the handler from being called.

## Compartments

A `Compartment` is an in-realm sandbox: its own `globalThis`, its own module
namespace (loader hooks supplied by the host), and shared frozen intrinsics
from the parent realm. A guest gets exactly what the host hands it via
`globals` and the module map. Default new globals are `undefined`.

```js
lockdown();
const c = new Compartment({
  globals: { print: harden(console.log) },
  modules: { /* host-curated module map */ },
});
c.evaluate('print("hello")');
```

The compartment's `globalThis` starts mutable. Hosts must call
`harden(c.globalThis)` before letting untrusted code run if they want the
guest's view of the world to be locked down. If they don't, the guest can mutate
its own globals (which is sometimes deliberate — guests can run setup, then
freeze themselves).

Compartments do **not** create a new realm. There is one realm per agent
(one Node process, one browser tab), and all compartments inside it share
intrinsics. Two compartments cannot exchange identity-bearing primordials in a
way that lets one observe the other's mutations — because there are none.

### Compartment maps

The [`@endo/compartment-mapper`](https://github.com/endojs/endo/blob/master/packages/compartment-mapper)
package builds a compartment-per-package map from a Node-style `package.json`
graph. Each npm dependency lands in its own compartment with a curated module
map describing exactly which modules from which other compartments it can
import. This is the runtime expression of "every package gets least authority".
See [./modules-and-bundling.md](./modules-and-bundling.md).

## Limitations

The SES threat model is explicit about what is *not* covered. From the
[secure-coding guide](https://github.com/endojs/endo/blob/master/packages/ses/docs/secure-coding-guide.md):

- **Resource exhaustion is not contained.** A guest can `while (true) {}`,
  allocate until OOM, or recurse to stack overflow. SES has no metering.
  Agoric's SwingSet adds metering as a separate kernel-level concern; SES
  itself does not.
- **Timing channels are not closed.** SES tames `Date.now()` and
  `performance.now()`, but a guest with access to `Promise` can still build a
  scheduling-based clock. From the SES README: "Any two JavaScript programs
  sharing a SharedArrayBuffer can use the shared buffer to construct a high
  resolution timer." Hosts must not hand `SharedArrayBuffer` to mutually-
  suspicious guests.
- **Engine-level side channels are out of scope.** Spectre, cache timing, GC
  pauses observable across compartments — SES makes no claim to defend.
- **Reentrancy is partially mitigated, not eliminated.** Synchronous calls into
  a guest object run on the caller's stack. Stabilize/Non-trapping is the
  proposed structural fix; today, defensive code defers callbacks via
  Promises (see the secure-coding guide).
- **`class` syntax has no good ocap pattern.** The SES team explicitly says:
  "We do not yet have a good pattern that meets these goals and also uses the
  JavaScript `class` syntax." The recommendation is closure-based factory
  functions or `Exo` (see below).
- **Regex censor false positives.** Code containing `<!--`, `-->`, or
  certain `import(` shapes near parens may be rejected even when safe. This is
  inherent to using a regex on source text instead of parsing.
- **No `WebAssembly`, no `Buffer`, no `process`, no `setTimeout`, no `URL`,
  no `TextEncoder`/`TextDecoder` by default** in a locked-down compartment.
  Hosts that want them must explicitly endow them, taking responsibility for
  whatever authority leaks.

The honest framing: **SES makes mutually-suspicious *cooperating* code safe,
provided the host is careful and the engine is uncompromised.** It does not
make a hostile engine safe, and it does not contain a guest that wants to burn
CPU or RAM.

## Exo

[`@endo/exo`](https://github.com/endojs/endo/tree/master/packages/exo) layers
*input validation* on top of `Far()`. An Exo is a remotable object guarded by
an `InterfaceGuard` that auto-validates argument and return shapes at every
method boundary. There are three flavors:

- `makeExo(label, guard, methods)` — single instance.
- `defineExoClass(...)` — many instances, per-instance state.
- `defineExoClassKit(...)` — multiple "facets" of an object sharing state, the
  ocap idiom for least-authority capability splitting (e.g. `mintFacet` vs
  `purseFacet`).

For new code on top of hardened JS, Exo is the recommended pattern; bare
`Far()` remains valid but skips input validation. See
[./capabilities.md](./capabilities.md).

## Test stack: `@endo/ses-ava` and `@endo/test-ava`

[`@endo/ses-ava`](https://github.com/endojs/endo/blob/master/packages/ses-ava/README.md)
(1.4.1, 2026-04-16) wraps the AVA test runner so tests execute under SES with
*debug-friendly* tamings: deep stacks across promise turns, unredacted error
messages, full stack traces. Standard SES redacts these for security in
production; the test wrapper undoes that for local visibility.

The intended use is `import { test } from '@endo/ses-ava/prepare-endo.js'`.
Add it to `devDependencies` only. The package also offers a small CLI for
running the same suite with and without lockdown via `sesAvaConfigs` in
`package.json`, which is useful for catching things that pass when SES is on
but break under raw Node (or vice versa).

`@endo/test-ava` is referenced in the brief; the npm registry currently lists
it as a `null` dist-tag (no published version). Treat the active package as
`@endo/ses-ava`.

## Implications for Myrhiza

We do not run JavaScript guests, so we do not need SES. But the architecture
is highly transferrable:

1. **Frozen primordials → deterministic component imports.** Where SES freezes
   `Array.prototype` to make cross-compartment data flow stable, our
   `state-apply` profile demands the analogous property at the component
   boundary: every WIT-imported function must be a deterministic helper. If a
   host import is added (`now()`, `random()`, network, disk), `state-apply`
   must not be allowed to import it. This is the same discipline as SES
   refusing to give compartments `Date.now()` by default.
2. **`harden()` → state-event freezing.** Before passing an event into
   `state-apply`, we should treat it as immutable in a way enforceable at the
   ABI. The Component Model gives this for free for value types, but it does
   *not* give it for resources. If we ever pass resource handles into
   `state-apply`, we need a `harden`-equivalent invariant: no mutation, no
   ambient authority. The likely answer is "`state-apply` cannot import
   resource-typed functions at all".
3. **SES does not contain runaway CPU/RAM.** Neither does WASM by default.
   We will need fuel/metering at the kernel layer (Wasmtime supports this) for
   the same threats. SES's choice to leave metering to the host (SwingSet) is
   the same split we should plan for: kernel meters, components do not.
4. **Compartments-per-dependency is the model for component graphs.** The
   compartment-mapper rule "every package gets its own compartment with an
   explicit module map" is exactly the rule we want for component bundles:
   every component has its own world with explicit declared imports, and
   nothing more. See [./modules-and-bundling.md](./modules-and-bundling.md).
5. **Don't redact errors in dev.** SES-AVA's pattern of swapping in a
   debug-friendly test setup that *unredacts* errors and stacks is worth
   stealing. We will eventually want a kernel "dev mode" that surfaces
   determinism violations and capability denials with full context, separate
   from production traces.
6. **Stabilize / Non-trapping is the analog of "no synchronous re-entry into
   `state-apply`".** Whatever we build should treat re-entry into a state
   transition as a correctness bug, not a runtime hazard.
7. **The honest framing applies to us too.** Capability discipline contains
   *cooperating* code. It does not contain malicious WASM authors who
   side-channel via timing, memory pressure, or kernel-mediated I/O patterns.
   The Myrhiza spec should say so out loud rather than implying ocap is a
   panacea.

See also: [./capabilities.md](./capabilities.md) for what `Far()`, `E()`, and
the marshal/pass-style layer look like; [./modules-and-bundling.md](./modules-and-bundling.md)
for compartment-mapper and the bundle story; [./vat-model.md](./vat-model.md)
for how SwingSet uses all of the above; [./comparisons.md](./comparisons.md)
for SES vs. WASM-component sandboxing.

## Sources

- SES package, npm: https://www.npmjs.com/package/ses
- SES README: https://github.com/endojs/endo/blob/master/packages/ses/README.md
- SES guide: https://github.com/endojs/endo/blob/master/packages/ses/docs/guide.md
- SES secure-coding guide: https://github.com/endojs/endo/blob/master/packages/ses/docs/secure-coding-guide.md
- SES "preparing for stabilize": https://github.com/endojs/endo/blob/master/packages/ses/docs/preparing-for-stabilize.md
- `@endo/exo`: https://github.com/endojs/endo/tree/master/packages/exo
- `@endo/ses-ava` README: https://github.com/endojs/endo/blob/master/packages/ses-ava/README.md
- Endo repository: https://github.com/endojs/endo
- SES release history: `gh api repos/endojs/endo/releases` (verified 2026-05-09)
