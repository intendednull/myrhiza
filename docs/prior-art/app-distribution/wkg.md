**Date:** 2026-05-22
**Status:** active
**Subject:** `wkg` — Bytecode Alliance's wasm-pkg-tools. The component-aware client on top of OCI.

# `wkg` (wasm-pkg-tools)

## What it is, in one sentence

`wkg` (short for "**w**asm pac**k**a**g**e keeper", pronounced "weak" in BA conversations) is the Bytecode Alliance's command-line tool + Rust library family for fetching and publishing **WASM components and WIT interface packages** to OCI registries — analogous to what `cargo` is to crates.io or `npm` is to the npm registry, but layered on top of generic OCI registries instead of a dedicated central index.

## Repository

- **Repo:** [`github.com/bytecodealliance/wasm-pkg-tools`](https://github.com/bytecodealliance/wasm-pkg-tools)
- **License:** Apache-2.0 WITH LLVM-exception
- **Current release (verified 2026-05-22):** v0.15.0, published 2025-02-06
- **Crate:** `wkg` on crates.io, currently 0.15.0; companion crates `wasm-pkg-client`, `wasm-pkg-common`, `wasm-pkg-core`
- **Stewardship:** Bytecode Alliance, small-team active development (see Bus factor section below)

## Naming + restructure history

Worth getting right because the brief flagged this and the project has had three names:

1. **`wkg` (early 2024)** — initial release as a thin OCI client for WASM components, single repo.
2. **`wasm-pkg-tools` (mid 2024)** — repo restructured into a Cargo workspace with library crates (`wasm-pkg-common`, `wasm-pkg-client`, `wasm-pkg-core`) plus the `wkg` CLI on top. The repo was renamed but the binary name stayed `wkg`.
3. **(no further rename as of 2026-05-22)** — the project is stable under `wasm-pkg-tools`. Beware of older blog posts that say "wkg is its own repo"; that hasn't been true since the workspace restructure.

**Critical disambiguation:** `wkg` is the **client tool**. `warg` (different project: [`bytecodealliance/registry`](https://github.com/bytecodealliance/registry)) was the **server-side registry protocol** — archived 2025-07-28 with the explicit note *"Work on an OCI-based registry system continues in the bytecodealliance/wasm-pkg-tools repository."* The lineage went: warg-the-protocol (2022–2025) → wkg-on-OCI (2024–present). OCI won; warg is dead. See [`registries.md`](./registries.md) §warg-postmortem for the longer story.

## What `wkg` does

Two distinct functions, often confused:

### Function 1: WIT package resolution

A component's WIT world has imports from interface packages: `wasi:http@0.2.0`, `wasi:cli@0.2.0`, `my-org:my-iface@1.0.0`. `wkg` resolves these names to OCI artifacts in a registry, mediated by `~/.config/wasm-pkg/config.toml`:

```toml
default_registry = "wa.dev"

[namespace_registries]
wasi = "ghcr.io/webassembly"
my-org = "ghcr.io/my-org"

[registry."ghcr.io"]
default = { type = "oci" }
```

The mapping is **two-stage**: (namespace → registry) then (registry → backend type). Default registries can be overridden per-namespace. Authentication is delegated to the registry's standard `docker login` / `oras login` credential store — no `wkg`-specific keychain.

```
wkg get wasi:http@0.2.0           # fetch a WIT package, writes ./wit/deps/http/
wkg get wasi:http@0.2.0 -o foo.wasm  # fetch as a single file
wkg wit fetch                     # populate ./wit/deps from wkg.toml
wkg wit build                     # encode local WIT into a `.wasm` package
```

This integrates with `cargo-component` and `wit-bindgen` so that `cargo component build` will transparently resolve missing WIT deps via `wkg`. Spin 2.6+ uses this code path internally; see [`spin/sdks-and-tooling.md:188-214`](../spin/sdks-and-tooling.md).

### Function 2: OCI push / pull for components

The low-level "I have a component, put it in the registry" surface:

```
wkg oci push ghcr.io/foo/bar:0.1.0 my-component.wasm
wkg oci pull ghcr.io/foo/bar:0.1.0           # writes my-component.wasm
wkg publish my-component.wasm                # publishes per wkg.toml's [publish] config
```

Under the hood, `wkg oci push` produces a manifest with `artifactType: application/vnd.bytecodealliance.component.v0+wasm` (the CNCF TAG Runtime layout) and a single `application/wasm` layer.

This is roughly what `oras push` does, but `wkg` adds:
- Automatic CM media-type selection based on the file's section layout (module vs component, WIT package vs runnable component).
- A `--dry-run` mode that validates the manifest without pushing.
- Integration with the `wkg.toml` namespace map (so you can `wkg publish` without typing the full registry path).

## `wkg.toml`

Per-project config, sits at the repo root:

```toml
[package]
name = "foo:bar"
version = "0.1.0"

[dependencies]
"wasi:http" = "0.2.0"
"my-org:auth" = "1.2.0"

[publish]
registry = "ghcr.io/foo"
```

The `[dependencies]` table is the canonical "what does my WIT world need". `wkg wit fetch` reads this and populates `./wit/deps/`. The shape deliberately echoes `Cargo.toml` to lower cognitive load.

## Backends

`wkg` v0.15.0 supports two registry backends:

- **`type = "oci"`** — generic OCI registry. The dominant production path. Works against everything in the [registry compliance table](./oci-artifacts.md#registry-compliance-landscape-verified-2026).
- **`type = "warg"`** — the now-dead Wasm Registry protocol. Code path exists but server-side is archived; treat as legacy.

Earlier `wkg` releases (≤0.10) had experimental backends (`type = "local"` for filesystem, `type = "wa.dev"` for the Bytecode Alliance-hosted shared registry). The local backend remains; `wa.dev` is now just an OCI-typed registry.

## Bus factor — honest assessment

Per the brief's "wkg is small-team Bytecode Alliance" callout — flag honestly:

- **Top contributors (as of 2026-05):** ~5 active maintainers, all Bytecode Alliance or BA-member-company employees (Cosmonic, Fermyon-now-Akamai, Microsoft, Fastly). [Lann Martin](https://github.com/lann) (Fermyon-now-Akamai) is the closest thing to a primary architect.
- **Release cadence:** ~6 weeks between minor releases; 9 minor releases between 2024-06 and 2025-02. Not abandoned, but not Spin-velocity either.
- **External adopters:** Spin (built-in), `cargo-component` (build-time dep), `wasmtime serve` (component fetching), several BA-internal demos. Not yet a third-party ecosystem of plugins / extensions.
- **What would happen if BA defunded it?** The OCI-as-component-registry pattern would survive — `oras` does most of the same job — but the WIT-package-resolution piece would stall. Myrhiza spec authors should not assume `wkg` is permanently load-bearing infrastructure; budget for re-implementing WIT resolution against `oras-rs` if needed.

This is structurally less resilient than (e.g.) `oras` itself (CNCF Sandbox, 13+ active maintainers across companies) or `cosign` (OpenSSF Incubating, 200+ contributors). `wkg` is the BA's package-tool side project that everyone agrees should exist, not a project with a foundation behind it.

## Practical commands cheatsheet

```bash
# project setup
wkg init                                # create wkg.toml in current dir
wkg wit fetch                            # download WIT deps to ./wit/deps/

# inspect
wkg oci ls ghcr.io/foo/bar               # list tags
wkg oci show ghcr.io/foo/bar:0.1.0       # show manifest + decoded WIT world

# build + publish
wkg wit build -o my-iface.wasm          # encode local WIT into a CM package
wkg publish my-iface.wasm               # push per [publish] in wkg.toml
wkg oci push ghcr.io/foo/bar:0.1.0 component.wasm   # direct path

# config
wkg config show                          # print effective resolved config
wkg config edit                          # open ~/.config/wasm-pkg/config.toml
```

## Implications for Myrhiza

**Yes-borrow:** the `wkg.toml` shape (Cargo-style WIT dep table + a `[publish]` block) is the right manifest format for declaring app-side WIT imports. Myrhiza apps will need an equivalent. Don't invent something new — if Myrhiza apps are CM components, they already have a `wkg.toml` story for free.

**Maybe-borrow:** the WIT-package-resolution machinery. If Myrhiza apps `import "wasi:keyvalue@0.2.0"` from the kernel, the kernel needs to validate the world against a known WIT package. Cargo-style "fetch from registry into `./wit/deps/`" is one viable shape; an alternative is "Myrhiza ships canonical WIT packages in-tree." We'll need to decide before the first app-bundle spec.

**Borrow-with-caveats:** `wkg oci push/pull` as the developer-facing CLI verb. But Myrhiza's distribution story is P2P-first, not registry-first. The right Myrhiza UX may be `myrhiza publish` that does *both* OCI push (for discoverability) *and* iroh-blob-store seeding (for actual P2P delivery). `wkg` is one of the two halves.

**Don't-borrow:** the `warg` backend code path. Dead branch.

**Bus-factor caveat:** small-team BA-stewarded; do not assume `wkg` will exist permanently. If we hard-bake against it in a Myrhiza spec, list `oras` as the documented fallback.

## Sources

- wasm-pkg-tools: <https://github.com/bytecodealliance/wasm-pkg-tools>
- crates.io `wkg`: <https://crates.io/crates/wkg>
- Bytecode Alliance Wasm Package Discussion: <https://github.com/bytecodealliance/wasm-pkg-tools/discussions>
- warg archive note: <https://github.com/bytecodealliance/registry>
- Spin SIP-008 (uses wkg since 2.6): <https://github.com/spinframework/spin/blob/main/docs/content/sips/008-using-oci-registries.md>
- Lann Martin's wkg design talk (referenced in [Cosmonic blog](https://cosmonic.com)): see also wasmCloud weekly meeting notes
- CNCF TAG Runtime WASM OCI layout: <https://tag-runtime.cncf.io/wgs/wasm/deliverables/wasm-oci-artifact/>
