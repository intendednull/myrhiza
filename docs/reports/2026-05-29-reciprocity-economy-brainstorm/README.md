**Date:** 2026-05-29
**Status:** active
**Subject:** Reciprocity-based participation economy — anti-freeloading without a global token. Brainstorm findings + prior-art survey.

# Reciprocity economy brainstorm

Ideation session exploring how Myrhiza enforces "peers are the
infrastructure" against freeloaders **without a global token or central
ledger** — by mirroring human reciprocity: each peer privately tracks
who has done cryptographically-attributed work for it, and prioritizes
serving peers in proportion to that personally-observed contribution.

This report captures **(a) the model that emerged**, **(b) what's
locked vs. open**, and **(c) a prior-art survey** (in-repo corpus +
online, produced by an 11-agent research workflow and citation-verified).
It is exploratory — a launchpad for a future `myrhiza-participation-*`
spec — not a settled design. Several hard sub-problems are deliberately
parked behind placeholders.

> **Relationship to existing docs.** This realizes nothing yet; it feeds
> the participation framework named-but-unbuilt in
> [`specs/2026-05-09-myrhiza-master-design/maintenance.md §12`](../../specs/2026-05-09-myrhiza-master-design/maintenance.md).
> It is the consumer-side application of
> [`prior-art/sybil-resistance/`](../../prior-art/sybil-resistance/).

## Files in this report

| File | Contents |
|---|---|
| `README.md` (this file) | Distilled findings, the model, locked/open decisions, prior-art map |
| [`transcript.md`](transcript.md) | Full verbatim transcript (the primary source) |

## How to use this report

1. **Read "The model so far"** for the current design shape.
2. **Read "Open forks"** before extending — several decisions are
   deliberately unresolved and parked behind placeholders.
3. **Read "Prior-art map"** for what exists in the wild and what to
   borrow; the **promotion candidates** name folders the corpus is
   missing.
4. **The full reasoning** (why each refinement was made) is in
   [`transcript.md`](transcript.md).

---

## The question

A P2P runtime where peers *are* the infrastructure can't assume everyone
runs a cooperative client — a freeloader can run custom software that
consumes but never contributes. The proposed defense mirrors human
trust: **no global token** (not Ethereum-style absolute truth); instead
each peer keeps a **private, per-counterparty record** of
cryptographically-attributed work done *for it*, and biases its own
serving/prioritization toward peers in proportion to that personally
observed work. Anti-freeloading is **emergent** — a freeloader is
independently deprioritized by each peer it fails to reciprocate with —
not centrally enforced.

## The model so far

A layered design built up over the session. Each layer is independently
swappable; the hard valuation question is isolated behind a placeholder.

### 1. Standing curve — smooth, deficit-tolerant, no cliff
A peer's **standing** with me is a **smooth monotonic function of our
running net imbalance** (`Σ value(their work for me) − Σ value(my work
for them)`). High contributors get near-certain priority; deep debtors
slide toward "ignored." **No hard cutoff** — priority degrades on a
gradient (the deployed analogue is IPFS Bitswap's whitepaper debt-ratio
probability curve; GNUnet's "priority capped by earned trust" is the
same shape). Recovering from a deep hole costs proportional work by
default; making deep holes *disproportionately* sticky (betrayal is hard
to forgive) is an optional convex/hysteresis knob.

