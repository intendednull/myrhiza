**Date:** 2026-05-09
**Status:** active
**Subject:** wasmCloud — Cosmonic, the commercial layer, and the OSS-vs-product boundary

# Commercial layer

A reference snapshot of Cosmonic Inc., the commercial entity behind wasmCloud, and its current product, funding posture, and relationship to the OSS project. Current as of May 2026.

## Cosmonic Inc.

Cosmonic Inc. is the commercial entity associated with wasmCloud. The wasmCloud project itself was originally created by **Liam Randall and Kevin Hoffman** (per the CNCF announcement); Bailey Hayes is a Cosmonic co-founder and currently the wasmCloud project's tech lead, but is *not* a co-founder of the wasmCloud open-source project per the CNCF record. Public verifiable signals about Cosmonic:

- **Cosmonic GitHub org created:** 2021-03-11 (`gh api orgs/Cosmonic --jq '.created_at'`).
- **Cosmonic was a Bytecode Alliance member as of May 2026** ([bytecodealliance.org](https://bytecodealliance.org/) member list).
- **Domain `cosmonic.com` resolves and returns a working website** (HTTP 200, Cloudflare-fronted, verified 2026-05-09).
- **Site title:** *"Cosmonic Control: Secure and efficient platform engineering."*
- **Latest blog post:** 2026-05-06, by CTO Bailey Hayes, on "Sandboxing AIOps and Agentic AI Security."
- **Maintainer affiliation:** 3 of 7 wasmCloud org maintainers list Cosmonic as their employer (see [governance.md](./governance.md)).

Cosmonic is alive, employing engineers, shipping product, and posting at multi-week cadence as of May 2026. There is no acquisition, no public layoff announcement, and no shutdown notice findable on the company site, GitHub, or in CNCF/wasmCloud announcements as of this writing. (Searches: `gh search repos cosmonic --sort updated`, Cosmonic blog index, wasmCloud blog. Negative result: no pivot or layoff posts surfaced.)

## Current product: Cosmonic Control

The flagship and only currently-marketed product is **Cosmonic Control**.

### What it is

A **Kubernetes-native enterprise control plane** for running WebAssembly components, built on top of the OSS wasmCloud project. From the Cosmonic homepage:

> "Cosmonic Control is a Kubernetes control plane that installs into your current clusters using CRDs and an Operator… Control lets WebAssembly components run side-by-side with containers."

### Launch timeline

- **2025-07-07** — Cosmonic Control **Technical Preview** announced. Source: [blog.cosmonic.com/engineering](https://blog.cosmonic.com/engineering), Liam Randall post: *"Announcing the Cosmonic Control Technical Preview."* Quote: *"Cosmonic Control is the enterprise control plane for managing ultra-dense sandboxed platforms with WebAssembly (Wasm). Building on the Incubating CNCF project wasmCloud, Cosmonic Control gives platform engineering teams a single interface and unified control plane to…"*
- **2025-07-07** — Companion post by CTO Bailey Hayes: *"Cosmonic's Approach to Open Source"* — laying out the OSS-vs-commercial boundary (see below).
- **2025-08-06** — Cosmonic Control + Argo CD GitOps post.
- **2026-01-26** — Cosmonic Control on VMware vSphere Kubernetes Service (VKS) integration post.
- **2026-03-13** — KubeCon EU 2026 demo posts.

The product is in **Technical Preview** status as of May 2026 — not GA. Cosmonic is iterating with design partners. There is no public pricing page (`https://cosmonic.com/pricing` returns HTTP 404).

### What it adds over OSS wasmCloud

Per the homepage and the July 2025 launch posts, Cosmonic Control adds, on top of the OSS wasmCloud host:

- A Kubernetes-native control plane with CRDs and an Operator.
- Envoy xDS-compatible ingress/egress management with native cloud integrations.
- Integrated OIDC / SSO for enterprise IAM alignment.
- Capability "controls" — unified policy, observability, and lifecycle management across Wasm workloads.
- AI-agent-specific positioning: secure-by-default sandboxes for MCP servers, AIOps, and agentic workloads (heavy 2025–2026 emphasis).

### What is upstream

Cosmonic's stated approach is **open-source-first**:

- The host runtime (`wasmCloud`), the CLI (`wash`), `wadm`, and the operator (`wasmcloud-operator`) are all upstream Apache-2.0.
- wRPC was upstreamed beyond wasmCloud to the Bytecode Alliance.
- `SandboxMCP` (the MCP server generator) is described on the Cosmonic homepage as: *"a free and open-source plugin built on CNCF wasmCloud."*

### What is Cosmonic-only

Cosmonic Control itself — the enterprise control plane, the Kubernetes Operator integration as Cosmonic ships it, the Envoy xDS gateway, the OIDC/SSO integrations, the unified UI/UX — is the proprietary layer. There is no public source repository for Cosmonic Control; installation goes through `docs.cosmonic.com/install-cosmonic-control`, which gates on a trial.

This is the **"open core" pattern** in its standard form: OSS substrate that can be self-hosted, commercial control plane that adds enterprise features on top. Compare to HashiCorp Terraform / Terraform Cloud, GitLab CE / GitLab EE, Grafana / Grafana Enterprise.

## The "approach to open source" statement

The 2025-07-07 Bailey Hayes post is the closest thing to a public OSS-charter Cosmonic has published. Worth quoting in shape if not verbatim — the operative principles:

- Cosmonic Control "is built on the open source foundations of wasmCloud."
- wasmCloud "originated with the founders of Cosmonic" but is governed independently as a CNCF Incubating project.
- The boundary is the control plane, not the runtime. Anyone can self-host the wasmCloud host without ever touching Cosmonic.

The post is the canonical artifact for the "is this an open-core land grab?" question. The TL;DR answer, as of May 2026: **the runtime is genuinely upstream, governed by CNCF, with diverse maintainer affiliation. The commercial layer sits clearly above the OSS line.**

## Funding and corporate posture

Public funding details are not findable on the Cosmonic site (no "About" investor section, no press-releases page, no pricing page). What is findable:

- **TechCrunch coverage exists:** [Cosmonic launches its WebAssembly PaaS into open beta](https://techcrunch.com/2023/04/17/cosmonic-launches-its-webassembly-paas-into-open-beta/), 2023-04-17. The piece confirms Cosmonic donated wasmCloud to CNCF in 2021 and lists early users including Capital One, Volvo, BMW, and Intel. **It does not disclose a funding round amount in the parts of the article reachable via plain-text scrape.**
- **No acquisition or shutdown announcement** has surfaced. The blog cadence (May 6, 2026 most recent; multiple 2025–2026 posts) is consistent with an operating company, not a wind-down.
- **Pivot signal — but it is a sharpening, not a flip.** Cosmonic's 2023 product was a hosted WebAssembly PaaS ("Cosmonic Connect" / cloud-hosted lattice). That product is no longer marketed. The 2025+ product is "Cosmonic Control," a Kubernetes-native control plane the customer self-installs. **The PaaS-to-control-plane pivot is the most consequential commercial change** since 2023: from "we host Wasm for you" to "we sell you the control plane to host Wasm yourself, in your Kubernetes cluster, on your Wasm components." This tracks the broader 2024–2025 industry shift away from hosted-PaaS toward self-hosted enterprise control planes (Fermyon Spin Hub similarly de-emphasized hosted offerings in this period).

What is **not verifiable from public sources** as of 2026-05-09:

- Specific funding round amounts, dates, or lead investors.
- Headcount or any layoff/RIF events.
- Revenue or customer count.

If the spec audience needs precise funding numbers, those would need to come from Crunchbase / PitchBook (paid sources) or a direct ask to Cosmonic. The public-evidence story is "alive, shipping, pivoted product positioning once."

## Other commercial entities running wasmCloud

Public testimonials in the 2024-11-12 CNCF announcement and the 2024-01-05 CNCF telecom blog name production deployments at:

- **Adobe** — Colin Murphy (`@cdmurph32`) is a wasmCloud org maintainer at Adobe; Adobe + Akamai have published joint demos on running wasmCloud at the edge.
- **Akamai** — Doug Rodrigues collaborates with Adobe on edge-Wasm patterns.
- **Capital One** — Brooks Townsend (`@brooksmtownsend`) is a wasmCloud org maintainer at Capital One; original creators were at "a top 10 US bank" (Capital One per public talks).
- **Orange** — TM Forum WebAssembly Canvas Catalyst project participant.
- **Vodafone, Etisalat by e&, nbnCo** — TM Forum CSPs from the same telecoms PoC.
- **MachineMetrics** — industrial / IoT use case, subject of the 2024-08-23 CNCF blog post.
- **Synadia** — Jordan Rash is an org maintainer; Synadia is the company behind NATS, which wasmCloud uses as its lattice transport.
- **TM Forum partners SigScale, Wavenet, Comviva** — telecom OSS/BSS innovators on the same Catalyst.

This list is broader than "Cosmonic plus a few users," which is what the wasmCloud project survival story rests on.

### The Microsoft Hyperlight question

Microsoft published [Hyperlight Wasm: Fast, secure, and OS-free](https://opensource.microsoft.com/blog/2025/03/26/hyperlight-wasm-fast-secure-and-os-free/) on 2025-03-26. The post mentions wasmCloud in the runtime list:

> "they can run their programs locally using runtimes like wasmtime or Jco. Or run them on a server using for Nginx Unit, Spin, WasmCloud—or now also Hyperlight Wasm."

**This is parallel-runtime positioning, not a Microsoft-runs-wasmCloud collaboration.** Hyperlight is a separate VMM project (now hosted at `hyperlight-dev/hyperlight` as a CNCF Sandbox project, after originating at Microsoft). Hyperlight Wasm is a separate Wasm runtime that consumes the same Component-Model artifacts. wasmCloud and Hyperlight share the *artifact format* (Wasm components) and the *substrate* (Wasmtime, in different embedding modes), not a product collaboration. Spec authors should not cite Microsoft as a wasmCloud production deployment.

## Single-vendor governance risk: what survives a Cosmonic disappearance

If Cosmonic shut down tomorrow:

| Asset | Survives? | Why |
|---|---|---|
| Apache-2.0 source code | Yes | DCO-signed contributions; copyright is "The wasmCloud Authors" |
| `wasmCloud` trademark | Yes | Held by The Linux Foundation per CNCF charter |
| `wasmcloud.com` domain | Yes | Held by LF |
| Governance structure | Yes | CNCF TOC and project's own `GOVERNANCE.md` are independent |
| Org maintainers | Mostly | 4 of 7 are not at Cosmonic; could continue. The 3 Cosmonic maintainers would need new employers or step back. |
| Engineering velocity | **No** | Cosmonic engineers are top contributors by commit count. A Cosmonic exit would slow the project by months at minimum. |
| Cosmonic Control | **No** | Proprietary; would not become OSS by Cosmonic disappearing. Customers would need to migrate to OSS-only `wasmcloud-operator` or a future replacement. |

CNCF Incubation provides **structural protection, not operational protection.** This is the single most important point for spec audiences considering wasmCloud's longevity.

## Implications for Myrhiza

Myrhiza currently has no commercial entity, no foundation, single founder. The Cosmonic case offers calibration:

- **Open-core can coexist with credible OSS governance** if and only if the open line is drawn at the runtime, not the protocol or the format. Cosmonic Control sits *above* the wasmCloud host on the stack; it does not fork the protocol, it does not gate the network, it does not add proprietary capability interfaces. Myrhiza can model a similar division if a commercial entity ever emerges: keep the kernel, the capability ABI, the wire formats, and the consensus rules upstream and Apache-licensed; sell management, observability, hosted operation, or enterprise integrations.
- **The pivot risk is real and the public-information density is low.** The 2023 PaaS-to-2025 control-plane pivot was significant, but it took deliberate reading to find. Funding amounts, headcount, and revenue are not findable. Anyone betting on a wasmCloud deployment because Cosmonic is "well-funded" is making an unverifiable bet. Don't structure Myrhiza's commercial story to rely on opacity working in your favor.
- **Marks and IP-assignment dominate the resilience story.** The CNCF/LF holding of `wasmCloud` is what makes "Cosmonic disappears" a recoverable event. If Myrhiza ever takes a commercial partner, the trademark and domain disposition is the first thing to negotiate, not the last.
- **A "Cosmonic Approach to Open Source"-style public commitment is a cheap and high-value artifact.** A blog post that explicitly draws the OSS-vs-commercial line, signed by the founders, sets enforceable expectations and attracts upstream contributors who would otherwise stay away.

## See also

- [governance.md](./governance.md) — the CNCF/maintainer structure that constrains Cosmonic
- [history.md](./history.md) — chronological context for the PaaS-to-Control pivot
- [architecture.md](./architecture.md) — what Cosmonic Control adds on top of, technically
- [BA governance](../wasm-component-model/governance.md) — Cosmonic's role at the substrate level
- [Holochain governance](../holochain/governance.md) — comparison: a different foundation-plus-commercial split
