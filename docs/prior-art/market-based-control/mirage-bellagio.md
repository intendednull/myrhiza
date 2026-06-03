**Date:** 2026-05-29
**Status:** active
**Subject:** The two adoption post-mortems — Mirage (succeeded: sole path to a scarce binding resource) vs Bellagio (failed: opt-in market vs free best-effort, non-binding resource). The deployment-survival lesson, with the savings-tax/decay finding.

# Mirage vs Bellagio — when a computational market actually survives

This is the **most directly actionable file** in the folder. Mirage and Bellagio were built by
overlapping teams (UCSD / Intel Research / Harvard) on SHARP-style claims, within a couple of
years of each other, and one succeeded while the other did not. Because the variables that
differ are few and named explicitly in the authors' own retrospective, the pair is close to a
controlled experiment in *what makes a computational market deploy*. The conclusions transfer
almost directly to Myrhiza's "what should low standing concretely do" question (the reciprocity
report's [Open fork #6](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md)).

## Mirage — the success (and *why* it succeeded)

**System.** Chun, Buonadonna, AuYoung, Ng, Parkes, Shneidman, Snoeren, Vahdat — "Mirage: A
Microeconomic Resource Allocation System for Sensornet Testbeds," IEEE EmNetS-II, 2005. It
allocated time/space on Intel Research Berkeley's **148-mote** sensornet testbed via a **repeated
combinatorial auction** in a **closed virtual-currency** environment. Users bid for resource
bundles in space/time (e.g. *"any 32 MICA2 motes for 8 hours anytime in the next three days"*)
with a max willingness-to-pay; the auction periodically clears to maximize aggregate stated
value. It ran in daily use for ~4 months.

**Why it succeeded — the one variable that matters.** Mirage was **the sole means of getting
physical access to the testbed.** Verified from the primary: *"Mirage currently serves as the
sole means of getting physical access to testbed resources."* There was no free side door. The
priced resource was genuinely **scarce and binding** — physical motes, with demand exceeding
supply. When the market is the *only* path to a *truly scarce* resource, users have no choice but
to engage, and the market clears real contention. This is the deployment-survival condition.

## Bellagio — the failure (and *why* it failed)

**System.** AuYoung, Chun, Snoeren, Vahdat — "Resource Allocation in Federated Distributed
Computing Infrastructures," OASIS 2004. A combinatorial-auction, virtual-currency, strategy-proof
market intended for **PlanetLab** (a ~440-machine wide-area overlay). Same mechanism family as
Mirage.

**Why it failed — two named differences from Mirage.**
1. **It competed with a free best-effort default.** PlanetLab *already* gave every slice a free
   proportional share of each machine. The Bellagio market was therefore **opt-in**: a user could
   just take the free best-effort allocation and ignore the auction. A market you can route around
   is a market most users route around. The HotOS retrospective frames the general problem
   exactly this way — participants "will have access to a best-effort staging ground" against
   which any market must compete.
2. **It priced a non-binding resource.** On PlanetLab, CPU/bandwidth on most nodes most of the
   time was *not* the binding constraint (the value of PlanetLab was global *distribution* of
   machines, not raw scarce cycles). Pricing a resource that isn't actually scarce gives users no
   reason to pay. Contrast Mirage's physical motes, where contention was real.

