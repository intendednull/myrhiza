**Date:** 2026-05-09
**Status:** active
**Subject:** Spin (Akamai, formerly Fermyon) — request-driven serverless WebAssembly Component Model framework; sister CM runtime to wasmCloud (different design point)

# Spin prior art

Reference folder for Spin — the open-source serverless WASM Component Model framework originally built by Fermyon, acquired by Akamai 2025-12-01. 11 files, ~1,170 lines.

This is a *sister-folder to wasmcloud/* — both are production CM runtimes built on Wasmtime, but they sit at opposite design points: Spin is request-driven (HTTP-handler-shipped-per-trigger, stateless by default) where wasmCloud-v2 is K8s-orchestrated long-running. The corpus surfaces what Spin's design choices teach about component-driven serverless and the WIT-imports-as-permissions pattern.

## Key facts at a glance

| Field | Value |
|---|---|
| Repo | github.com/spinframework/spin (renamed from `fermyon/spin` on 2025-01-21 — *pre-acquisition*, driven by CNCF vendor-neutrality requirement) — 6,407 stars, 302 forks, created 2021-11-02 |
| Latest version | `v4.0.0` (2026-04-20) — major release introducing async interfaces, WIT package upgrades, Wasmtime 43.0.1, SIP-023 fine-grained capability inheritance |
| Maintenance line | `v3.6.3` (2026-04-09 security patch) |
| Wasmtime base | `v4.0.0` tag pins `43.0.1`; `main` workspace pins `44.0.0` |
| License | Apache-2.0 WITH LLVM-exception |
| WASI / Component Model | `wasm32-wasip2` (WASI Preview 2 / Component Model). v4.0 ships dual-target with Preview 3 RCs (`0.3.0-rc-2026-03-15`) alongside Preview 2 |
| Official SDKs | JS/TS, Rust, Go (TinyGo), Python (componentize-py); third-party Zig, Moonbit |
| Triggers | HTTP, Redis (custom triggers authorable in Rust) |
| Composition | `wac` (build-time only — no runtime composition like wasmCloud-v1's lattice) |
| CNCF status | **Sandbox** — both `spin` and `spinkube` accepted 2025-01-21 |
| Bytecode Alliance | Member; Fermyon (now Akamai) team contributes to Wasmtime, Component Model, WASI |
| Stewardship | Akamai (post-2025-12-01); 9 of 10 top contributors are Fermyon-now-Akamai |
| Co-founders | Matt Butcher, Radu Matei (joined Akamai's Cloud Technology Group post-acquisition) |
| Pre-acquisition funding | $26M total; $20M Series A from Insight Partners (announced 2022-10-24) |

## How to use

Read in this order:

1. **[architecture.md](architecture.md)** — runtime architecture: trigger model, factor execution lifecycle, Wasmtime embedding, SIP-021 `Factor` trait, SIP-023 `inherit_configuration` grant model.
2. **[triggers-and-components.md](triggers-and-components.md)** — HTTP/Redis triggers, `wit/world.wit` imports/exports, `spin.toml` manifest, OCI distribution, capability binding semantics.
3. **[sdks-and-tooling.md](sdks-and-tooling.md)** — official SDKs (Rust most mature, JS via ComponentizeJS, Go via TinyGo wasip2, Python via componentize-py), third-party Zig/Moonbit, `wac` composition, `wkg` package resolver, OCI artifact layout, local dev (`spin watch`, variables/secrets).
4. **[spinkube.md](spinkube.md)** — Kubernetes operator (spin-operator v0.6.1), containerd shim (containerd-shim-spin v0.24.0), runtime-class-manager, `SpinApp` / `SpinAppExecutor` CRDs.
5. **[governance.md](governance.md)** — Fermyon→Akamai timeline, $20M Series A 2022-10-24 from Insight Partners, acquisition 2025-12-01, post-acquisition OSS commitments (Spin + SpinKube CNCF Sandbox, Bytecode Alliance membership), bus factor analysis (9 of 10 top contributors are Fermyon/Akamai).
6. **[comparisons.md](comparisons.md)** — Spin vs wasmCloud (request-driven vs long-running), vs Cloudflare Workers (open-source vs proprietary V8), vs AWS Lambda (Wasmtime vs Firecracker), vs Hyperlight Wasm (Sandbox 2025-03-04), vs Wasmer Edge.
7. **[critiques.md](critiques.md)** — third-party criticism. Java Code Geeks "Three Years of Almost Ready," Cloudflare engineering on V8 vs WASM, LowEndBox on the Linode-acquisition-aftermath precedent, Glauber Costa.
8. **[open-problems.md](open-problems.md)** — what Spin doesn't solve. Stateless-only, websocket caveats, cold-start nuance by toolchain, no determinism, build-time-only composition, observability gaps, qualified portability.
9. **[lessons.md](lessons.md)** — *the decision file*. Validates / avoid / borrow + recommendation matrix.
10. **[glossary.md](glossary.md)** — Spin-specific terms.

If you only have time for two files: read **lessons.md** + **architecture.md**.

## Why this folder exists

Myrhiza is a P2P WASM Component Model runtime with deterministic `state-apply` semantics. Spin sits at the *opposite* design point on several axes:

- Spin assumes HTTP-handler shape; Myrhiza assumes pure-function-of-`(prior, event)` shape.
- Spin assumes stateless components; Myrhiza assumes stateful (state-apply produces next state).
- Spin makes no determinism claims; Myrhiza requires it.
- Spin assumes a server topology (single host, multi-tenant); Myrhiza assumes P2P (each peer is a host).

So why study it? Three reasons:

1. **Factor architecture (SIP-021/023)** — Spin's per-factor capability mediation pattern is directly applicable to Myrhiza's kernel-cap design.
2. **`spin.toml` manifest as capability declaration** — manifest-static `allowed_outbound_hosts` / `key_value_stores` is the right shape for Myrhiza app-bundle capability declaration.
3. **OCI distribution + componentize-* build paths** — Spin pioneered the WASM-component-as-OCI-artifact distribution model; Myrhiza app distribution should follow.

The corpus completes the WASM-platform survey alongside [`../wasm-component-model/`](../wasm-component-model/) (the substrate) and [`../wasmcloud/`](../wasmcloud/) (the long-running CM runtime).

## Akamai acquisition context (2025-12-01)

**Akamai acquired Fermyon on 2025-12-01** ([press release](https://www.akamai.com/newsroom/press-release/akamai-announces-acquisition-of-function-as-a-service-company-fermyon)). Acquisition price undisclosed. Co-founders Matt Butcher and Radu Matei joined Akamai's Cloud Technology Group; the Fermyon Cloud product was sunset in favor of Akamai Functions (the productized commercial form). `fermyon.com` redirects to `akamai.com/products/akamai-functions`.

Akamai committed in the press release to:

- Continuing Spin and SpinKube as open-source CNCF projects
- Continuing Fermyon's Bytecode Alliance membership
- Maintaining the existing OSS contributor relationships

A reader auditing whether Akamai stewardship is healthier or worse than Fermyon's previous independence should weigh:

- **Pre-acquisition Fermyon** was VC-backed, smaller-than-Akamai team, single-product-focused.
- **Post-acquisition Akamai** is a public CDN/edge company with a broader portfolio; Spin is now one product among many.
- The contributor distribution (9 of 10 top contributors are Fermyon/Akamai) means Akamai's commitment is load-bearing — if Akamai deprioritizes Spin in 12-24 months, the project's bus factor is structurally exposed. Compare to wasmCloud's multi-vendor CNCF Incubating posture, which is structurally healthier.

The repo rename `fermyon/spin` → `spinframework/spin` happened on 2025-01-21 — *the same day* SpinKube was accepted to CNCF Sandbox — driven by CNCF vendor-neutrality requirements, not the Akamai deal that came 10 months later.

## Framing disclosure

These docs are written from the **Myrhiza-as-deterministic-stateful-runtime** stance — the "Implications for Myrhiza" sub-sections frame Spin's request-driven serverless choices through that lens. Spin's design point is *the opposite* of Myrhiza's `state-apply` purity, but specific patterns (factor architecture, manifest capability declaration, OCI distribution, componentize-* tooling) translate cleanly. A reader should weigh that the corpus is "Spin is a sister CM runtime, not a model-to-copy" — not a comprehensive Spin tutorial.

The corpus also reads through the **WASM Component Model substrate** lens (see [`../wasm-component-model/`](../wasm-component-model/)) — Spin's commitment to WASI Preview 2 + early Preview 3 RCs makes it a leading-edge consumer of substrate work.

## Sources

Per-file `## Sources` sections list URLs cited in that file. Aggregate top-level sources:

- Spin: <https://github.com/spinframework/spin>, <https://spinframework.dev>
- SpinKube: <https://github.com/spinframework/spin-operator>, <https://www.spinkube.dev>
- CNCF Spin/SpinKube: <https://www.cncf.io/projects/spin/>, <https://www.cncf.io/projects/spinkube/>
- Akamai acquisition: <https://www.akamai.com/newsroom/press-release/akamai-announces-acquisition-of-function-as-a-service-company-fermyon>
- Bytecode Alliance: <https://bytecodealliance.org>
- Spin SIPs (Spin Improvement Proposals): <https://github.com/spinframework/spin/tree/main/docs/content/sips>