### 2. Decay — consumption-relative, not wall-clock
Standing erodes **only as a peer consumes without reciprocating** — a
quiet peer that asks for nothing never decays; a peer that keeps taking
while giving nothing decays fast. "Time" is measured in *work I've done
for you since you last reciprocated*, **not** calendar time. (Rejected:
wall-clock decay — non-convergent across peers, and punishes
honest-but-idle peers. The deployed analogue of the temporal-erosion
idea is MeritRank's epoch decay.)

### 3. Grace buffer — deficit tolerance, scaled by social graph
Relationships need not balance moment-to-moment; every peer extends a
**grace buffer** of work-before-reciprocation. **Critical:** a flat
grace buffer is exactly what a Sybil farms (mint N identities, harvest
N × grace, discard). The fix mirrors human trust — **buffer size scales
with social-graph proximity** (Myrhiza's invite graph). This bounds the
Sybil harvest to the attacker's real graph footprint, not its identity
count, and composes with the social graph as the primary Sybil-admission
defense.

### 4. `value(work)` — the parked hard problem, behind a placeholder
The value of a unit of work is hard and deferred. Everything else is
designed against a **placeholder** with this contract:
- **Additive & commensurable** — contributions sum into one running
  scalar per relationship, so imbalance is well-defined.
- **Non-negative; illegitimate work scores 0** — the entire "is it real
  / was it wanted" problem *collapses into* "`value()` returns 0 for
  work that doesn't count." Downstream assumes any positive value is
  legitimate.
- **Local & subjective** — each peer runs its own `value()`.

**Leading proposal (turns 6–9): resource-recipe × subjective shadow-prices.**
Work = running a *module* on a peer's behalf; the *module* (not the app)
is the natural value authority. The leading shape:

> `value_P(action) = resource_vector(action) · shadow_prices_P`

- The **module declares the objective resource recipe** — what the action
  consumes (CPU-ms, byte-hours, bandwidth). Same neutral code everywhere →
  verifiable, agreed by both peers, cross-app comparable, resistant to
  per-app inflation. Physical units are universal across all apps, which
  also shrinks the cross-app `convert()` residue (layer 5).
- Each **peer applies its own subjective shadow-prices** — how scarce each
  resource is *to it* (fast CPU → compute cheap to give; scarce disk →
  persistence expensive). Value emerges from hardware reality, not
  declaration, and produces comparative-advantage gains from trade.
- **Directional rule (trust-minimal):** value work *I do* by my *measured*
  cost (I ran it — no trust needed); value work *I receive* by my *own
  replacement cost* (what it would cost me to reproduce) — never by the
  counterparty's self-reported number. Concentrates the gaming surface on
  the crediting side, then removes it.

This **subsumes** the two pure forms rather than choosing between them:
module-declared *objective* schedules and *subjective* self-measured cost
are its two halves. **Substrate reality (verified against code,
2026-05-29):** the resource vector is largely *greenfield*. Myrhiza meters
WASM fuel only as a per-*profile* trap-on-exhaustion *limit* that is
**consumed-but-never-read** (no `get_fuel` call anywhere in `crates/`;
`determinism.md §5.3`, `limits.rs`); per-host-call fuel costs are spec'd
but **not enforced** (`gating.rs`); memory is a flat 64 MB cap; and
bandwidth / byte-hours have **no counter at all**. Per-module resource
declarations don't exist (`maintenance.md §12.7` defers them; v1 ships
zero maintenance modules). So `resource_vector` needs **new
instrumentation** — fuel read-back through the `Backend` trait + byte
counters in `crates/network/` — and the "module declares the recipe"
premise is greenfield. Still open within this candidate: doer-vs-recipient
edge cases, the replacement-cost cap for work I couldn't reproduce, and
`value()` proper (Open fork #2) stays parked.

### 5. Scope — per-(peer, module/app) base; global deferred
Value is well-defined **per module/app**, where there's context to say
what an action is worth. A **global** per-peer standing additionally
requires a cross-app *exchange function* (`convert()`), which is hard
because apps can't be trusted to set their own cross-app rate (a junk
app would inflate its actions to mint cheap global standing). **Shared
modules partially dissolve this** — work through the same standard
module is denominated identically across apps for free; and if work is
measured in *physical resource units* (the synthesis above), those are
universal across all apps, shrinking the residue further. **Base scope =
per-(peer, module/app); global = a future aggregation layer.**

### 6. Where it lives — non-authoritative, per-peer
The ledger is **not** converged state. Per
[`determinism.md`](../../specs/2026-05-09-myrhiza-master-design/determinism.md),
no transport/real-time/peer-local observation (who-served-me, latency,
current CPU/disk pressure) may enter `state-apply`. So the standing
ledger and `value()` live in a **per-peer behavior/interaction
component**, non-authoritative network-wide. This is not a limitation —
it's *why* subjective real-time resource pricing (layer 4, turn 9) is
even possible: such prices can never converge, so they can only live
here.

## Locked decisions (this session + spec constraints)

| Decision | Rationale / source |
|---|---|
| **No global token, ever** | Shifts center of gravity to speculation; `prior-art/sybil-resistance/lessons.md:31` |
| **Reciprocity logic is a *module*, not a kernel built-in** | `maintenance.md §12.2` names `myrhiza-participation-tit-for-tat` |
| **Ledger is per-peer, non-authoritative (behavior component)** | Determinism boundary (`determinism.md`) |
| **Social-graph admission is the *primary* Sybil defense; reciprocity composes with it** | `sybil-resistance/lessons.md:50` (three-leg framing) |
| **Decay is consumption-relative, not wall-clock** | Robust to lurkers; convergence-friendly (turn 4) |
| **Standing is a smooth gradient, no hard cutoff** | Deficit tolerance; matches Bitswap/GNUnet (turn 4) |
| **Grace-buffer size scales with social-graph proximity** | Bounds Sybil harvest to real graph footprint (turn 3) |
| **Base scope = per-(peer, module/app)** | Value is only well-defined where there's context (turn 6) |

## Open forks (parked — resolve before spec)

1. **Valuation approach — leading candidate chosen (layer 4), details
   open.** The resource-recipe × shadow-prices synthesis is now the
   *leading* proposal; the pure module-schedule and pure self-cost forms
   are subsumed as its two halves. Remaining open: doer-vs-recipient
   directional edge cases, the replacement-cost cap for non-reproducible
   work, and whether the resource vector binds to Myrhiza's existing
   fuel/memory metering or a richer recipe.
2. **`value()` definition** — what is a unit of work, and how is it made
   verifiable + tied to value the recipient actually *wanted* (so
   colluders can't trade real-but-useless work to pump balances)? The
   hardest problem; deliberately deferred.
3. **Global vs. per-app standing** — the `convert()` cross-app exchange
   placeholder. Shared modules / physical-resource units shrink it but
   don't fully erase the residue for app-specific work.
4. **Recovery shape** — proportional vs. deliberately-sticky (convex
   penalty / hysteresis) for deeply imbalanced relationships.
5. **Ratio vs. absolute deficit** — normalize the imbalance by total
   throughput (so a 90%-reciprocating heavy contributor isn't punished
   for a large *absolute* deficit)?
6. **The output/enforcement side — NOT YET DISCUSSED.** What low standing
   *concretely does*: kernel-enforced queue priority, fuel/bandwidth
   caps, probabilistic decline. This is where Myrhiza's
   capability-mediated kernel is the distinctive lever (refusal is a
   first-class primitive, not advisory app logic). The natural next
   session.

## Prior-art map

**Taxonomy placement.** Category 1 — *tit-for-tat / pairwise
reciprocity* (`sybil-resistance/taxonomy.md §1`), pushed one step toward
Category 2 (*subjective/local reputation*) by the durable, signed,
multi-interaction memory. Explicitly **not** Category 4 (PoW/PoS/token,
rejected) and **not** a replacement for Category 3 (social-graph Sybil
defense — the orthogonal admission layer it composes with).

**Closest existing systems (ranked):**

| System | Why close | What it adds |
|---|---|---|
| **OurGrid Extended Network of Favors (ExtNoF)** (HPDC 2004) | Token-free, local per-peer favor balances over **multiple heterogeneous services** (CPU/disk/bandwidth); peers swap cheap-for-them for scarce-for-them *because they value services differently* — `value_P = resource_vector·shadow_prices` in all but name | **The closest published realization.** Empirically marginalizes free-riders even when donation-cost ≈ utility-received; its open problem (deriving exchange rates / which service to offer) is exactly what replacement-cost crediting claims to answer. Now documented in [`p2p-resource-economics/ourgrid.md`](../../prior-art/p2p-resource-economics/ourgrid.md). |
| **GNUnet excess-based economic model** (Grothoff 2003) | Private per-peer trust earned-by-replies / spent-by-requests, no global currency — near 1:1 match | The **excess rule** (charge only under load → dissolves newcomer problem); structural Sybil immunity (fresh identity gains nothing; damage bounded `d ≤ c + ε`); transitivity-by-delegation. Now documented in [`p2p-resource-economics/gnunet.md`](../../prior-art/p2p-resource-economics/gnunet.md). |
| **IPFS Bitswap + BitTorrent choking** | Per-peer byte ledger / pairwise tit-for-tat; the debt-ratio probability curve *is* the grace-buffer gradient | Deployed lesson: "ledger exists ≠ ledger enforced" (`ipfs-bitswap.md:90`); optimistic unchoke as the newcomer escape valve |
| **Tribler TrustChain + MeritRank** | Signed bilateral per-peer work records (= "cryptographically-attributed work for me"); personalized Sybil-tolerant trust | TrustChain = the tamper-evident record structure; MeritRank = transitive trust via ego-centric walks with decay knobs, no global consensus |
| **Samsara** (Cox & Noble, SOSP 2003) | Closest precedent for **replacement-cost crediting** — prices received storage in units of the *same resource the recipient must give up*, token-free | But storage-for-storage only; it *destroys* specialization (the anti-comparative-advantage degenerate case) — shows the limit of replacement-cost crediting before heterogeneity. Now in [`p2p-resource-economics/samsara.md`](../../prior-art/p2p-resource-economics/samsara.md). |
| **Credit networks** (Trustlines/Ripple, **Bazaar** NSDI 2011) + **image scoring** (Nowak & Sigmund, Nature 1998) | Token-free transitive trust as a graph of bilateral records | Shows the cost of going transitive (something must see >1 edge); Bazaar's third-party-verifiable *value* edge |
| **Market-based control & pricing theory** (turn-9 lineage) | Subjective/marginal value (Menger), comparative advantage (Ricardo), shadow prices = LP-duals (Kelly NUM 1998), computational economies (Spawn 1992, Tycoon ~2004) | Legitimizes `value = resource_vector·shadow_prices` as a sound *local* Lagrangian cost — **but** mostly research-only (see model challenges), and DRF/Kelly bound how far it generalizes. BOINC credit is the contrast (normalizes hardware *out*; we keep it *in*). Now documented across [`market-based-control/`](../../prior-art/market-based-control/) + [`resource-pricing-theory/`](../../prior-art/resource-pricing-theory/). |

**Hard tensions any design must confront** (full detail in transcript /
`sybil-resistance/`): newcomer/bootstrap, whitewashing via cheap
pseudonyms (Friedman-Resnick 2001; iroh EndpointIds are free to mint),
Sybil multiplication (per-connection tolerant ≠ aggregate resistant),
no-transitivity / the "everywhere" gap, defining + *verifying* work
(collusion on useless work), and asymmetric demand (snapshot/sync
providers and relay-bound peers can't reciprocate symmetrically).

**What's novel about the Myrhiza framing:** (1) the unit of work is
**cryptographically-attributed component/module execution, not bytes** —
the signed substrate gives for free what BarterCast lacked and TrustChain
bolted on; (2) the **capability-mediated kernel** makes "refuse to serve"
a first-class primitive, not advisory app logic (why Bitswap's ledger
went unenforced); (3) it's runtime-native and composes with a **free
social graph** (the Sybil anchor prior systems lacked). The *composition*
is novel and unproven even though each piece has prior art.

## What the leading `value()` must answer (model challenges)

The gap analysis (2026-05-29) surfaced verified challenges the
resource-recipe × shadow-prices model must answer before it becomes a
spec. The throughline: **use the dot-product only as a cost/credit
*metric*, never as the resource-*allocation* rule.**

1. **Fairness-inferiority (the load-bearing hit).** DRF's *Asset-Fairness*
   counterexample (Ghodsi et al., NSDI 2011) proves "price every resource
   and sum" — structurally `resource_vector·shadow_prices` with a shared
   price vector — can **violate the sharing-incentive property**: a peer
   ends up worse off than under a static equal split. DRF achieves fairness
   over the resource *vector* with *no prices* and shipped at scale
   (Mesos/YARN). **Mitigation:** confine the dot-product to a cost/credit
   metric; govern allocation with a demand-aware dominant-resource rule.
2. **Non-substitutability (Leontief).** CPU/RAM/bandwidth aren't
   substitutable — a CPU-blocked job gets zero value from extra RAM — so a
   fixed price vector misprices the *bottlenecked* actions (clouds bundle
   prices for this reason). **Mitigation:** price against the *binding*
   resource (max over resources), or carry the vector and reconcile
   per-resource rather than collapsing prematurely.
3. **"Markets are overkill."** The cluster that killed every computational
   economy — price volatility, discovery latency, bid-authoring UX — plus
   the damning tell: Waldspurger built Spawn (a market, 1992) then
   *abandoned it* for price-free lottery/stride scheduling (1994), and the
   price-free one shipped (Linux CFS lineage). **Our defense:** this is
   per-peer *local crediting off the determinism path, not a live auction*,
   so it sidesteps volatility/discovery/bid-UX entirely — state that
   explicitly. Default to share-based; reserve pricing for cross-trust
   exchange (the Coase firm/market boundary).
4. **Gaming self-reported cost.** A self-attested `resource_vector` is a
   credit-stuffing vector (Gridcoin minted 72.4 coins from unauthenticated
   BOINC claims; BOINC's credit history is one long replication+quorum
   arms race). **Mitigation = the directional rule** (value *received* work
   by *my own* replacement cost, never their number) + kernel-side metering
   of *actual* consumption + signing the cost claim *inside* the hashed
   event + re-execution via the existing `state-apply` dry-run as a
   verification quorum.
5. **Soundness honesty.** Independently-set local shadow prices are a
   *locally-valid Lagrangian cost*, **not** a globally-optimal market
   clearing (NUM optimality needs cross-peer price reconciliation the
   directional rule deliberately forgoes). Label it a trust-minimal
   heuristic — don't borrow NUM's rigor without its preconditions.
6. **Granularity (Coase).** Pricing every inter-component action has real
   overhead; below a threshold flat fair-share is cheaper (Miller & Drexler
   concede this; Bitswap shipped having *stripped* its byte ledger).
   **Mitigation:** flat fair-share *within* a trust domain; `value_P` only
   *across* trust boundaries.

## Prior-art promotion candidates

Flagged per `using-prior-art`. **Items 1–4 were built and landed on
2026-05-29** (this session) — they are now folders in the corpus, linked
below. Items 5–6 remain open.

1. ✅ **LANDED — [`prior-art/p2p-resource-economics/`](../../prior-art/p2p-resource-economics/)** — token-free heterogeneous reciprocity + replacement-cost precedents. **OurGrid ExtNoF** (priority-1, closest realization), **Samsara** (SOSP 2003), **GNUnet** excess-model (Grothoff 2003), with **Karma** / **Maze** / **Dandelion** as global-scrip cautionaries. Where the leading model actually lives.
2. ✅ **LANDED — [`prior-art/market-based-control/`](../../prior-art/market-based-control/)** — the computational-economy paradigm `value_P` descends from + its deployment-failure evidence (Sutherland 1968, Miller-Drexler 1988, Spawn 1992, Tycoon, Mirage-vs-Bellagio, SHARP, Clearwater). Kept **separate** from `agoric-endo/` (SES company ≠ market-control paradigm); sharpens the "tokenomics are not a runtime concern" tension.
3. ✅ **LANDED — [`prior-art/resource-pricing-theory/`](../../prior-art/resource-pricing-theory/)** — the formal soundness *and* the load-bearing critique: Kelly NUM (shadow-price = LP-dual), **DRF + the Asset-Fairness counterexample** (read off the primary NSDI PDF), Briscoe ConEx.
4. ✅ **LANDED — [`prior-art/sybil-resistance/self-reported-cost-verification.md`](../../prior-art/sybil-resistance/self-reported-cost-verification.md)** — verification-of-self-reported-cost: **BOINC** (cobblestone + replication/quorum + anomaly bounds), Folding@home, **Gridcoin** (the 72.4-coin unauthenticated-claim attack, verified verbatim).
5. 🟡 **OPEN — Indirect-reciprocity / credit networks** — TrustChain (FGCS 2020), MeritRank (BRAINS 2022, arXiv:2207.09950), Trustlines, Bazaar (NSDI 2011), image scoring (Nature 1998). For the transitive-contribution gap. (Previewed in `p2p-resource-economics/` via GNUnet's delegation-with-margin + the EigenTrust cross-ref; no dedicated folder yet.)
6. 🟢 **OPEN (low) — Free-riding economics + REA/ValueFlows** — Friedman & Resnick "cheap pseudonyms" (2001) for the formal newcomer-distrust limit; REA as the nearest ledger ontology if the per-peer ledger needs a schema.

**Corrections the build surfaced** (2026-05-29, all verified off primary
sources): the **DRF Asset-Fairness counterexample** was read off the primary
NSDI PDF — the brainstorm/HTML-mirror had conflated the §5.1 `$6/$7` pricing
illustration with the *separate* ⟨30,30⟩ Theorem-1 sharing-incentive proof;
now correctly separated in `resource-pricing-theory/dominant-resource-fairness.md`.
Other fixes: Karma's middle author is **Chandrakumar** (not "Chakravarty");
**Dandelion** is server-mediated (not decentralized); DRFH authorship is
**Wang/Liang/Li** (no "Liu"); Bellagio venue is **OASIS 2004**; ConEx is
**IETF Experimental** (not deployed); GNUnet confirmed still maintained
(0.27.0, 2026); Spawn's "sealed-bid second-price (Vickrey)" detail confirmed
verbatim from IEEE TSE p.105.

## Cross-links

- [`maintenance.md §12`](../../specs/2026-05-09-myrhiza-master-design/maintenance.md) — the participation framework this feeds (`myrhiza-participation-*`).
- [`determinism.md`](../../specs/2026-05-09-myrhiza-master-design/determinism.md) — why the ledger is non-authoritative.
- [`prior-art/sybil-resistance/`](../../prior-art/sybil-resistance/) — the corpus this consumes (`taxonomy.md`, `lessons.md`, `bittorrent.md`, `ipfs-bitswap.md`).
- [`prior-art/willow/open-problems.md`](../../prior-art/willow/open-problems.md) — the canonical statement of Myrhiza's enforcement problem.
- [`prior-art/agoric-endo/`](../../prior-art/agoric-endo/) — shared *lineage* only (Miller-Drexler personnel via `history.md`); market-based-control is a **separate** candidate folder, not an extension of this one (it commits to the SES/ocap story).

## Next step

Resolve **Open fork #6** — the output/enforcement side (what low standing
concretely does at the capability boundary) — then converge **Open fork
#1** (valuation approach) enough to draft a `myrhiza-participation-*`
spec. The `value()` definition (#2) can remain a placeholder through the
first spec.