The Bellagio OASIS paper is itself a *design/simulation* paper ("we plan to deploy... to gain
experience with real users"); the durable-adoption verdict comes from the authors' later
retrospectives (HotOS 2005; the 2009 Wiley chapter). The honest read across those: the PlanetLab
market did not achieve the engaged, contention-clearing usage Mirage did.

## The shared finding — idle scrip must be taxed/decayed

Both systems discovered that a **closed virtual currency hoards**. Idle or light users
accumulate currency they never spend, which (a) lets them later dominate auctions on a windfall
and (b) drains spendable currency from active users. Mirage's currency policy had **two explicit
components** (verified from the primary): *"(i) proportional-share profit sharing, to allow idle
users to accumulate transient credit and (ii) a savings tax, which implements a 'use it or lose
it' policy"* to bound hoarding. The HotOS retrospective generalizes the hazard list for any
virtual currency: **starvation** (heavy users run out), **depletion** ("as users leave the system
or hoard currency reducing the total amount of currency available to others"), and **inflation**
("as users are added to the system with an initial credit"). The lesson: a scrip economy is not
free — it requires active monetary policy (decay, taxation, rebalancing) just to stay functional.

## What this means for Myrhiza

- **Refusal must be the only path, or it is no path.** Mirage worked because there was no free
  side door. Myrhiza's [capability-mediated kernel](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md)
  makes "refuse to serve" a *first-class* primitive — which is exactly the mechanism needed to
  make low standing *binding* rather than opt-in. Bellagio is the cautionary case: if a freeloader
  can still get served by some default path, standing is advisory and ignored. This is the direct
  input to Open fork #6 (the enforcement side).
- **Price the binding resource.** Standing should bite where the resource is actually scarce *to
  the serving peer* (Mirage's lesson, and exactly why the reciprocity model uses *subjective*
  per-peer shadow prices — scarcity is local). Pricing a non-scarce resource is Bellagio's mistake.
- **The decay design is pre-validated — and Myrhiza already chose its shape.** Both deployments
  needed a "use it or lose it" decay to stop hoarding. The reciprocity report's **consumption-
  relative decay** (standing erodes only as a peer consumes without reciprocating) is a *better-
  targeted* version of the same idea: it taxes the exact behavior — taking without giving — that
  Mirage's flat savings-tax approximated. The deployment evidence here validates that a decay knob
  is mandatory, not optional.
- **A local ledger sidesteps the scrip-hazard list entirely.** Starvation/depletion/inflation are
  pathologies of a *shared closed currency*. Myrhiza's per-peer, non-authoritative ledger has no
  shared currency to deplete or inflate — each peer's standing of others is private bookkeeping,
  not a circulating token. See [`lessons.md`](lessons.md).

## Sources

- [Chun / Buonadonna / AuYoung / Ng / Parkes / Shneidman / Snoeren / Vahdat, "Mirage: A Microeconomic Resource Allocation System for Sensornet Testbeds," IEEE EmNetS-II, May 30–31, 2005](https://cseweb.ucsd.edu/~aauyoung/papers/mirage-emnets05.pdf) — "sole means of getting physical access" and the two-component currency policy ("proportional-share profit sharing" + "savings tax / use it or lose it") verified from primary PDF.
- [AuYoung / Chun / Snoeren / Vahdat, "Resource Allocation in Federated Distributed Computing Infrastructures" (Bellagio), OASIS, Boston, Oct 2004](https://cseweb.ucsd.edu/~aauyoung/papers/bellagio-oasis04.pdf) — combinatorial-auction / virtual-currency / strategy-proof design and PlanetLab free-share context verified from primary PDF. (Seed listed "OASIS / WORLDS 2004"; the primary Bellagio paper is OASIS 2004 — see flag below.)
- [Shneidman / Ng / Parkes / AuYoung / Snoeren / Vahdat / Chun, "Why Markets Could (But Don't Currently) Solve Resource Allocation Problems in Systems," HotOS X, Santa Fe, June 2005](https://www.usenix.org/legacyurl/hotos-x-151-technical-paper-25) — "best-effort staging ground," and the currency starvation/depletion/hoarding/inflation hazards verified from primary PDF (Harvard DASH mirror).
- Retrospective: [Chun & Vahdat et al., "Two Auction-Based Resource Allocation Environments: Design and Experience," ch. 23 in *Market-Oriented Grid and Utility Computing* (Buyya & Bubendorfer, eds.), Wiley, 2009](https://onlinelibrary.wiley.com/doi/10.1002/9780470455432.ch23) — *(paywalled; chapter contents not directly extracted — cited for the Mirage/Bellagio comparison; the substantive findings above are taken from the open HotOS 2005 and primary system papers).*
- Cross-references: [`history.md`](history.md), [`markets-overkill.md`](markets-overkill.md), [`lessons.md`](lessons.md), [reciprocity report Open fork #6](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md).

> **Flag — venue:** The seed cited Bellagio as "OASIS / WORLDS 2004." The primary design paper is **OASIS 2004** (1st Workshop on Operating System and Architectural Support for the on-demand IT InfraStructure, Boston, Oct 2004). A separate WORLDS 2004 workshop existed and adjacent work by these authors appeared around it, but the canonical Bellagio citation is OASIS 2004. Treated as OASIS here; the "WORLDS" half of the seed appears to be a conflation.
