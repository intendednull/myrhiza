**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — governance, funding, and stewardship risk

# Governance & funding

Who runs Iroh, how decisions get made, what licensing applies, and — most importantly for Myrhiza — what happens if the steward company stops existing. Current as of May 2026.

## The single-entity structure

Iroh is owned and developed by **Number 0** (also stylized **n0**, "number zero"), described on its own site as *"an open R&D organization focused on efficient distributed systems"* with a team carrying *"combined 70+ years in networked, edge, and cloud distributed systems"* ([n0.computer](https://n0.computer/)).

Unlike Holochain (Foundation + for-profit + subsidiary) or Spritely (Institute as 501(c)(3)), iroh has **no foundation, no nonprofit backstop, no separate IP-holding entity** as of May 2026. The intellectual property, the brand, the canonical implementation, and the operated infrastructure (the default relay servers, the DNS discovery service) all sit inside one venture-backed Delaware-shaped company.

The founder is **Brendan O'Brien** (`b5` on GitHub), an ex-Protocol-Labs / IPFS engineer ([profile](https://agentictech.substack.com/p/the-network-revolutionary-how-brendan)). Other core team members visible from commits and blog bylines include **Friedel Ziegelmayer** (`dignifiedquire`), **rüdiger klaehn** (`rklaehn`), and several others; full headcount is not publicly disclosed but is reportedly low double digits.

## Funding

The team's own framing in the [FAQ](https://docs.iroh.computer/about/faq) is verbatim:

> *"The company behind iroh is number 0. It is partly venture capital and partly founder backed (as in: founders have invested their own money)… number 0 is healthy and has investors we actually think are a value-add."*

Specific funding amounts, round sizes, and investor names are **not publicly disclosed** as of May 2026 — there is no Crunchbase entry, no TechCrunch announcement, no SEC Form D in the public corpus we can find. This is a gap a Myrhiza spec author should treat as a real unknown: we know venture money is involved, but not how much runway it buys.

The commercial monetization story went public in **October 2025** with [**Iroh Services**](https://www.iroh.computer/blog/iroh-0-93-iroh-online) — a managed-relay + DNS-discovery hosted offering, with sign-up at [services.iroh.computer](https://services.iroh.computer). Pricing tiers exist (free public infra for dev, dedicated/private relays for production) but specific dollar amounts are behind the sign-up wall.

## License

Iroh is **dual-licensed Apache-2.0 / MIT** (recipient's choice), the standard Rust-ecosystem dual license ([iroh repo LICENSE](https://github.com/n0-computer/iroh)). Verbatim from the contribution policy:

> *"Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions."*

This is **inbound=outbound**: contributing means you've licensed your contribution under the same terms. **No CLA**, **no DCO sign-off** is enforced as of this writing — n0 took the lighter-weight stance that copyright assignment is not a precondition. For a Myrhiza spec author this is the most permissive realistic license stance and the easiest to live with downstream.

## Patent and IP posture

The Apache-2.0 / MIT dual gives downstream users **Apache-2.0's express patent grant** (§3) when they receive the work — Apache-2.0's grant covers any patents the contributor "necessarily infringes," with reciprocal termination if the user sues for patent infringement. The MIT half offers no patent grant, but downstream users get the dual choice, so Apache's grant is the floor.

Adjacent IP risks worth flagging for a load-bearing-dep commitment:

- **QUIC patents.** QUIC implementations (including iroh's `noq` fork of Quinn) inherit the same patent landscape as any QUIC stack — the IETF QUIC working group's IPR disclosures ([datatracker.ietf.org/wg/quic/documents/](https://datatracker.ietf.org/wg/quic/documents/)) are the canonical source. No iroh-specific patent claims are publicly tracked as of May 2026.
- **BLAKE3.** Public domain (CC0); patent-unencumbered ([BLAKE3 license](https://github.com/BLAKE3-team/BLAKE3?tab=License-1-ov-file)). No risk.
- **The `noq` fork.** Inherits Quinn's MIT-or-Apache-2.0 license; no separate patent grant beyond Apache-2.0's. The fork's divergence from upstream Quinn does not introduce new IP exposure visible from outside the codebase, but a careful audit before 1.0 commitment is reasonable.
- **The relay protocol.** DERP-derived (Tailscale's design); Tailscale ships DERP under MIT and BSD-3-Clause and has not asserted patents against alternative implementations. Iroh's revision is permissively licensed in the same vein.

For a load-bearing dependency this is a tolerable picture — Apache-2.0 patent grant on the canonical implementation, public-domain on the hash function, MIT-equivalent on the transport-layer fork — but a Myrhiza spec author committing to a specific iroh subsystem (e.g. `iroh-blobs` for app-bundle distribution) should re-confirm patent posture against that subsystem's contributors at commitment time, not rely on this snapshot.

## Contribution model

The repos are open: PRs from outside n0 are accepted regularly (visible in the [iroh PR history](https://github.com/n0-computer/iroh/pulls?q=is%3Apr)). However, the strategic direction, the major refactors, and the breaking changes are set by n0 employees. There is no public RFC process, no IETF-style working group, no community vote. The pattern is:

1. **n0 publishes a blog post** announcing direction (the [pivot post](https://www.iroh.computer/blog/a-new-direction-for-iroh), the [1.0 roadmap](https://www.iroh.computer/blog/road-to-1-0), the [Quinn fork](https://www.iroh.computer/blog/why-we-forked-quinn), the [crate spinout](https://www.iroh.computer/blog/iroh-0-28-let-them-have-crates)).
2. **GitHub issues / discussions** capture community feedback after the fact.
3. **Subsequent releases** ship the change.

This is "benevolent dictator" governance, executed openly. It scales to a small team and ships fast; it does not scale to a community that wants veto power. As of May 2026 the community has not pushed back hard enough on any decision to test what disagreement looks like.

## Stewardship risk — the central question

What happens to iroh if Number 0 the company fails?

**The codebase survives.** Apache-2.0 / MIT means anyone can fork, redistribute, and continue development. There is no copyright-assignment trapdoor. The sibling crates (`iroh-blobs`, `iroh-docs`, `iroh-gossip`, `iroh-willow`, `noq`) are also Apache-2.0. The Quinn fork that became `noq` is permissively licensed.

**The infrastructure does not.** The four default relay servers are operated by n0 (US×2, Europe×1, Asia×1 per the [FAQ](https://docs.iroh.computer/about/faq)). The DNS discovery service (`dns.iroh.link`) is operated by n0. If n0 goes dark, every iroh node using default config loses NAT-traversal fallback and global node discovery overnight. **This is a single point of operational failure for the entire default deployment.**

**The brand and the spec do not.** There is no foundation that owns the "Iroh" trademark or the protocol specification. A community fork would have to relaunch under a different name, and the wire format itself is not yet a published RFC-style spec — the [1.0 roadmap](https://www.iroh.computer/blog/road-to-1-0) commits to *"publishing open standards specifications"* before 1.0 finalizes, but as of May 2026 (1.0.0-rc.0) the spec document has not yet shipped.

**There is no foundation backstop.** Compare:

| Project | Backstop if company fails |
|---|---|
| Iroh | None — venture-backed company is sole steward |
| Holochain | Foundation (501-style) owns IP; Holo Ltd. is a subsidiary |
| Spritely | Spritely Institute (501(c)(3)) is the steward |
| libp2p | Protocol Labs (LLC) + active multi-org maintainer set |
| Quinn (the QUIC iroh forked) | Multi-maintainer, no single corporate steward |

**Honest assessment.** This is a real risk, not a theoretical one. The mitigations are: (a) the code is permissively licensed and forkable; (b) Holochain has already committed to iroh as default transport, raising the probability of an external "rescue fork" if needed; (c) the team has been transparent about the funding reality. The aggravators are: (a) no public funding numbers means we don't know runway; (b) the relay infrastructure is *operationally* concentrated in n0 even when the *code* would survive; (c) no IP-holding nonprofit means no obvious receiver of the assets if the company winds down.

## Implications for Myrhiza

**Acceptable risk profile, with explicit mitigations.** If Myrhiza commits to iroh as a load-bearing dependency:

1. **Pin a known-good version and self-mirror sources.** Apache-2.0 lets us. The cost is one cron job and one S3 bucket.
2. **Plan for self-operated relays from day one.** Treat the n0 default relays as a *development convenience*, not a production substrate. Document the relay-self-hosting requirement in the Myrhiza ops spec.
3. **Track a Tier 0 list of iroh forks / alternatives.** If n0 disappears, who continues the work? Holochain's Kitsune2 team is one obvious candidate (they ship iroh as transport). Quinn upstream is the fallback for the QUIC layer.
4. **Wait for the protocol spec.** The team has committed to publishing one before 1.0 finalizes. A published wire-format spec converts iroh from "library with one impl" to "protocol with reference impl," which is a much smaller stewardship-risk profile. Re-evaluate this section when that spec ships.
5. **Don't depend on the Iroh trademark or brand identity in Myrhiza-facing materials.** Depend on the protocol shape and the wire format. The brand is the volatile asset.

The risk is real but tolerable for a load-bearing dep, *provided* Myrhiza takes the relay-hosting and version-pinning hygiene steps above. Eyes open.

## Sources

- [n0.computer — company site](https://n0.computer/)
- [Iroh FAQ — funding, relays, libp2p relationship](https://docs.iroh.computer/about/faq)
- [Iroh 1.0 Roadmap (Oct 28, 2024)](https://www.iroh.computer/blog/road-to-1-0)
- [iroh services launch (Oct 9, 2025)](https://www.iroh.computer/blog/iroh-0-93-iroh-online)
- [GitHub — n0-computer/iroh (license, contribution policy)](https://github.com/n0-computer/iroh)
- [GitHub — n0-computer organization](https://github.com/n0-computer)
- [Comparing iroh & libp2p (Jan 5, 2024)](https://www.iroh.computer/blog/comparing-iroh-and-libp2p)
- [Why we forked Quinn (May 20, 2024)](https://www.iroh.computer/blog/why-we-forked-quinn)
- [Let them have crates (Nov 6, 2024)](https://www.iroh.computer/blog/iroh-0-28-let-them-have-crates)
- [Agentic Tech — The Network Revolutionary (Brendan O'Brien profile)](https://agentictech.substack.com/p/the-network-revolutionary-how-brendan)
- [GitHub — iroh discussion #1277 (relation to rust-libp2p)](https://github.com/n0-computer/iroh/discussions/1277)
