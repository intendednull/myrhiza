**Date:** 2026-05-09
**Status:** active
**Subject:** Lessons from Spin prior art for Myrhiza

# Lessons for Myrhiza

The decision-relevant synthesis. Other files are evidence; this file is what we take away.

Spin sits at the *opposite design point* from Myrhiza on several axes (request-driven vs deterministic-state-apply, stateless vs stateful, single-host-multi-tenant vs P2P). So lessons here are mostly **borrow specific patterns**, not "use Spin as a model." Format: validates / avoid / borrow / open questions.

## Validates

Spin prior art **confirms** these Myrhiza design bets:

- **WIT-imports-as-permissions is the right shape.** Spin's `spin.toml` `allowed_outbound_hosts` / `key_value_stores` / `databases` declarations are the manifest-static analog of Myrhiza's "capabilities are the only host surface." A component cannot do what its manifest doesn't declare. This is exactly Myrhiza's kernel-mediated-capability model. Spin's evidence: this works in production at Akamai-scale traffic.
- **OCI artifacts are the right component distribution format.** Spin pioneered shipping WASM components as OCI artifacts (~2022, ahead of the BA-stewarded `wkg` tooling). The pattern is now well-understood: components push/pull through standard OCI registries (GHCR, Docker Hub, custom). Myrhiza app bundles should follow.
- **`componentize-*` build paths work for non-Rust languages.** ComponentizeJS, componentize-py, TinyGo wasip2 demonstrate that JS/Python/Go developers can author Component Model components without learning Rust. Myrhiza apps written in any of those languages will use the same toolchain.
- **`wac` for component composition is the right primitive.** Build-time composition via `wac` lets developers split apps into multiple components (one per concern) and ship a single composed artifact. Spin uses this; Myrhiza should too.
- **CNCF Sandbox is a viable governance posture for an early-stage WASM CM project.** Both Spin and SpinKube are Sandbox; the path to Incubating is clear. Compare to wasmCloud's Incubating posture which took 4+ years.
- **Bytecode Alliance membership matters.** Spin's BA membership keeps the project aligned with substrate evolution (Wasmtime, Component Model, WASI). Myrhiza-as-host should likewise track BA work.

## Avoid

Spin prior art shows where the **easy mistakes** are:

- **Don't model Myrhiza components as request-handlers.** Spin's per-trigger-instantiation model is fundamentally request-shaped. Myrhiza `state-apply` is *not* request-shaped — it is `(prior, event) → next` shaped. Treat Spin's runtime model as instructive, not as a template.
- **Don't assume Spin's `1ms cold start` claim translates.** Spin's claim is for warm, pre-instantiated components. Cold-cold starts pay Wasmtime engine + component-instantiation cost (~30-100ms depending on toolchain — Rust faster than JS/Python). For Myrhiza this matters because P2P apps may not have a "warm pool" — every kernel invocation might be cold.
- **Don't skip determinism analysis on borrowed components.** Spin makes no determinism claims; its components routinely call `wasi:clocks/wall-clock`, `wasi:random`, and outbound HTTP. A Spin-shaped component *will not* be a Myrhiza `state-apply` component without changes. The borrow boundary is at the *pattern* level (factors, capability binding), not at the *component* level.
- **Don't depend on Spin's runtime composition being available.** Spin only does composition at build time via `wac`. Runtime composition (loading a component and binding its imports dynamically based on policy) is wasmCloud-v1's lattice model, retired in v2 — and Spin never had it. If Myrhiza wants runtime composition, that's Myrhiza's design problem.
- **Don't conflate Spin (the runtime) with Akamai Functions (the commercial product).** Akamai Functions is closed-source SaaS built on Spin. The OSS project is Spin itself. Myrhiza will integrate against Spin (or its design patterns), not Akamai Functions.
- **Don't underestimate the bus factor risk on Akamai stewardship.** 9 of 10 top Spin contributors are Fermyon-now-Akamai. Akamai has committed to OSS continuity, but a single-corporate-steward CNCF Sandbox project has structurally less resilience than wasmCloud's multi-vendor Incubating posture. If Akamai deprioritizes Spin, the project's continuity depends on community pickup that may not materialize.
- **Don't pick the `fermyon:spin/*` legacy WIT namespace.** Use the `spin:*` 3.x/4.x namespace if borrowing patterns. The `fermyon:*` names exist for backward compat but are discouraged.

## Borrow

Specific patterns Myrhiza host design should **steal**:

- **Factor architecture (SIP-021).** The `Factor` trait + `init` / `configure_app` / `prepare` lifecycle is the right shape for Myrhiza's per-capability runtime modules. Each Myrhiza host capability (network, storage, identity, MLS, CRDT) is a "factor" with the same init-configure-prepare lifecycle.
- **Per-key configuration inheritance (SIP-023).** When a parent component spawns a child, only the explicitly granted configuration keys are inherited. Cleaner than "child inherits all of parent's environment." Myrhiza's component composition story should adopt this discipline.
- **Manifest-static capability declaration (`spin.toml`).** A component's permissions are declared in the manifest, *not* requested at runtime. Auditable, static, easy to reason about. Myrhiza app bundles should declare capabilities the same way.
- **OCI artifacts + `wkg` package resolution.** Spin's component-as-OCI-artifact + `wkg`-resolves-WIT-packages is the right distribution shape. Myrhiza app distribution should follow exactly.
- **CLI shape: `spin new` / `build` / `up` / `watch` / `registry push`.** This verb set has converged across the WASM-component developer experience. Myrhiza's developer CLI should adopt the same vocabulary so onboarding from Spin/wasmCloud/Wasmer is friction-free.
- **`SpinApp` / `SpinAppExecutor` separation (SpinKube).** SpinApp = "what to run"; SpinAppExecutor = "how to run it". The decoupling is clean. Myrhiza-app vs Myrhiza-app-execution-policy could borrow this shape (though Myrhiza is P2P, not K8s).
- **componentize-* tooling.** ComponentizeJS, componentize-py, TinyGo wasip2 are the right build paths for non-Rust languages. Myrhiza apps in those languages should use them unchanged.
- **`spin watch` developer hot-reload.** Local-dev hot-reload of a component is a quality-of-life feature Myrhiza developers will expect.

## Open questions

Myrhiza spec authors should address, with this corpus loaded:

- **What's Myrhiza's analog of Spin's factor?** Likely "host capability" — but factor's init/configure_app/prepare lifecycle is a more specific pattern worth borrowing wholesale.
- **What's Myrhiza's manifest format?** `spin.toml`-shaped TOML with capability declarations? `Cargo.toml`-shaped? Something new? Decide early; the format propagates everywhere.
- **How does Myrhiza handle multi-component apps?** `wac`-style build-time composition is a good starting point. Runtime composition is a stretch goal.
- **What's the Myrhiza equivalent of `spin up` for local dev?** Single-binary kernel-as-dev-server is one option. Embedded-mode-in-the-IDE is another.
- **How does Myrhiza distribute components?** OCI artifacts via `wkg` is the expected answer; verify it fits the P2P case.
- **Does Myrhiza track Akamai's Spin direction?** Probably not directly — different design points — but watch for SIP-shaped designs that translate (e.g. SIP-023 capability inheritance is highly relevant).

## Recommendation matrix for Myrhiza

If Myrhiza is sketching its host architecture today and looking for Spin-inspired patterns:

| Myrhiza concern | Spin pattern to borrow | Source file in this folder |
|---|---|---|
| Per-capability runtime module | **Factor architecture (SIP-021)** | [architecture.md](architecture.md) |
| Capability inheritance across composition | **`inherit_configuration` (SIP-023)** | [architecture.md](architecture.md) |
| App manifest format | **`spin.toml` shape** | [triggers-and-components.md](triggers-and-components.md) |
| Component distribution | **OCI artifacts + `wkg`** | [sdks-and-tooling.md](sdks-and-tooling.md) |
| Multi-component app composition | **`wac` build-time** | [sdks-and-tooling.md](sdks-and-tooling.md) |
| Non-Rust language SDKs | **componentize-js / componentize-py / TinyGo wasip2** | [sdks-and-tooling.md](sdks-and-tooling.md) |
| Developer CLI shape | **new / build / up / watch / registry push** | [sdks-and-tooling.md](sdks-and-tooling.md) |
| Local dev hot-reload | **`spin watch`** | [sdks-and-tooling.md](sdks-and-tooling.md) |
| Component invocation lifecycle | **NOT directly applicable** — Spin's per-trigger model differs from `state-apply` | [architecture.md](architecture.md) |
| Capability mediation across runtime | **factor + manifest, NOT lattice/link-definitions** | [architecture.md](architecture.md), [`../wasmcloud/`](../wasmcloud/) |

## Recommended posture for the runtime spec

A defensible default given the corpus:

1. **Adopt Spin's factor architecture as the host-capability internal pattern.** SIP-021's Factor trait is well-shaped for Myrhiza's per-capability runtime modules.
2. **Adopt Spin's manifest-static capability declaration shape.** Myrhiza app bundles declare capabilities up-front; the kernel enforces. Auditable, static.
3. **Adopt OCI artifacts + `wkg` for component distribution.** Standard pattern across the WASM ecosystem.
4. **Don't model Myrhiza components as request-handlers.** `state-apply` is its own shape; Spin's request-driven model doesn't translate.
5. **Track but don't depend on Spin governance.** Akamai stewardship is single-vendor; the bus factor is real. Watch SIPs for transferable design patterns; don't depend on Akamai keeping the project's direction aligned with Myrhiza's.

## Sources

This file synthesizes from sibling files. Primary sources cited per sibling:

- [architecture.md](architecture.md), [triggers-and-components.md](triggers-and-components.md) — Spin runtime mechanics
- [sdks-and-tooling.md](sdks-and-tooling.md), [spinkube.md](spinkube.md) — developer + deployment surface
- [governance.md](governance.md), [comparisons.md](comparisons.md) — Akamai acquisition + design-space positioning
- [open-problems.md](open-problems.md), [critiques.md](critiques.md) — gaps + third-party voices
- Spin Improvement Proposals (SIPs): <https://github.com/spinframework/spin/tree/main/docs/content/sips>
- Akamai press release: <https://www.akamai.com/newsroom/press-release/akamai-announces-acquisition-of-function-as-a-service-company-fermyon>
