**Date:** 2026-05-09
**Status:** active
**Subject:** Agoric/Endo — Modules, compartment-mapper, bundle-source, vat loading

# Modules and Bundling

This is the closest analog to Myrhiza's component-bundle distribution story:
Endo takes a Node-style package graph, partitions it into per-package
compartments under SES, and produces a single-file, content-addressed bundle
that a SwingSet kernel can install and run reproducibly. Two packages do the
work: `@endo/compartment-mapper` plans the link, `@endo/bundle-source`
materializes it.

For SES + harden + Compartments see [./hardened-js.md](./hardened-js.md). For
how the bundles are loaded into vats and managed across upgrades see
[./vat-model.md](./vat-model.md) and [./persistence.md](./persistence.md).

## The problem Endo is solving

Standard Node module loading is the opposite of ocap. Every package is loaded
into a shared realm with mutable primordials, ambient `require`/`process`/
network/disk, and a flat resolver that walks `node_modules` upward until
something matches. A malicious or merely buggy transitive dependency can
monkey-patch `Array.prototype` or read `process.env` because nothing structural
prevents it.

Endo's bet: **every npm package gets its own SES Compartment with an explicit
module map**. Imports are not resolved by walking the filesystem at runtime;
they are resolved at *bundle time* into a fixed graph, embedded in the bundle,
and re-instantiated identically on every load.

## `@endo/compartment-mapper`

`@endo/compartment-mapper` (2.1.0, 2026-04-16) takes an entry-point file and a
filesystem and produces a **compartment map** — a JSON manifest that
describes:

- one compartment per (package name, version) tuple in the dep graph;
- for each compartment, a `modules` map: `local-import-name → { compartment,
  module }` pointing into another compartment;
- which modules are "exits" (host-supplied; i.e. Node builtins or capabilities
  the host endows);
- per-compartment scope (browser-style globals it can see, parser conditions
  like `import` / `browser` / `endo` / `node`).

At load time, the SES module loader walks the map, instantiates one
`Compartment` per entry, wires `importHook`/`moduleMapHook` to follow the map,
and produces a frozen module graph. There is no runtime filesystem traversal
and no `node_modules` walking; the map is the source of truth.

Three flavors of output:

1. `makeArchive(...)` — produces a zip with `compartment-map.json` plus all
   source files; this is the bundle format used downstream.
2. `makeScript(...)` / `makeFunctor(...)` — produce a single evaluable script
   suitable for `<script>` tags or `eval`. Scripts do *not* preserve
   per-package compartment isolation; they merge all modules into one scope.
   Use for browser-side delivery only when you trust the whole graph.
3. `loadLocation(...)` — load directly from disk under SES; for development.

Multi-language support exists: ESM, CommonJS, JSON, and text modules via
pluggable parsers. Conditional exports respect `import`, `browser`, `node`,
and an `endo` condition Endo packages opt into.

### Caveats

The compartment-mapper README is explicit about what does *not* work:

- **`require()` must be statically analyzable.** "The system assumes
  CommonJS modules use only single-string `require()` calls at the top level
  for static analysis; complex require patterns break compartmentalization."
  Dynamic `require(variable)` calls cannot be planned at bundle time and will
  fail or silently break isolation.
- **No realm freezing.** The mapper builds a graph; it does *not* call
  `lockdown()`. The host is responsible for applying the SES shim before
  loading.
- **Scripts lack isolation.** The `makeScript` output is a flat bundle —
  everything in one compartment. It exists for environments that cannot run
  the SES module loader.
- **JSON-import semantics diverge from Node ESM.** Endo allows JSON imports
  uniformly across module types; standard Node ESM is stricter. This is
  intentional but a source of "works in Endo, fails in raw Node" surprises.
- **Source maps require extra plumbing.** They are generated but need
  explicit hooks during archival and additional computation when loading.

### Node builtins under SES

Most Node builtins (`fs`, `net`, `process`, `crypto`, `os`, `child_process`)
are *not* available inside a locked-down compartment. Where a package needs
one, the host must declare it as an exit and explicitly endow it. SwingSet's
vat compartment endows almost nothing — `console`, `harden`,
`HandledPromise`, `E`, and the `Compartment` constructor itself, plus the
core language objects.

There is no general-purpose "shim Buffer" or "shim setTimeout"; if a package
needs them, it either gets explicitly endowed (and the host accepts the
authority leak) or it is rejected for vat use. This is why much of npm does
not work inside Agoric vats without modification.

