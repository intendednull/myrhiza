**Date:** 2026-05-22
**Status:** active
**Subject:** Governance and stewardship of Cap'n Proto / Cap'n Web / Workers RPC — who owns what, funding, bus factor

# Governance

Three projects, three different stewardship models. The pattern is messy by design: Cloudflare-the-company funds the C++ ref impl + Workers RPC + Cap'n Web; individual maintainers steward the Rust + Java bindings; the Sandstorm-community-under-Open-Source-Collective runs Sandstorm. No single foundation or governance board exists across the family.

## Cap'n Proto C++ reference implementation — Cloudflare-stewarded

The canonical statement is in the Cap'n Proto FAQ:

> *"The Cloudflare Workers team are now the primary developers and maintainers of Cap'n Proto's primary C++ implementation."*

In practice this means:
- Kenton Varda's day-job at Cloudflare includes Cap'n Proto upkeep.
- Other Cloudflare Workers engineers contribute as their work requires.
- The release cadence and v2 roadmap are driven by what workerd needs.
- The project is *not* part of Cloudflare's product surface — it remains MIT-licensed, repo lives at `github.com/capnproto/capnproto` (not under `cloudflare/`), and Cloudflare's branding is absent. But the work is Cloudflare-funded in effect.

**Bus factor.** If Varda leaves Cloudflare, the C++ ref impl loses its primary contributor. Cloudflare has other contributors but Varda is the architectural lead. Sandstorm-community could not realistically pick this up; the C++ codebase + KJ toolkit is large enough that a hobbyist re-take is implausible.

**License.** MIT (verified via LICENSE file: copyright 2013-2017 Sandstorm Development Group, Inc. + Cloudflare, Inc. + other contributors). The GitHub API reports `NOASSERTION` because the LICENSE file header doesn't match GitHub's canonical SPDX detection regex, but the text *is* the MIT license verbatim.

## Cap'n Proto Rust — independent + sole-maintainer

David Renshaw (@dwrensha) has maintained `capnproto-rust` since 2013. The crate has shipped ~monthly minors through 2026 across the v0.x line; the latest is `capnp` v0.25.4 (2026-04-12). Repo lives at `github.com/capnproto/capnproto-rust` (org-shared with the C++ ref impl) but the work is independent of Cloudflare.

**Funding model.** None disclosed. Renshaw appears to maintain capnproto-rust as a side project; he is also the maintainer of capnproto-java. There is no GitHub Sponsors page on his profile that funds the work, and no corporate sponsor is named in the README.

**Bus factor.** Single-maintainer for ~13 years. There is no co-maintainer with merge rights. If Renshaw becomes unavailable, the crate will go into community-fork mode; the wider Rust ecosystem (Iroh, others) depends on this crate, so a community successor is likely but not guaranteed. The lesson 5 / 7 / 12 pattern (verify per-repo license + maintainer) is especially important here.

**License.** MIT.

## Cap'n Web — Cloudflare project

