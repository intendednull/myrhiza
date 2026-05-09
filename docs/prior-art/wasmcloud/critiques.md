**Date:** 2026-05-09
**Status:** active
**Subject:** wasmCloud — honest critique: tooling, governance, architecture risks

# Critiques

wasmCloud is the closest existing art to a production CM runtime, and the production-runtime experience cuts both ways. This file documents the load-bearing critiques — what's gone wrong, what's stuck, what's brittle, what Myrhiza must steer around.

Verbatim quotes where findable; paraphrase + `[paraphrased]` otherwise. URLs all visited 2026-05-09.

## 1. The wasmCloud v2.0 pivot: a bigger story than tooling staleness

The dominant fact about wasmCloud as of 2026-05-09 is that **wasmCloud v2.0 dropped non-Kubernetes support**. The standalone CLI (`wash up`/`down`), capability providers, and wadm-as-the-control-plane are all "gone or transformed" in v2. NATS is no longer the lattice; the K8s API server is the scheduling backend.

This reframes the "tooling staleness" reading below: wash 0.43.0 (2026-02-04) and wadm 0.21.1 (2026-01-29) aren't merely slow-moving — they're the **last v1-line releases** before the project pivoted.

Source: [wasmCloud/wasmCloud#5020](https://github.com/wasmCloud/wasmCloud/issues/5020) — *"wasmCloud v2 - no non-K8s support, no RFC?"* (open, 2026-04-03):

> "I came back to build a multi-component prototype, and see if i can contribute anything to the ecosystem, but found the project fundamentally different: `wash up`/`down` removed, capability providers gone, wadm dropped, K8s required. I couldn't find any public RFC or discussion where this was decided. What happened, and is the non-K8s use case dead?" — @mattwilkinsonn

Maintainer response (@LiamRandall, Cosmonic):

> "Yes, there has been a lot of changes over the last year leading up to wasmCloud v2.0 - including an alignment with the Cloud Native infrastructure. K8s API integration…" `[truncated]`

@ricochet (maintainer) clarifying:

> "That first service is actually just the Kubernetes API server, not a full Kubernetes cluster. You don't need k8s to run wasmCloud; the API server is used as a lightweight scheduling backend. You can run it standalone without the rest of the Kubernetes stack."

The wasmCloud docs site itself ([https://wasmcloud.com/docs/](https://wasmcloud.com/docs/)) leads with "wasmCloud v2 is built for Kubernetes."

**Myrhiza implication:** This is the single most important fact. wasmCloud v1 was the design point we'd most want to study — peer-distributed lattice, NATS-as-bus, declarative reconciliation. wasmCloud v2 has narrowed to the cloud-native operator persona. For Myrhiza, the v1 architecture (link definitions, capability providers, wadm reconciliation) is the *prior art*; v2 is a divergent commercial path we don't follow.

## 2. Tooling staleness in the v1 line (now explained)

- `wash` (CLI): max stable **0.43.0**, published **2026-02-04**. Crates.io: `updated_at: 2026-02-04T20:22:34Z`. The standalone wash is no longer the primary tool; v2 uses `kubectl` + new K8s CRDs.
  - Source: `curl -s https://crates.io/api/v1/crates/wash | jq '.crate'` (verified 2026-05-09).
  - The `wasmCloud/wash` repo's main branch *has* moved to a v2.0.0-rc.7 line ([release tag, 2026-02-19](https://github.com/wasmCloud/wash/releases)) but only as a runtime-operator preview. Last commit on `main` is 2026-03-09 (`chore: Update README before v2 move`).
- `wadm`: max stable **0.21.1**, published **2026-01-29**. Repo last commit 2026-04-16 (a typo fix). `pushed_at: 2026-04-16T08:49:48Z`.
- `wadm-client`: max stable **0.10.0**, published **2025-03-19** (>14 months old).

**This is consistent with a project pivoting**, not a project rotting. The v1 tooling went into maintenance mode while v2 was rebuilt around K8s primitives. But for any external observer wanting the v1 model (closer to Myrhiza's interest), the tooling is frozen.

**Myrhiza implication:** Don't expect the v1 wadm/wash codebase to track upstream Wasmtime / WASI changes much further. If we want to mine that code for ideas, do it now.

## 3. Cosmonic-stewardship single-vendor risk

wasmCloud is CNCF Incubating — but the bulk of contributions, releases, and roadmapping have come from Cosmonic and Cosmonic-affiliated maintainers (LiamRandall is Cosmonic CEO; brooksmtownsend, ricochet, vados-cosmonic are all Cosmonic). The CNCF backstop matters in theory; in practice the project moves with Cosmonic.

Indicators (2026-05-09):
- `cosmonic.com` returns HTTP/2 200 — site is up.
- `cosmonic-labs` GitHub org: 47 public repos, last org metadata update **2025-06-05** (~11 months ago). Active project-level commits (e.g., `wasmstreet`, `dark-vessels` pushed 2026-05-05) — work is happening.
- The v2 K8s pivot **is** a commercial bet: enterprise platform teams are the buying persona for Cosmonic Control (Cosmonic's commercial product).

The v2 pivot itself is the loudest signal of vendor coupling: wasmCloud's roadmap is now whatever Cosmonic's enterprise customers want. Issue #5020's response from @LiamRandall is candid:

> "we needed to be laser-focused on which part of the stack we were being ambitious about. There are well-thought-out systems and tool[ing]…" `[truncated, paraphrased — points to focusing on the K8s integration layer rather than re-inventing scheduling]`

I did not find a public Cosmonic layoff announcement. The financial state of Cosmonic is undisclosed.

**Myrhiza implication:** wasmCloud's design choices reflect Cosmonic's commercial constraints, not first-principles distributed-runtime engineering. When borrowing patterns from wasmCloud, separate the *technical* lessons (capability providers, link definitions, wRPC) from the *commercial* lessons (lattice multi-tenancy by operator, K8s integration). We want the former.

## 4. NATS dependency (v1) — direct anti-pattern for Myrhiza

For wasmCloud v1, NATS is a hard requirement. Operating a lattice means operating a NATS cluster. NATS is well-engineered, but:

- **It's a federated message broker, not a peer-symmetric P2P fabric.** NATS clusters have a designated leader for each subject's metadata; clients connect to nominated NATS endpoints. There's no equivalent of gossipsub in vanilla NATS.
- **The operational model assumes the operator owns the broker.** Multi-tenant lattices are isolated by lattice-ID prefixes on subject names; the broker sees everyone.
- **Failure modes are broker-shaped.** When NATS partitions, the lattice partitions; recovery is whatever NATS gives you.

**Myrhiza implication:** This is *the* anti-pattern wasmCloud v1 demonstrates for our use case. Peer-symmetric P2P is the *opposite* design point: no broker, no operator, no central observer. We borrow wasmCloud's *interface-typed addressing* (call `wasi:keyvalue/store.get(k)`, route to whichever provider impls it) but the routing must happen over a peer-symmetric overlay (Iroh + gossipsub), not over NATS.

## 5. Component Model migration friction (the actors → components rewrite)

wasmCloud's transition from "actors" (custom ABI, pre-CM) to "components" (CM/WIT-typed) was a multi-quarter project that broke compatibility. RFCs document the path:

- [#1389 RFC: Componentizing Links with wRPC](https://github.com/wasmCloud/wasmCloud/issues/1389) (closed) — the core proposal.
- [#1548 wRPC MVP Integration](https://github.com/wasmCloud/wasmCloud/issues/1548) (closed) — the implementation issue.
- [#4642 Transition the capability provider model into support for wRPC servers](https://github.com/wasmCloud/wasmCloud/issues/4642) (closed) — provider rewrite.

Some users abandoned the project mid-migration; #5020 itself is testimony from a returning v1 contributor.

**Myrhiza implication:** Don't ship a non-CM ABI. Adopt the Component Model from day one — wasmCloud paid the rewrite cost so we don't have to. The post-CM ABI is the only one worth having.

## 6. Documentation lag (v2-shaped)

The wasmCloud docs site has been substantially rewritten for v2; the v1 documentation is partly archived. The friction in #5020 — "I couldn't find any public RFC or discussion where this was decided" — is in part a docs-lag complaint: the architectural pivot landed in code before the rationale landed in writing.

I did not find a long-running "docs are out of date" issue thread in the wasmCloud repo. The complaint is more diffuse: maintainers ship code; the docs site catches up later. This is normal for a fast-moving runtime, but it does mean external evaluators (us) work from the docs at a lag.

**Myrhiza implication:** When something architectural lands in Myrhiza, the spec lands at the same time. `docs/specs/` is mandatory, not aspirational. The wasmCloud v2 pivot demonstrates the cost of the inverse posture.

## 7. Resource consumption

A wasmCloud host is a heavy process: Wasmtime + lattice control plane + NATS client (v1) or K8s controller (v2) + provider runtime. The footprint is at least an order of magnitude larger than `spin up` on a developer laptop.

I did not find a published head-to-head benchmark of wasmCloud vs Spin RAM/CPU at idle. The closest is the open issue [#5052 \[Performance\] Benchmarking Suite for wasmCloud v2](https://github.com/wasmCloud/wasmCloud/issues/5052) (2026-04-15) — *the project is acknowledging that a canonical benchmarking story does not yet exist*. Quote `[paraphrased]`:

> "Add a first-class benchmarking harness for wasmCloud v2 that measures workload and cluster performance under configurable load, using k6 as the load generator…"

Related: [#4940 \[FEAT\] Component cache](https://github.com/wasmCloud/wasmCloud/issues/4940) (2026-04-08) — open, acknowledging that "components that use datafusion take a long time to compile, and the compiled component itself consumes a lot of memory when running. If you deploy multiple workloads that use this component, each instance loads its own copy into memory, which leads to high cumulative RAM usage."

**Myrhiza implication:** A peer-side runtime in Myrhiza must be much lighter than wasmCloud — the kernel runs on user devices, not on a fleet of beefy hosts. Component-cache / instance-deduplication is a load-bearing optimization, not a v2 nice-to-have. wasmCloud's open issue is a precedent for how to think about it.

## 8. CNCF Incubation tail

wasmCloud entered CNCF Sandbox on **2021-07-13** and was promoted to **Incubating on 2024-11-08** — a 3-year-4-month Sandbox tail. As of 2026-05-09, [https://www.cncf.io/projects/wasmcloud/](https://www.cncf.io/projects/wasmcloud/) lists it as Incubating; it has not graduated.

CNCF's Graduation criteria (committers from multiple orgs, security audit, end-user adoption testimony, etc.) are stringent; several projects sit at Incubating for 3+ years (NATS itself was Incubating from 2018 through current graduation review — see [cncf/toc#2042](https://github.com/cncf/toc/issues/2042)). wasmCloud reached Incubating less than two years ago at the time of writing; a multi-year Incubation tail is plausible. Graduation would depend substantially on demonstrated committer diversity beyond Cosmonic.

**Myrhiza implication:** Don't read "CNCF Incubating" as a stability guarantee. It's a useful credibility signal; it's not insurance against vendor pivot, as the v2 K8s realignment proves. Myrhiza needs no equivalent governance backstop because we're a runtime, not a platform — but if we ever ship multi-tenant infrastructure, the CNCF tail is a cautionary timeline.

## 9. The "production ready?" thread

Closed but illustrative: [wasmCloud/wasmCloud#270](https://github.com/wasmCloud/wasmCloud/discussions/270) — *Production ready?* (2023; answered chosen 2023-01-31). The maintainer answer is "yes, with caveats" — appropriate for a project that was then ~3 years old and has now had two more major version cycles. The point isn't the answer; the point is that production-readiness has been a recurring user concern that the project has had to repeatedly answer.

**Myrhiza implication:** "Production-ready" is a moving target tied to the user's deployment model. wasmCloud's answer changed when the deployment model changed (lattice → K8s). Myrhiza's answer will change as our profiles (state-apply, interaction, behavior) mature at different rates. Be explicit per-profile about readiness state in the spec.

## 10. Single-implementation risk

wasmCloud has one implementation: the Rust host in `wasmCloud/wasmCloud`. There is no spec independent of the code — the WIT packages (`wasmcloud:*`) are the closest thing, and they're versioned ad-hoc. wRPC is a wire protocol, but there's no published wire spec separate from the implementation.

This isn't a critique of the project so much as a structural feature: production runtimes ship one implementation. But for Myrhiza, where peers across the network must interop **without** trusting the same vendor's binary, single-implementation is a cliff.

**Myrhiza implication:** Myrhiza's wire-format work has to be specs-first, not implementation-first. Component-to-kernel ABI, kernel-to-kernel gossip, capability handshake — all need writable specs that another implementation could conform to. wasmCloud's lack of this is acceptable for them; it's not for us.

## See also

- [`architecture.md`](architecture.md) — what got rebuilt in v2 and why.
- [`tooling.md`](tooling.md) — wash / wadm / cosmo / washboard inventory and current state.
- [`governance.md`](governance.md) — CNCF status, Cosmonic stewardship, charter.
- [`history.md`](history.md) — actor → component migration timeline; v1 → v2 pivot.
- [`commercial.md`](commercial.md) — Cosmonic Control vs OSS wasmCloud.
- [`open-problems.md`](open-problems.md) — what's still unsolved at the runtime layer.
- [`lessons.md`](lessons.md) — what to copy and what to avoid.
