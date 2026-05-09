**Date:** 2026-05-09
**Status:** active
**Subject:** wasmCloud — adopters, commercial layer, CNCF cohort, community metrics

# wasmCloud Ecosystem

This file maps the wasmCloud-adopter and -contributor ecosystem in 2026. Companion files in this folder cover [tooling](./tooling.md), [architecture](./architecture.md), the [capability model](./capability-model.md), [wRPC](./wrpc.md), [interfaces](./interfaces.md), [governance](./governance.md), [history](./history.md), [commercial layer](./commercial.md), [comparisons](./comparisons.md), [critiques](./critiques.md), [open problems](./open-problems.md), and [lessons](./lessons.md). Cross-prior-art neighbours: [WASM CM tooling](../wasm-component-model/tooling.md), [Holochain dev tooling](../holochain/), [Iroh CLI tooling](../iroh/).

## CNCF status

- Project page: [cncf.io/projects/wasmcloud](https://www.cncf.io/projects/wasmcloud/) (200 OK on 2026-05-09).
- Maturity: **Incubating**. Promoted from Sandbox at the **2024-11-08 TOC vote** (announcement 2024-11-12). Category: App Definition & Development.
- Donating organisation: Cosmonic.
- Filtered case-studies query (`?_sft_lf-project=wasmcloud`) on cncf.io returns zero results on 2026-05-09. There are no CNCF-curated wasmCloud case studies. The "case studies" wasmCloud blog posts that exist live on `wasmcloud.com/blog`, not on cncf.io.

## Repository metrics (verified 2026-05-09)

| Repo | Stars | Last push | Status |
|---|---|---|---|
| [wasmCloud/wasmCloud](https://github.com/wasmCloud/wasmCloud) | 2,301 | 2026-05-08 | active monorepo (host + wash + crates) |
| [wasmCloud/wadm](https://github.com/wasmCloud/wadm) | (separate) | 2026-05-06 | active; release cadence slowing |
| [wasmCloud/typescript](https://github.com/wasmCloud/typescript) | 5 | 2026-05-08 | active templates + npm packages |
| [wasmCloud/wasmcloud-otp](https://github.com/wasmCloud/wasmcloud-otp) | n/a | 2026-02-26 | the v1 Elixir host; mostly historical |
| [wasmCloud/wash-archive](https://github.com/wasmCloud/wash-archive) | n/a | 2026-01-08 | read-only; v1 wash is here |
| [wasmCloud/capability-providers](https://github.com/wasmCloud/capability-providers) | n/a | 2025-06-22 | **archived** |
| [wasmCloud/examples](https://github.com/wasmCloud/examples) | n/a | 2026-02-26 | **archived** (moved into monorepo) |
| [wasmCloud/interfaces](https://github.com/wasmCloud/interfaces) | n/a | 2024-11-26 | **archived** (Smithy-era) |

The graveyard is a real signal: four repos archived in the last ~18 months as the project consolidated to the monorepo and moved off Smithy IDL. New contributors arriving in 2026 should expect to ignore most search results.

The 2,301-star count is small for a CNCF Incubating project. Compare: Spin (Fermyon) ~6.4k, Wasmer ~20.7k, Krustlet 3.6k (archived). wasmCloud's adoption is enterprise-channel-driven rather than developer-popular.

## Production deployments

Honest assessment: this is harder to verify than the brief implies. There are no CNCF-published wasmCloud case studies as of 2026-05-09. Public claims from wasmCloud blog posts and KubeCon talks include:

- **Adidas** — referenced in 2024 wasmCloud talks as a user; details are thin and the public posts are vendor-authored. Treat as a logo, not an architecture reference.
- **Akamai** — referenced in early 2024 community calls; no production-scale architecture writeup public.
- **Orange** (telco) — referenced in 2023 KubeCon EU sessions as exploring wasmCloud. Status in 2026 unverified.
- **Bayer / Bosch / German Industrie 4.0** — the brief mentions German Industrie 4.0 partners. There is a real working group (the Industrial Digital Twin Association uses Wasm for shop-floor portability) but specific wasmCloud production deployments are **[unverified]**.

Recommendation: do not cite specific company adoption in Myrhiza specs without first reading a primary source (a conference talk recording or an engineering blog post **from the company itself**, not from Cosmonic or wasmCloud marketing). Most third-party-attributed adoption claims for wasmCloud chase back to Cosmonic-authored content.

## Cosmonic — the commercial steward

Founded 2021 by ex-Capital One and ex-Microsoft engineers, including Liam Randall and Kevin Hoffman (creator of waSCC, wasmCloud's predecessor). Cosmonic is the primary corporate contributor and CNCF donor.

Product line as of 2026-05-09:

- **Cosmonic Control** is the current flagship (per `cosmonic.com` homepage, fetched 2026-05-09). Description on the site: *"Securely run MCP servers in WebAssembly sandboxes with Cosmonic — fast, portable, and Kubernetes-native AI infrastructure anywhere."* This is a **significant pivot from 2024** when the flagship was "Cosmonic Connect / Cosmonic Cloud" (a managed wasmCloud lattice). The 2026 product positions wasmCloud as the secure-sandboxing substrate underneath an MCP-server (Anthropic's Model Context Protocol) deployment platform.
- The brief asks "Cosmonic acquired? Pivoting?" — pivoting, not acquired. The website is live, the team is shipping, but the product narrative changed.
- Cosmonic is a single-vendor commercial layer: there is no second managed-wasmCloud-cloud provider of note.

cosmonic-labs (separate GitHub org) holds:
- `concordance` (62 stars) — opinionated event-sourcing framework on wasmCloud.
- `netreap` (142 stars) — Cilium / k8s networking utility.
- `wrpc` (recently moved) — wRPC reference implementation.
- `wasmpay`, `awesome-cosmonic`, `kubecon2024-eu-wasm-workshop`, etc.

The cosmonic-labs repos overall are smaller and more demo-grade than the wasmCloud-org repos. They are useful examples but not production substrates.

## Other contributors

- **Microsoft Azure team.** Sustained contributions from Azure / Hyperlight engineers around wasmtime integration and capability providers. Kevin Hoffman (wasmCloud's co-creator) is a Microsoft alum; some Microsoft engineers contribute via the WebAssembly working group rather than as a corporate Cosmonic-equivalent. **[unverified]** as a formal corporate contributor relationship in 2026; the contribution flow is via individual maintainers, not an Azure product team.
- **German Industrie 4.0 partners.** As above: there is community engagement, no specific large-scale production deployment publicly verifiable on 2026-05-09. Treat as folklore until cited.
- **Independent contributors.** The wasmCloud monorepo has 241 forks and 55 open issues — small but engaged.

## CNCF cohort: App Definition & Development

Other Wasm-adjacent or competing projects in the same neighbourhood:

| Project | Cohort | Stars | Notes |
|---|---|---|---|
| [Spin (Fermyon)](https://github.com/fermyon/spin) | not CNCF; commercial OSS | 6,407 | Direct competitor for the developer-experience story; smaller scope than wasmCloud (single-binary apps, not lattices). |
| [SpinKube](https://github.com/spinkube/spin-operator) | CNCF Sandbox | 285 | Spin-on-Kubernetes operator; closer in scope to v2 wasmCloud's Kubernetes pivot. |
| [Wasmer](https://github.com/wasmerio/wasmer) | not CNCF | 20,654 | Runtime + Wasmer Edge (commercial). Different layer (runtime, not platform). |
| [Krustlet](https://github.com/krustlet/krustlet) | CNCF Sandbox; **archived** 2023 | 3,603 | Predecessor of all "Wasm on K8s" experiments. Archived after Microsoft pulled funding. |
| [WasmEdge](https://github.com/WasmEdge/WasmEdge) | CNCF Sandbox | n/a (high) | Runtime focused on edge / IoT. |

The "Wasm on Kubernetes" niche is now contested between SpinKube (Spin-on-K8s) and v2 wasmCloud (CRDs + operator). Krustlet's death is the cautionary tale: a project can be CNCF Sandbox, technically interesting, and still die when its single corporate sponsor disengages.

## Conferences and community

- **wasmCon** — Linux Foundation event, runs annually since 2023. wasmcon.io 200 OK 2026-05-09. wasmCloud has a regular keynote / track.
- **WasmIO** — Bytecode Alliance-adjacent European event. wasmio.com is intermittent (HTTP 522 on 2026-05-09 — Cloudflare origin error, not 404; site still exists).
- **KubeCon co-located days.** wasmCloud organises a "Wasm Day" at KubeCon EU and NA most cycles. Workshop materials are typically published in `cosmonic-labs/kubecon2024-eu-wasm-workshop` and successors.
- **wasmCloud Wednesdays.** Weekly community call, agendas + recordings at [wasmcloud.com/community](https://wasmcloud.com/community/) (200 OK 2026-05-09). Latest meeting 2026-05-06 at time of writing.
- **Slack.** [slack.wasmcloud.com](https://slack.wasmcloud.com) redirects to a `join.slack.com` invite. Member count is not exposed via invite metadata; **[unverified]** in 2026, but estimate <2,000 active based on community-call attendance.
- **Discord.** No primary wasmCloud Discord; Cosmonic uses a Discord that mixes commercial and community traffic. Member counts not extractable without authentication.

## Adoption metrics quick-summary

- GitHub stars (main repo): **2,301** (2026-05-09).
- Monorepo last push: **2026-05-08** (active).
- Forks: 241; open issues: 55.
- npm `@wasmcloud/lattice-client-core`: 75 downloads in last 30 days (small).
- crates.io `wash-cli` (legacy): 144,309 lifetime downloads; `wash` (v2): 5,434 lifetime downloads (very early).
- crates.io `wadm`: 74,947 lifetime downloads.

These numbers tell a consistent story: **enterprise-pilot-scale**, not mass-developer adoption. The runtime is technically credible and CNCF-Incubating, but the community is small.

## Books / talks / blog posts of note

- Kevin Hoffman, *Programming WebAssembly with Rust* (Pragmatic, 2019). Pre-dates wasmCloud but introduces the actor / capability model that became wasmCloud's foundation.
- Hoffman & co., wasmCloud KubeCon NA 2022 keynote — establishes the lattice + capability-provider model. Recording on YouTube via CNCF channel.
- wasmCloud blog post series on the v1 → v2 migration (2025–2026) at [wasmcloud.com/blog](https://wasmcloud.com/blog) (200 OK 2026-05-09). Read these for the rationale behind the K8s pivot.
- *"Why we're moving wasmCloud onto Kubernetes"* — wasmCloud blog, late 2025 [paraphrased title; verify before citing].

## Implications for Myrhiza

Patterns worth borrowing:

- **OCI as the bundle registry.** wasmCloud got this right: OCI is well-tooled, mirror-able, and has decent supply-chain primitives (cosign, sigstore). Myrhiza should do the same for app bundles, with mandatory digest pinning where wasmCloud's wadm leaves it optional. See [tooling](./tooling.md).
- **CNCF-style governance documents up front.** wasmCloud has GOVERNANCE.md, MAINTAINERS.md, SECURITY.md, CONTRIBUTION_LADDER.md, RELEASE_RUNBOOK.md, and a quarterly roadmap process visible in the repo. Whatever Myrhiza thinks of CNCF, having those documents in place keeps governance disputes legible.
- **Public community calls with notes.** wasmCloud Wednesdays' agenda + recording archive is a real asset — outsiders can audit decisions without joining Slack.

Anti-patterns to avoid:

- **Single corporate steward.** Cosmonic's pivot to Cosmonic Control / MCP-as-product (2026) is reorienting the wasmCloud roadmap toward Cosmonic's commercial interests. The runtime is healthy because Cosmonic ships it; the runtime is also captured because Cosmonic ships it. Myrhiza should design governance to be sustainable under multiple stewards or none — not bet on one company's product strategy. See [governance](./governance.md).
- **Architectural pivots that re-break the surface.** Two big v0 → v1 → v2 surface breakages in five years. Each one cost adopters their CI scripts and tutorials. Myrhiza should plan its 1.0 surface to be defensible for at least 5 years, and treat host imports as ABI.
- **A central broker as a runtime requirement.** v1 wasmCloud needed NATS. Adopters had to operate NATS clusters they did not want. v2 hides NATS behind Kubernetes, but the dependency is still there. A P2P runtime cannot have a "go install a broker" pre-requisite — the runtime must be the broker.
- **Marketing-attributed adoption as a community KPI.** Vendor case studies that no production engineer at the vendee will confirm in their own blog post are noise. Myrhiza should weight adoption claims by primary-source attribution: a customer's own engineering blog beats a vendor case study by an order of magnitude.

## Verification status

URLs checked with `curl -sLI` on 2026-05-09 (HTTP 200 unless noted):
- wasmcloud.com/community/, wasmcloud.com/docs/, wasmcloud.com/blog (all 200 with Netlify redirects)
- github.com/wasmCloud/wasmCloud, /wadm, /typescript, /wash-archive (200)
- cosmonic.com (200; current title "Cosmonic Control")
- cncf.io/projects/wasmcloud/ (200; "Incubating" confirmed)
- wasmcon.io (200), kubecon redirects to cncf.io/community/kubecon-cloudnativecon-events/
- crates.io HTML routes return 403/404 to bot UAs but the JSON API confirms the crates and versions cited.

Items flagged `[unverified]` are claims propagated through the brief or community folklore that I could not back with a primary source on 2026-05-09. Specs that cite them should re-verify before publication.
