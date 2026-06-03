**Date:** 2026-05-29
**Status:** active
**Subject:** What the market-based-control paradigm structurally never solved — price volatility/transients, discovery latency, bid-authoring UX, and the Coasean firm-vs-market overhead boundary the founders themselves conceded.

# Open problems — what 50 years of computational markets never fixed

These are not bugs in particular systems; they are *structural* properties of running a live
market for fine-grained computational resources. Every system in this folder exhibits some
subset. The reciprocity brainstorm's escape — local crediting off the determinism path, not a
live auction — is engineered to dodge #1–#3 and to accept #4 as a design boundary rather than
fight it. Each entry: problem + why it matters for Myrhiza + canonical source.

## 1. Price volatility and transients

A market's prices are an emergent feedback signal; they oscillate before (if ever) settling, and
they spike under bursty load. Spawn's own evaluation foregrounds *the dynamics of transients* and
*price equilibria* as core difficulties. Volatile prices make value non-stationary: the "cost" of
the same action swings with unrelated market activity, so a credit earned at one moment is worth
something different at the next.

**Why it matters for Myrhiza.** A converged, deterministic state cannot depend on a volatile
real-time price — this is *why* the [reciprocity report](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md)
puts the ledger in a per-peer, non-authoritative behavior component, off the determinism path.
Volatility is fatal to a live market but harmless to private bookkeeping that no one else must
agree with. **Source:** Spawn (IEEE TSE 1992); [`spawn-tycoon.md`](spawn-tycoon.md).

## 2. Price-discovery latency

To know what to bid, an agent must discover the current price; to set a price, a seller must
observe demand. This round-trip is *latency*, and for fine-grained actions the discovery overhead
can dwarf the action being priced. Tycoon's entire pitch was minimizing this ("acquisition
latency limited only by communication delays," "no manual bidding overhead") — which it achieved
only by *dropping the auction* for proportional share. The latency problem is what pushes every
practical market toward thinner mechanisms.

**Why it matters for Myrhiza.** Pricing every inter-component action with a discovery round-trip
is precisely the overhead the paradigm could never amortize at fine grain. Local crediting needs
no discovery: a peer credits work by *its own* measured/replacement cost, immediately, with no
negotiation. **Source:** Tycoon (HP Labs, ~2004); [`spawn-tycoon.md`](spawn-tycoon.md).

## 3. Bid-authoring / valuation UX

Markets demand that users *express valuations* — and humans are bad at it. The HotOS 2005
retrospective is blunt: the bidding interface "is the most public face of a market mechanism" and
the one with "the greatest effect on user perception (and acceptance)," yet it received "almost no
attention." It paints the worst case: *"imagine a market interface that asked the user for their
valuation, one question at a time, over the entire space of good combinations"* — a "painful,"
"time-consuming and sometimes difficult task." Combinatorial bids (Mirage/Bellagio) make this
worse, not better. Users can't easily say what a resource bundle is worth to them.

**Why it matters for Myrhiza.** A reciprocity model that required users to *author bids* would
inherit the adoption-killing UX. The model avoids it entirely: value is computed by the *module's*
declared resource recipe times the *peer's own* shadow prices — **no human bids.** The valuation
burden that sank Bellagio simply does not arise. **Source:** Shneidman et al., HotOS 2005;
[`mirage-bellagio.md`](mirage-bellagio.md).

## 4. The Coasean firm-vs-market boundary (conceded by the founders)

The deepest unsolved question is *where to draw the boundary* between market coordination (price
everything) and non-market coordination (just share / centrally direct). Markets carry
transaction overhead — accounting, negotiation, trust establishment — and below some granularity
that overhead exceeds any allocative benefit. **Miller & Drexler conceded this in the founding
1988 paper:** citing Coase, they note *"market transactions typically incur higher overhead costs
than do transactions inside firms,"* that *"for small enough objects and transactions, the cost of
accounting and negotiations will overwhelm any advantages,"* and that *"computational markets will
consist of islands of central direction in a sea of trade."* The paradigm's own authors never
told you *where* the islands' coastlines are — that boundary is left to the system designer, and
no general answer exists.

**Why it matters for Myrhiza.** This is the reciprocity report's model-challenge #6 (granularity).
The mitigation it adopts is exactly the Coasean move: **flat fair-share *within* a trust domain;
`value_P` pricing only *across* trust boundaries.** Bitswap is cited as precedent — it *stripped*
its byte ledger because per-block accounting wasn't worth the overhead inside a cooperative swarm.
Myrhiza must pick its own coastline deliberately and document it; the paradigm offers the
vocabulary, not the answer. **Source:** Miller & Drexler 1988; [`history.md`](history.md),
[`markets-overkill.md`](markets-overkill.md).

## 5. Currency monetary-policy burden (the scrip never runs itself)

A shared closed virtual currency is not a free coordination device — it needs ongoing monetary
policy or it breaks. The HotOS retrospective enumerates the failure modes: **starvation** (heavy
users run out), **depletion** (users leave or hoard, draining circulating currency), **inflation**
(new users minted with initial credit). Both Mirage and Bellagio needed a savings-tax / decay to
function. "A well-defined currency is a major stumbling block to market adoption in systems."

**Why it matters for Myrhiza.** This is a *shared-currency* pathology. Myrhiza's per-peer ledger
has **no shared circulating token** — standing is private bilateral bookkeeping, so there is
nothing global to inflate, deplete, or starve. The one piece Myrhiza *does* keep — decay — it
targets better than the deployments did (consumption-relative, not a flat wall-clock tax). This is
the locked no-token decision validated by the failure evidence. **Source:** HotOS 2005;
[`mirage-bellagio.md`](mirage-bellagio.md).

## 6. Adoption requires being the binding, sole path (and that's rare)

Across the deployments, the only market that survived (Mirage) was the *sole* path to a *scarce
binding* resource; the one that competed with a free best-effort default (Bellagio) did not catch
on. Markets need genuine, inescapable scarcity to engage users — a condition most systems can't or
won't manufacture. The paradigm never produced a market that thrived *alongside* a free option.

**Why it matters for Myrhiza.** Standing only bites if low standing has a binding consequence and
no free side door. This is Open fork #6 (the enforcement side), and it is where Myrhiza's
capability-mediated kernel — refusal as a first-class primitive — is the distinctive lever. The
paradigm tells Myrhiza *what condition adoption requires*; it does not tell Myrhiza how to meet it
without the kernel. **Source:** Mirage/Bellagio; [`mirage-bellagio.md`](mirage-bellagio.md).

## Cross-references

- [`README.md`](README.md), [`history.md`](history.md), [`lessons.md`](lessons.md)
- [`spawn-tycoon.md`](spawn-tycoon.md), [`mirage-bellagio.md`](mirage-bellagio.md), [`markets-overkill.md`](markets-overkill.md)
- [`prior-art/resource-pricing-theory/`](../resource-pricing-theory/) — the formal fairness counterpart (#1, #4 in optimality terms: DRF / NUM).
- [reciprocity report "model challenges"](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md) — #3 (overkill), #5 (soundness), #6 (granularity) map directly onto #1–#4 here.

## Sources

All sources cited inline above; full bibliographic detail in [`history.md`](history.md), [`spawn-tycoon.md`](spawn-tycoon.md), [`mirage-bellagio.md`](mirage-bellagio.md), and [`markets-overkill.md`](markets-overkill.md). Coasean concession quotes verified from the agoric.com full-text of Miller & Drexler (1988); HotOS currency/UX quotes verified from the primary HotOS 2005 PDF.