## `@endo/bundle-source`

`@endo/bundle-source` (4.3.0, 2026-04-16) is the build tool. Given an entry
file, it returns a single JSON-serializable bundle object. Bundle formats:

- **`endoZipBase64`** — *the* format for Agoric. A zip archive containing
  `compartment-map.json` plus every module, encoded as base64. Self-describing
  and content-addressable. Includes an optional `endoZipBase64Sha512` for
  integrity verification.
- **`nestedEvaluate`** — a script wrapping per-module sources, with a
  `nestedEvaluate(src)` function provided at load time so submodules can
  request their source by id. Preserves filenames in stack traces. Used for
  the older "submodule" loader.
- **`getExport`** — minimal: a script whose completion value is a function
  taking an optional `sourceUrlPrefix`. Supports `require` for host imports.
  The legacy format.
- **`endoScript`** — a script suitable for `<script>` tags. No `require`
  support. Newer browser-side format.

Version 4 of `bundle-source` (current) made several breaking changes over v3:

- No more live bindings (the ESM machinery that lets one module observe
  reassignment of another module's export).
- Explicit `package.json` dependency declaration required; no walking up
  parent dirs.
- Node 18+ module-format inference replaces older heuristics.

### Bundle hashes and content-addressing

For `endoZipBase64`, the bundle ID is:

```
b1- || lowercase-hex(SHA512(compartment-map.json))
```

The hash covers only the `compartment-map.json` inside the archive — the
manifest of compartments, modules, and exit points — *not* the raw module
bytes. Module sources are referenced by hash *inside* the manifest, so a
source change changes the manifest, which changes the bundle ID. The split
lets you transmit a manifest first, then ship only the missing module
chunks, and still arrive at a stable bundle ID.

This is what makes "is this the same bundle?" a primitive operation. Two
parties referring to the same bundle ID are guaranteed (under SHA-512) to
have the same module graph.

## Loading bundles back: `@endo/import-bundle`

`@endo/import-bundle` (1.6.1) takes a bundle object plus an `endowments` map
and returns the bundle's exports, freshly instantiated under SES. The bundle's
`compartment-map.json` is parsed, one Compartment per package is created, the
module graph is linked, and the entry point's exports are returned.

The host controls everything the bundle sees: globals, allowed modules
(via `modules` config), and how dynamic imports are resolved (via hooks).
A vat-compartment endowment list is small and fixed; an end-user endowment
list might be larger.

## SwingSet bundle lifecycle

[SwingSet bundles doc](https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/bundles.md):

1. **Author** writes a vat as a JS module exporting `buildRootObject`.
2. **`bundleSource(entryFile)`** is called offline; produces an
   `endoZipBase64` bundle.
3. **Bundle ID** = `b1-` + SHA-512 of the compartment-map. Stable per source.
4. **`controller.validateAndInstallBundle(bundle)`** stores the bundle in the
   kernel's bundleStore (a kvStore subspace mapping bundle ID → JSON).
5. **Static vats** are configured with bundle IDs at swingset boot time.
6. **Dynamic vats** are created at runtime via
   `E(vatAdminService).createVat(bundlecap)`, where `bundlecap` is a device
   node referring to an installed bundle. Bundlecaps are passed by reference
   between vats — much cheaper than passing the whole bundle through CapTP.
7. **Vat upgrade** swaps the bundle behind a vat ID, runs the new code's
   `buildRootObject`, and replays / migrates state. Bundle IDs make the
   "are we running the right code?" question structural.

Determinism follows from this: every validator in an Agoric chain installs
the same bundle by ID, instantiates the same compartment graph, and runs the
same code on the same inputs. The compartment-mapper's static planning makes
the resolver deterministic; SES makes the runtime deterministic; the bundle
hash makes "same code" verifiable.

## Static vs. dynamic require

A recurring theme: **Endo's bundling forbids dynamic require**. The
compartment-mapper plans the import graph statically; runtime `require` exists
only as `require('builtin')` for explicitly-endowed builtins, and even those
are static strings. There is no equivalent of Webpack's `require.context`,
no per-call code-splitting, no runtime module discovery.

This is the cost of determinism. A vat that wanted to "load this plugin if
the operator drops it in a directory" cannot exist; the operator must build a
new bundle, install it, and upgrade.

## Implications for Myrhiza

Compare line-by-line: this is the closest production system to what we want.

1. **Compartment-per-package = component-per-package.** Endo's "every npm
   package gets its own SES compartment" is the same shape as our "every
   crate-level boundary becomes a WASM component". The discipline is sound.
   Don't fight it; embrace it.
2. **Static module graph + content-addressed bundle is the right shape for
   our distribution layer.** A bundle ID = SHA of the compartment-map gives
   them: identity, dedup-by-ID transmission (ship only missing chunks),
   and trivial "are we running the same code?" checks. We want the same:
   bundle ID = hash of a manifest that names component bytecode by hash, and
   the manifest is the single source of truth for the component graph and its
   declared imports/exports.
3. **Don't hash the source bytes; hash the manifest.** Endo hashes only
   `compartment-map.json`, which references modules by hash. We should do the
   same: hash a manifest that names component-blobs by hash, not the
   concatenation of all bytes. This is what makes "transmit only deltas"
   trivial.
4. **No dynamic resolution at runtime.** Endo's strict no-dynamic-require
   rule maps directly onto our "no dynamic component instantiation outside
   the kernel". A `state-apply` component that wants to dynamically pick a
   sub-component breaks determinism the same way dynamic `require` breaks
   reproducibility. Forbid it; make plugins a bundle-level concern.
5. **The endowment list is the ABI.** SwingSet's vat compartment is endowed
   with a *fixed, small* set of globals (console, harden, E, HandledPromise,
   Compartment). That list is the vat ABI. Adding to it is an upgrade. We
   want the same discipline for component imports: the kernel's exposed
   imports are the runtime ABI, additions are versioned ABI changes, and
   `state-apply` must import a strict subset (no I/O, no nondeterminism).
6. **Bundles install before they run.** The "validate & install, then create
   vat from bundlecap" two-step is worth copying. It separates "is this
   bundle well-formed and stored?" from "instantiate a running instance".
   For us: peers install component bundles into a content-addressed local
   store, then start app instances by reference to bundle IDs. Apps refer to
   each other by bundlecap-equivalent — a handle to a content-addressed
   bundle, not the bundle bytes.
7. **Most of npm does not work in a locked-down compartment.** The Agoric
   experience is that integrating real-world libraries requires substantial
   surgery — anything that uses ambient `process`, dynamic `require`, or
   non-statically-analyzable patterns has to be forked or rewritten. Apply
   this lesson preemptively: the WASI/component ecosystem is small now, and
   we should resist pulling in dependencies that would force us to expose
   ambient authority through host imports.
8. **Bundles are upgrade boundaries.** SwingSet vat upgrades replace a
   bundle ID and re-run `buildRootObject`. This forces the upgrade story to
   be honest about what state survives. We get the same: component upgrades
   replace a bundle ID, must explicitly migrate the state model, and cannot
   silently change the determinism contract.
9. **Bundlecaps as a separate type from bundles.** Passing a 5MB bundle by
   value through every message is expensive. SwingSet's separation of
   "bundle (data)" from "bundlecap (handle to installed bundle)" is the
   right shape. Our equivalent: components reference each other by bundle ID
   handles that the kernel resolves, not by passing bundle bytes around.

See also: [./hardened-js.md](./hardened-js.md) for the SES substrate that
makes per-compartment isolation work; [./capabilities.md](./capabilities.md)
for the ocap discipline these bundles operate under;
[./distribution.md](./distribution.md) for how bundles are propagated across
the Agoric network; [./persistence.md](./persistence.md) for vat state
across bundle-driven upgrades.

## Sources

- `@endo/compartment-mapper` README: https://github.com/endojs/endo/blob/master/packages/compartment-mapper/README.md
- `@endo/bundle-source` README: https://github.com/endojs/endo/blob/master/packages/bundle-source/README.md
- `@endo/import-bundle`: https://github.com/endojs/endo/tree/master/packages/import-bundle
- SwingSet bundles doc: https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/bundles.md
- SwingSet vat-environment doc: https://github.com/Agoric/agoric-sdk/blob/master/packages/SwingSet/docs/vat-environment.md
- npm registry (versions verified 2026-05-09): https://registry.npmjs.org/@endo/compartment-mapper, https://registry.npmjs.org/@endo/bundle-source, https://registry.npmjs.org/@endo/import-bundle
- Endo release history: `gh api repos/endojs/endo/releases` (verified 2026-05-09)