Repo at [`github.com/cloudflare/capnweb`](https://github.com/cloudflare/capnweb), under the `cloudflare/` org directly. This is unambiguously Cloudflare-owned; the author block on `capnweb@latest` on npm reads:

```json
{
  "author": {"name": "Kenton Varda", "email": "kenton@cloudflare.com"},
  "_npmUser": {"name": "GitHub Actions", "email": "npm-oidc-no-reply@github.com"}
}
```

The npm publish identity is `GitHub Actions` via OIDC — so the publish pipeline runs in Cloudflare's CI, not from a personal account. This is the production-grade Cloudflare-CI publish pattern.

**Funding model.** Cloudflare-funded directly. This is a product-adjacent project (Wrangler's "remote bindings" feature is the named use case in the launch post).

**Bus factor.** Currently Varda + Steve Faulkner + Cloudflare Workers team. Healthier than capnproto-rust because Cloudflare has resources to keep contributors involved.

**License.** MIT.

## Workers RPC — inside workerd, Apache-2.0

Workers RPC is not a separate project; it's a component of `cloudflare/workerd`. The schema (`worker-interface.capnp`) and implementation (`worker-rpc.c++`) are in the workerd repo under its Apache-2.0 license. Cloudflare maintains workerd as part of their Workers product line.

**Funding model.** Cloudflare core product. workerd is the runtime that runs Cloudflare Workers; Workers RPC is the inter-Worker calling mechanism. Both are operationally critical to Cloudflare's revenue.

**Bus factor.** Cloudflare-the-company. Single corporate steward, but a high-investment one.

**License.** Apache-2.0.

## Sandstorm — community-led under Open Source Collective

Per the [2024-01-14 hand-off post](https://sandstorm.io/news/2024-01-14-move-to-sandstorm-org):

> *"Sandstorm now belongs to the Sandstorm Community under Open Source Collective."*

Governance shape:
- **Fiscal host.** Open Source Collective (a 501(c)(6) US non-profit that fiscally hosts open-source projects on Open Collective).
- **Lead.** Jacob "ocdtrekkie" Weisz, a long-time community contributor who took over coordination.
- **Decision-making.** Informal consensus among the community contributors, similar to most small open-source projects.
- **Funding.** Donations via Open Collective; current named sponsor (per GitHub repo description) is **TestMu AI**.
- **Activity level.** Maintenance mode. Per Varda's own framing: *"I gave up pushing monthly releases, since there seemed to be no point: no code changes had been made and no dependencies could be updated."* Some incremental work continues (last push 2026-05-16) but no major roadmap.

**Bus factor.** Single community lead. The Tempest rewrite (started by Ian Denhardt) stalled after his death in 2023. The original Sandstorm codebase has accumulated technical debt (notably stuck on MongoDB 2.6).

**License.** Apache-2.0.

## Cross-project governance

There is **no umbrella foundation** for the Cap'n Proto / Cap'n Web / Workers RPC family. Each project's stewardship is independent. The closest thing to coordinated governance is:

- The `github.com/capnproto` GitHub org hosts the C++ ref impl, Rust, Go, Java, OCaml, Python, node-capnp implementations.
- Kenton Varda has admin access across these.
- Decisions about wire-format or RPC-protocol changes are de-facto Varda's call (he is the protocol designer); changes affecting downstream bindings get discussed in issues but there is no formal RFC process.

The capnproto-org-as-loose-affiliation pattern works because (a) the wire format has been stable since 0.5 (2014), (b) the RPC protocol hasn't changed in a backwards-incompatible way since promise pipelining shipped in 0.4 (2013), and (c) the C++ ref impl is the de-facto-canonical reference everyone else cross-tests against.

## Trademark + branding

"Cap'n Proto" is not a trademark of any single entity. The name and logo are used informally. Cloudflare does not assert ownership of the brand even though they fund the C++ ref impl.

"Cap'n Web" is in the `cloudflare/` GitHub org and is more clearly a Cloudflare product, though MIT-licensed.

"Sandstorm" is the project name owned by the Sandstorm Community via Open Source Collective; Sandstorm Development Group, Inc. (the former for-profit) is dissolved.

## Roadmaps + RFC processes

None of these projects has a formal RFC / governance-document process:

- **Cap'n Proto C++** — roadmap announced in blog posts (1.0 LTS announcement is the canonical example), discussed in GitHub issues, decided informally by Varda.
- **Cap'n Proto Rust** — bumps follow Varda's protocol decisions; Renshaw makes Rust-API decisions.
- **Cap'n Web** — pre-1.0; roadmap unstated publicly beyond "more transports, better TS types, eventually 1.0."
- **Workers RPC** — roadmap is Cloudflare-internal; public visibility is via Workers product announcements.
- **Sandstorm** — community-decided, lowest-velocity.

This compares unfavorably with gRPC (CNCF-governed, formal RFC process, multi-vendor neutrality) and OCapN (working group with explicit pre-spec consensus model). The honest read: Cap'n Proto governance has worked because the protocol has been stable; if it needed to evolve quickly, the governance lightness could become a liability.

## Implications for Myrhiza

- **Pin per-crate, not per-family.** capnproto-rust v0.25.x is the version we'd integrate; pin it, plan to bump quarterly, treat as if upstream might go quiet at any time.
- **The corporate-steward model works, but plan the exit.** Cloudflare-funds-Cap'n-Proto is healthy *now*. If Cloudflare's strategy shifts (acquisition, layoffs, product reshuffling), the C++ ref impl could enter maintenance mode within months. Myrhiza should not depend on protocol-level evolution from upstream; if Myrhiza needs CapTP Level 2 or Level 3, plan to implement it ourselves.
- **No CNCF / foundation backstop is real.** Compare gRPC (CNCF graduated) or NATS (CNCF incubating) to Cap'n Proto (no foundation). The risk profile is different; communicate this in any Myrhiza spec that depends on the Cap'n Proto family.
- **The license heterogeneity is workable.** MIT (Cap'n Proto, Cap'n Web, capnproto-rust) + Apache-2.0 (workerd, Sandstorm) is a permissive-mix that does not block downstream use. Myrhiza picks one and applies consistently; either is fine.
- **Don't expect roadmap visibility.** Plan against the *current* feature set, not promises. Cap'n Proto v2's full ship date is not pinned; Cap'n Web's 1.0 milestone is undated.

## Sources

- [Cap'n Proto FAQ](https://capnproto.org/faq.html) — *"Cloudflare Workers team are now the primary developers and maintainers"*
- [sandstorm.io/news/2024-01-14-move-to-sandstorm-org](https://sandstorm.io/news/2024-01-14-move-to-sandstorm-org) — Sandstorm community handoff
- [sandstorm.io/news/2017-03-13-joining-cloudflare](https://sandstorm.io/news/2017-03-13-joining-cloudflare) — acqui-hire
- [github.com/cloudflare/capnweb](https://github.com/cloudflare/capnweb)
- [github.com/cloudflare/workerd](https://github.com/cloudflare/workerd)
- [github.com/capnproto/capnproto-rust](https://github.com/capnproto/capnproto-rust) — sole-maintainer Rust repo
- [Open Source Collective](https://www.oscollective.org/) — Sandstorm's fiscal host
- [Cap'n Proto 1.0 LTS announcement (2023-07-28)](https://capnproto.org/news/) — roadmap-in-blog-post pattern
