**Date:** 2026-05-09
**Status:** active
**Subject:** Spin — project lineage, the Akamai acquisition, and post-acquisition stewardship

# Governance & stewardship

A reference snapshot of who runs Spin, how it landed at Akamai, and what the open-source posture looks like after 2025-12-01. Current as of May 2026. See also [`glossary.md`](./glossary.md), [`architecture.md`](./architecture.md), [`spinkube.md`](./spinkube.md), [`comparisons.md`](./comparisons.md).

## Project lineage

Spin started inside Fermyon Technologies and has stayed there throughout its working life. The repo (`fermyon/spin`, now `spinframework/spin`) was first pushed on 2021-11-02 — the same week Fermyon came out of stealth. Spin 1.0 shipped 2023-03-21; Spin 2.0 introduced WASI Preview 2 / Component Model support; Spin 4.0.0 (2026-04-20) is the current release line.

| Phase | What happened | When |
|---|---|---|
| Founding | Fermyon Technologies founded by Matt Butcher, Radu Matei, and others (Microsoft DeisLabs alumni) | 2021 |
| Repo creation | First commit to `fermyon/spin` | 2021-11-02 |
| Seed round | $6M | 2022 (Q2) |
| Series A | $20M led by Insight Partners (closed Q3 2022, announced 2022-10-24) | 2022-10-24 |
| Spin 1.0 | First stable release | 2023-03-21 |
| Fermyon Cloud GA | Commercial managed service | 2023 |
| SpinKube announced | Joint contribution with Fermyon, Microsoft, Liquid Reply, SUSE | 2024-03-21 |
| `spinframework` GH org | Created (vendor-neutral home for repo migration) | 2025-01-21 |
| SpinKube → CNCF Sandbox | Accepted | 2025-01-21 |
| Spin → CNCF Sandbox | Accepted (issue #116 closed `gitvote/passed`) | 2025-01-21 |
| Akamai acquires Fermyon | Closed; co-founders join Akamai's Cloud Technology Group | 2025-12-01 |

## Pre-acquisition Fermyon

Fermyon raised roughly $26M total before the Akamai deal: a $6M seed round (Q2 2022) plus the $20M Series A (Q3 2022) led by [Insight Partners](https://www.insightpartners.com/ideas/behind-the-investment-fermyon/), with participation from Amplify Partners and angels including Armon Dadgar (HashiCorp), Daniel Lopez Ridruejo (Bitnami), and Lachlan Evenson (Microsoft Azure). No Series B was ever announced.

Public-facing leadership across the run: **Matt Butcher** (CEO), **Radu Matei** (CTO), **Michelle Dhanani** (engineering / community). The technical core also drew heavily on engineers with deep WebAssembly history — several Spin maintainers were previously on Microsoft DeisLabs, where Butcher and Matei worked before founding Fermyon.

## The Akamai acquisition (2025-12-01)

Akamai announced the acquisition on 2025-12-01 ([press release](https://www.akamai.com/newsroom/press-release/akamai-announces-acquisition-of-function-as-a-service-company-fermyon)). What was actually disclosed:

- **Price**: undisclosed. Akamai stated "no material impact" to its 2025 financial guidance, which sets an implicit upper bound but is not a number.
- **People**: Matt Butcher and Radu Matei joined Akamai's Cloud Technology Group; the rest of the Fermyon team came along.
- **Open-source commitments**: Akamai committed to continued maintenance of Spin and SpinKube as CNCF projects and to continued Bytecode Alliance membership.

Quote from Adam Karon, COO and GM of Akamai's Cloud Technology Group:

> "Fermyon's FaaS capabilities, combined with Akamai's cloud, will make it even easier for developers to innovate and execute lightweight code at the edge."

Quote from Jon Alexander, SVP of Products at Akamai (in [Network World coverage](https://www.networkworld.com/article/4099424/akamai-acquires-fermyon-for-edge-computing-as-webassembly-comes-of-age.html)):

> "We didn't pick WebAssembly and Fermyon independent of each other. We believe that Fermyon gives us the foundation to span all the way from the edge out to highly distributed workloads, then also up to very powerful more resource intensive workloads as well."

Akamai and Fermyon had an existing partnership for roughly 12 months before the acquisition (SpinKube on Linode/Akamai Cloud was already a published reference architecture).

## Akamai Functions

Akamai Functions is the productized successor to Fermyon Cloud. The pitch: Spin-shaped serverless functions executed across Akamai's CDN POPs, with sub-millisecond cold starts and "no regions or replicas to configure." The product page ([akamai.com/products/akamai-functions](https://www.akamai.com/products/akamai-functions)) targets the same shape Fermyon Cloud did — HTTP-triggered Wasm components — but with Akamai's globally distributed edge as the substrate. The `fermyon.com` domain redirects to Akamai Functions; Fermyon's blog now redirects to `akamai.com/blog/developers`.

Strategic framing in Akamai's coverage emphasizes **edge AI inference** — running model-adjacent logic close to the user — as the primary commercial bet, not generic FaaS.

## Open-source commitments post-acquisition

Akamai's public commitment, repeated across the press release and Fermyon's transition post:

- **Spin** stays a CNCF Sandbox project.
- **SpinKube** stays a CNCF Sandbox project.
- **Bytecode Alliance** membership continues; Akamai is officially a sponsor of Wasm I/O 2026.
- The `spinframework` GitHub organization stays vendor-neutral (CNCF requirement).

What's *not* promised in writing: maintainer headcount on Spin, roadmap independence from Akamai Functions priorities, or any formal IP separation beyond what CNCF already requires. The "Akamai will continue to support" language is intent, not contract.

## Bytecode Alliance involvement

Spin's working dependency on Wasmtime, the Component Model toolchain, and WASI is total — Spin is, mechanically, a Wasmtime embedder with a triggers/factors layer on top. Fermyon (now Akamai) has been a Bytecode Alliance member throughout, with maintainers contributing to Wasmtime, `wit-bindgen`, `wasm-tools`, and the WASI HTTP / WASI keyvalue / WASI sqlite proposals. Cross-reference [`../wasm-component-model/governance.md`](../wasm-component-model/governance.md) for how the BA itself is run.

## CNCF status: Spin and SpinKube

Both Spin and SpinKube are at CNCF **Sandbox**, both accepted on 2025-01-21:

- SpinKube — [cncf.io/projects/spinkube](https://www.cncf.io/projects/spinkube/), tracking issue [`cncf/sandbox#90`](https://github.com/cncf/sandbox/issues/90).
- Spin — tracking issue [`cncf/sandbox#116`](https://github.com/cncf/sandbox/issues/116) (state `closed`, label `gitvote/passed`, closed 2025-01-21).

Neither has moved to Incubation. By comparison, wasmCloud spent 2021-07 → 2024-11 in Sandbox before incubating, so Spin is roughly on the early side of the typical Sandbox dwell. Akamai stewardship may complicate the Incubation case because CNCF Incubation requires demonstrated multi-vendor adoption and contributor diversity — see [bus factor](#bus-factor--stewardship-reality) below.

## Repo migration: `fermyon/spin` → `spinframework/spin`

The `spinframework` GitHub organization was created on **2025-01-21**, the same day Spin was accepted to CNCF Sandbox. This is not a coincidence — CNCF Sandbox acceptance requires the project to live in a vendor-neutral GitHub org. The migration was therefore **pre-acquisition** by ~11 months and was driven by CNCF onboarding, not the Akamai deal. As of May 2026, `github.com/fermyon/spin` redirects to `github.com/spinframework/spin`; older release artifacts still reference the `fermyon` paths.

## Bus factor & stewardship reality

Top contributors to `spinframework/spin` by commit count (via `gh api repos/spinframework/spin/contributors`):

| Contributor | Commits | Affiliation |
|---|---:|---|
| `itowlson` (Ivan Towlson) | 1005 | Fermyon → Akamai |
| `rylev` (Ryan Levick) | 822 | Fermyon → Akamai |
| `lann` (Lann Martin) | 696 | Fermyon → Akamai |
| `radu-matei` | 316 | Fermyon co-founder → Akamai |
| `vdice` (Vaughn Dice) | 278 | Fermyon → Akamai |
| `fibonacci1729` | 220 | Fermyon → Akamai |
| `rajatjindal` | 167 | external |
| `michelleN` | 147 | Fermyon → Akamai |
| `kate-goldenring` | 133 | Microsoft (SpinKube co-author) |
| `dicej` (Joel Dice) | 110 | Fermyon → Akamai |

Reading: of the top 10 contributors, **9 are Fermyon (now Akamai)** and the 10th is Microsoft (Kate Goldenring, primarily on SpinKube). The bus factor for Spin's runtime core is, in practice, "Akamai's Cloud Technology Group." Compare wasmCloud (Cosmonic-led but with Adobe / Orange / TM Forum producing material PRs) — Spin is *less* multi-vendor at the contributor level despite both being CNCF Sandbox projects.

This matches Spin's history: it was always a Fermyon-shaped project that the Bytecode Alliance and CNCF pulled toward more open governance. The shape did not change with the Akamai deal, only the parent company did.

## Implications for Myrhiza

- **Single-vendor stewardship is the load-bearing fact.** Akamai now controls roadmap, release cadence, and whose PRs land. Treat Spin's design as a reference and Wasmtime/Component-Model as the dependency we'd actually inherit — not Spin itself.
- **Akamai's commercial gravity pulls toward edge-FaaS**, not toward P2P or distributed authority. Future Spin features will likely optimize for Akamai Functions (sub-millisecond cold start, edge-AI inference, HTTP-shaped triggers). Anything Myrhiza needs that doesn't fit that pattern (deterministic state-apply, P2P gossip, capability quoting between peers) will not come for free.
- **CNCF Sandbox is governance, not engineering insurance.** It requires vendor-neutral org and trademark, not contributor diversity. Sandbox does not protect us from Akamai-driven scope choices.
- **The Bytecode Alliance commitments are the real durable piece.** Wasmtime, Component Model, WIT, and WASI keep advancing regardless of what Akamai does with Spin's triggers layer. That's where Myrhiza's hard dependencies should sit.
- **Track Spin → Incubation as the signal.** If Spin progresses to CNCF Incubation in 2026-2027, it means contributor diversity grew post-acquisition. If it stalls, single-vendor stewardship is real and we should reduce design-time gaze on Spin proportionally.

## Sources

- [Akamai press release — "Akamai Technologies Announces Acquisition of Function-as-a-Service Company Fermyon"](https://www.akamai.com/newsroom/press-release/akamai-announces-acquisition-of-function-as-a-service-company-fermyon) (2025-12-01)
- [GlobeNewswire mirror of acquisition press release](https://www.globenewswire.com/news-release/2025/12/01/3196978/0/en/Akamai-Technologies-Announces-Acquisition-of-Function-as-a-Service-Company-Fermyon.html)
- [Network World — "Akamai acquires Fermyon for edge computing as WebAssembly comes of age"](https://www.networkworld.com/article/4099424/akamai-acquires-fermyon-for-edge-computing-as-webassembly-comes-of-age.html) (Alexander, Butcher quotes)
- [Insight Partners — "Behind the Investment: Fermyon"](https://www.insightpartners.com/ideas/behind-the-investment-fermyon/)
- [TechCrunch — "Fermyon raises $20M to build tools for cloud app dev"](https://techcrunch.com/2022/10/24/fermyon-cloud-app-webassembly-20m-funding-series-a/) (2022-10-24)
- [CNCF — SpinKube project page](https://www.cncf.io/projects/spinkube/)
- [`cncf/sandbox#116` — Spin Sandbox application](https://github.com/cncf/sandbox/issues/116) (closed 2025-01-21, `gitvote/passed`)
- [`cncf/sandbox#90` — SpinKube Sandbox application](https://github.com/cncf/sandbox/issues/90)
- [Akamai Functions product page](https://www.akamai.com/products/akamai-functions)
- [Akamai blog — "Build Serverless Functions with Zero Cold Starts: WebAssembly and Spin"](https://www.akamai.com/blog/developers/build-serverless-functions-zero-cold-starts-webassembly-spin)
- [Bytecode Alliance home](https://bytecodealliance.org/)
- `gh api repos/spinframework/spin/contributors` (May 2026)
- `gh api orgs/spinframework` — org `created_at: 2025-01-21T21:17:24Z`
