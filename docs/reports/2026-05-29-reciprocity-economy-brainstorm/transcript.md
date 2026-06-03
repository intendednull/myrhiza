**Date:** 2026-05-29
**Status:** active
**Subject:** Full transcript — reciprocity-economy / anti-freeloading brainstorm

# Transcript — reciprocity economy brainstorm

Verbatim record of the ideation session that produced
[`README.md`](README.md). User turns are reproduced exactly (including
original typos); assistant turns reproduce the prose sent. Research-tool
actions (a multi-agent prior-art workflow, citation-verification greps)
are summarized in `[bracketed notes]` rather than dumping raw output.

For the distilled findings, locked decisions, open forks, and prior-art
map, read [`README.md`](README.md) instead — this file is the primary
source it was built from.

---

## Turn 1 — User

> I'd like to ideate a little bit about what this runtime is and what its goals are. There is a rather hard question that we've talked about a little before where the idea of peers being the infrastructure for the runtime apps is a little hard to enforce because peers might choose to run a different client and not follow the rules that we have set for our own clients. So we need some kind of protocol that is enforced for anybody running proper clients that is hardened against freeloaders. Now I was thinking we could mirror how human communication and human trust is already built up. We already have a sort of set of rules that we follow in society. That being, people that help you do things for you, tell you the truth, you trust them more. And so perhaps there isn't some sort of global token. This isn't like Ethereum where we have a generalized computer with absolute truth. we do kind of have a generalized computer. That is true. However, the work being done is based on the intimate trust between two peers. in that a peer can track who has done work for them. Right? because of our cryptography, we can guarantee that the work that was done is from a certain peer. So internally, we can track who has done work for us, and then we can say, okay. This peer has done work for us, so we don't mind doing work for this peer. and we would need to figure out what work actually means. But I think generally, the idea of delegating work based off of who has done work for you is an interesting thought. because for a freeloader, this means that if they interact with other peers and do no work for them, eventually, the peers will stop interacting with them. at least to the point where if they need work done or they are busy, the freeloader peer is the last priority. they experience poor network conditions because they are not contributing to the network. This isn't tracked anywhere centralized. There isn't a central source of who is doing however much work. But if you are doing lots of work for lots of peers, you are in a kind of high priority state based off of the amount of personal work that you've done for each of those peers. Does that make sense to you? Is there anything else out there in terms of software that expands on this idea?

## Turn 1 — Assistant

*[Loaded the `brainstorming` and `using-prior-art` skills; surveyed `docs/` and found the directly-relevant `docs/prior-art/sybil-resistance/` corpus + master spec; launched a 10-agent research workflow (4 reading the in-repo prior-art + master spec, 6 surveying the software landscape online with verification, then 1 synthesizing).]*

I've launched a 10-agent research workflow. While that runs, the short answer to *"does this make sense?"* — **yes, and you've independently reinvented one of the oldest live debates in P2P systems**, which is a good sign. What you're describing is **direct (pairwise) reciprocity** evolving toward **subjective/local reputation**: each peer keeps its own private ledger of "who has done work for me," and resource priority is an *emergent* property of those private ledgers rather than anything globally agreed. That's exactly the family BitTorrent's tit-for-tat, IPFS's Bitswap ledger, and — most precisely — **GNUnet's "excess-based economic model"** live in. GNUnet (Grothoff, ~2003) is almost a one-to-one match for your framing: purely local, per-peer *trust* earned by doing work, spent preferentially when allocating spare capacity, with no global currency. The repo's own `sybil-resistance/` corpus surveys most of this — but interestingly, **GNUnet isn't in it**, so your instinct is pointing at a real gap.

The research is digging into the parts you haven't pinned down yet: what counts as verifiable "work," how newcomers bootstrap (the famous weakness of pure reciprocity), and how to get *transitive* trust ("my friend vouches for you") without a global token.

## Turn 2 — Assistant (after research workflow completed)

*[The 11-agent workflow completed (~703k tokens). Verified the load-bearing in-repo citations by grep before relaying them: `maintenance.md §12` exists and names `myrhiza-participation-tit-for-tat`; `lessons.md` lines 13/31/50 confirm "reciprocity beats reputation," "don't add a token," and the three-leg framing.]*

### Yes — and you've half-decided this already

Your instinct is sound, and it's the **single most validated incentive shape in deployed P2P**. More than that: *Myrhiza's own repo already committed to this direction.* `docs/prior-art/sybil-resistance/lessons.md` states it outright — "Reciprocity beats reputation in practice" (line 13) and "Don't add a token… Myrhiza is a peer-runtime, not a market" (line 31). And `maintenance.md §12.2` already names `myrhiza-participation-tit-for-tat` as a planned module (alongside `-social-graph`). So you're not breaking ground here so much as *returning to a fork the spec already leans toward but left unbuilt* (v1 ships zero participation modules).

### The closest existing system to your *exact* idea: GNUnet

GNUnet's **excess-based economic model** (Grothoff, 2003 — `grothoff.org/christian/ebe.pdf`) is nearly a one-to-one match: each node keeps a *private, per-peer trust/credit* value, **earned** by doing work (serving valid replies) and **spent** by making requests, with no global currency and no central ledger. "No node owns trust; the trust a node earned is stored at the other nodes" — that's your "intimate trust between two peers," formalized in 2003.

It adds two things you haven't specified:

1. **The excess rule** — charge *only* under load. When a peer has spare capacity, it serves *everyone* free; reciprocity only gates access when resources are contended. This dissolves the newcomer problem with no special-casing.
2. **Structural Sybil/whitewashing immunity** — because earned trust is worthless at any *other* peer, and a request's priority is capped at the trust the requester has *earned with you*, "a fresh identity gains nothing." Attack damage is provably bounded to `d ≤ c + ε`, *regardless of how many identities they mint*.

### The rest of the landscape, in two buckets

**Deployed, no-token, pure-local (proves your model works):**
- **BitTorrent choking + optimistic unchoke** (Cohen 2003) — 25 years, hundreds of millions of clients. "Serve those who serve me" with one rotating charity slot for newcomers. (`prior-art/sybil-resistance/bittorrent.md`)
- **IPFS Bitswap** — a literal per-peer byte ledger (credit = received − sent). *But the cautionary tale:* the ledger was largely **never enforced** in production go-bitswap (`ipfs-bitswap.md:90`), because enforcement lived in cooperative app logic and peer-IDs were free.

**The signed + transitive extension (where "cryptographically attributed" leads):**
- **Tribler's TrustChain** (FGCS 2020) — every peer owns a personal hash-chain; each work record is a pair of *entangled, bilaterally-signed* blocks. *Literally* "cryptographically-attributed work done for me," built to fix what killed its predecessor BarterCast: unsigned, forgeable records.
- **MeritRank** (BRAINS 2022, `arxiv.org/abs/2207.09950`) — derives *personalized*, Sybil-tolerant priority via ego-centric random walks seeded at *you*, with decay knobs that asymptotically bound an attacker's gain. No global consensus, no token.
- **Credit networks** — Trustlines / Ripple-style IOU graphs, and **Bazaar** (NSDI 2011), where trust = max-flow of *completed-transaction value*. Theoretical root: **Nowak & Sigmund's "image scoring"** (Nature 1998) — reputation is what lets cooperation emerge among strangers.

### The one hidden tension worth naming

You want a freeloader to experience poor conditions **everywhere it goes** — but a *purely local, no-gossip* ledger structurally **cannot deliver "everywhere."** Each peer only knows the work done for *itself*. A fresh peer meeting the freeloader has zero signal until *it* gets burned. So "everywhere" is either **statistical-emergent** (the freeloader must independently disappoint enough peers) or it requires a **bounded transitive/gossip layer** — exactly what TrustChain+MeritRank and credit networks add, at the cost of a more-than-one-edge view. That fork — pure-pairwise vs. transitive — is the decision everything else hangs off.

### Your structural advantages

- **Work = cryptographically-attributed component execution, not bytes.** Every prior system tracks a forgeable transport commodity and had to bolt on signing (TrustChain) or gave up (BarterCast). You get the signed substrate *for free* — and crediting *work* rather than *bytes* attacks the asymmetric-peer problem.
- **Capability-mediated host → "refuse to serve" is a kernel primitive,** not advisory app logic. *Exactly* why Bitswap's ledger failed.
- You already carry an invite/social graph (Willow) as a costly-to-mint Sybil anchor — the "entry cost" Friedman & Resnick (2001) prove *any* free-identity reciprocity scheme needs.

### Prior-art flags (per `using-prior-art`, non-blocking)
- 🔴 **GNUnet excess-based model** — highest priority; closest formal match, *entirely absent* from `sybil-resistance/`.
- 🟡 **Indirect-reciprocity / credit networks** — TrustChain, MeritRank, Trustlines, Bazaar, image scoring.
- 🟡 **Free-riding economics** — Friedman-Resnick "cheap pseudonyms" theorem + Feldman et al. (EC'04).

### Question posed
Do you want true "everywhere" deprioritization (→ MeritRank/credit-network territory), or is per-peer + statistical-emergent enough? And what do you picture "work" *being*?

## Turn 3 — User

> No, the freeloader thing isn't exactly right. Work doesn't need to be perfectly balanced all the time. Everyone has a grace buffer, doing work even if no work has been done in return yet. Its the peers that don't do work for an extended time that start to be low priority

## Turn 3 — Assistant

*[Verified the Bitswap debt-ratio / sigmoid claim against `ipfs-bitswap.md` before leaning on it.]*

That's a meaningful sharpening, and it makes the model *more* robust. You've moved from a **balanced ledger** to a **deficit-tolerant one with decay** — a *leaky bucket* (or "debt ratio"), not a balance sheet:

- **The grace buffer = the flat top of a curve.** Bitswap's whitepaper does this literally: serve a peer *probabilistically*, where probability stays near-certain across a band of small debt (the grace buffer) and only slopes toward "ignored" as debt grows large (your "last priority"). Nobody is cut off the instant the books go negative — they slide down a gradient.
- **"Extended time → low priority" = decay.** MeritRank's *epoch decay* — old contributions lose weight, so standing has to be *continuously* refreshed.

**But the refinement relocates the hard problem.** The grace buffer is *precisely the thing a Sybil farms.* Mint N identities, harvest N × grace, discard, repeat — the freeloader just wears N faces, each looking like an honest newcomer for as long as the grace lasts.

The fix is the intuition you opened with: **human trust is graph-shaped, so the grace buffer shouldn't be flat.** Generous buffer to a friend-of-a-friend, thin buffer to an anonymous stranger. Concretely: grace-buffer *size scales with social-graph proximity* (Myrhiza's invite graph). That bounds the Sybil harvest to the attacker's real graph footprint instead of their identity count.

### Question posed — what makes standing decay?
- **(a) Wall-clock:** contribute periodically or erode even while idle. Leans on a non-convergent clock; punishes honest-but-idle peers.
- **(b) Consumption-relative:** erode *only as you consume without giving back.* A quiet peer never decays; a taker decays fast.

Argued (b) is the robust one (can't be gamed by a lurker; "time" = work-I've-done-for-you, not calendar time; fits the determinism boundary since the ledger is a per-peer non-authoritative view).

## Turn 4 — User

> b, consumtion on a spectrum. the further out of balance, the further decayed. there might not be a hard cutoff, but greatly imbalanced relationship will take a lot of work to recover

## Turn 4 — Assistant

(b) on a spectrum is the robust choice, and you've now fully specified the curve: **priority is a smooth monotonic function of the running net imbalance**, no cliff, and you climb out of a hole by doing the work to refill it. That's Bitswap's debt-ratio design and GNUnet's "priority capped by standing," generalized.

- **On "takes a lot of work to recover."** If priority tracks the *current level* of imbalance, recovery is automatically proportional. If you want deep holes *disproportionately* sticky (betrayal is hard to forgive — very human), that's an explicit extra knob: a convex penalty, or a slow-moving average that remembers the hole. A deliberate choice, not settled now.
- **Quiet sub-decision: ratio vs. absolute deficit.** A contributor who's done 10k for you and received 9k is −1k *absolute* but 90% reciprocated; punishing the absolute number would deprioritize your best partner. Robust forms normalize deficit by throughput (or use a ratio with a volume floor). A dial, not a fork to resolve yet.

### The crux you can no longer defer
"Imbalance on a spectrum" is undefinable until there's a *common unit* for work in both directions, and that unit must clear two bars:
1. **Commensurable** — serving-sync and running-a-persister must net in one currency.
2. **Tied to value the recipient actually wanted.** The smooth no-cutoff curve makes this *sharper*: two colluding identities can do *real but mutually useless* work for each other, keep balances healthy, then both freeload on honest peers. BitTorrent is immune only because the bytes you trade *are the file you asked for*. Bazaar (NSDI 2011) had to make trust edges third-party-verifiable *value*. The spec already names the hazard: `willow/open-problems.md` — "self-reported participation is gameable" — and `lessons.md:29` points at lightweight audit challenge-response.

### Question posed
What is one unit of work, concretely? (Maintenance-component executions? Volume of state served? An audit challenge passed?) And is there an anchor that says *I actually asked for this*, so work can't be unilaterally manufactured to pump a balance?

## Turn 5 — User

> the value of work is a hard problem, and will take a lot of discussion, for now lets keep it abstract, maybe with a placeholder

## Turn 5 — Assistant

The discipline: draw a clean boundary around the hard problem so everything else can be designed against it, and so when we *do* tackle it, we know exactly what slot it fills.

**Placeholder: `value(work) → scalar`** — a local function each peer runs to score a unit of work. The rest of the model only needs these properties:
1. **Additive & commensurable** — contributions sum into one running number per relationship, so `standing = Σ(their work for me) − Σ(my work for them)` is well-defined.
2. **Non-negative, illegitimate work scores 0** — the entire "is it real / was it wanted" problem *collapses into* "`value()` returns 0 for work that doesn't count." Downstream can assume any positive value is legitimate.
3. **Local and subjective** — each peer runs its own `value()`; I score your help by my own utility. Lives in the non-authoritative per-peer component.

With that boundary drawn, the deferred discussion is crisply scoped: *"define `value()` such that property #2 holds."* The surrounding mechanism — imbalance curve, consumption decay, graph-scaled grace — is fully designable now, treating `value()` as a black box.

### Question posed — scope of standing
Myrhiza is an *app* runtime and the spec scopes maintenance per-app (`maintenance.md:56`). So: **is standing global per peer-pair, or scoped per (peer, app)?**
- **Global per-pair:** goodwill spills over (a chat persister earns photo-app priority). Very human; one ledger. But couples unrelated app economies and lets someone farm standing in a cheap app to spend in an expensive one.
- **Per (peer, app):** matches the sandbox boundary; clean isolation, but no spillover and thin relationships fragment into many near-zero ledgers.

When you picture a freeloader deprioritized — globally with you, or only within the app they freeloaded on?

## Turn 6 — User

> global is interesting, but it might be hard to evaluate the work cross-app. Defining a global work value seems much harder than doing it per app. Maybe each app designates the value of the app actions, hard to translate that value to other apps. Lets keep both in mind, answer is unclear

## Turn 6 — Assistant

That resolves the base layer and tells us where the hard part lives:
- **Intra-app value is well-defined and app-designated.** Each app is the natural authority for what its own actions are worth. So `value()` is really *per-app*, supplied by the app.
- **Global = per-app base + a cross-app exchange function.** A single global standing needs to translate chat-points into photo-points — *that* is the hard, unsolved piece. "Global" isn't a different design, it's the per-app design plus a second placeholder: `convert(app_A_value) → common_unit`, deferrable like `value()`.

The sharp part — *why* global is hard: **per-app self-valuation is safe precisely because it's self-contained.** An app inflating its own action-worth only inflates standing *within its own sandbox* — harmless funny-money. It becomes an *attack* only at cross-app aggregation: a junk app that designates trivial actions as hugely valuable could mint cheap global standing. So the exchange function **can't be set by the apps themselves** — that's the real barrier.

It also cleans up "local & subjective": **within an app, value is objective** (app-defined schedule, both peers agree → consistent bilateral ledger), and **subjectivity moves up to the cross-app layer** (each peer weights each app's points for itself).

Recorded fork: **per-(peer, app) is the base we build on; global per-peer is a future aggregation layer gated by the cross-app exchange placeholder.**

*[Offered to checkpoint the model so far as a design note, or continue to the output/enforcement side. User continued with a new refinement.]*

## Turn 7 — User

> also keep in mind the imported modules may also define work in a way, so trans-app might be just be shard modules doing there thing

## Turn 7 — Assistant

That's the piece that makes the whole thing click — and it quietly solves the cross-app exchange problem. Recall the snag: the exchange rate *can't be set by the apps themselves*, and there was no neutral party to anchor it. **A shared module is exactly that neutral party.** The layering refines one level:

- **Value is really per-*module*, not per-app.** The module that does the work declares what its actions are worth — it's the code that knows the cost. An app's economy is mostly its modules' economies.
- **Cross-app commonality emerges for free wherever apps share modules.** If two apps both import the standard `iroh-blobs-serving` module, "served 100 blobs via that module" means the *same thing* in both — same code, same units, no translation. The shared module *is* the common currency.
- **Trust shifts from apps to a small set of standard modules.** A self-interested *app* can't be trusted to value its own actions cross-app, but a shared, neutral, auditable `myrhiza-*` module can — like trusting the standard library instead of each caller. Inflation requires forking the standard module, and peers running the *real* one won't honor a forked schedule.

This rhymes with the spec: maintenance is "encapsulated in maintenance-shaped modules" via the standard module ecosystem (`maintenance.md:20,28,61`). So the infrastructure-shaped work that matters *most* (serving, persisting, syncing, relaying) is exactly what runs through shared modules — and is therefore *already* cross-app-comparable.

**What's left hard** shrinks: only work through *different, non-shared* modules still needs the subjective `convert()` placeholder.

Flag banked: for two peers to net in a shared unit they must agree on the module *and* its value schedule → make the schedule part of the module's **versioned interface**.

Convergence: **work = running a module on a peer's behalf, valued by that module's declared schedule** — and the signed-event substrate can attribute *which module did what for whom*.

### Question posed
Turn to the output side — **what low standing concretely *does*** (kernel-enforced queue priority, fuel/bandwidth caps, probabilistic decline)? Or more on valuation first?

## Turn 8 — User

> Lets save this discussion in a report. The file should contain a full transcript, as well as a summaraized report of our findings. Lets make both easy to discover and navigate for future agents.

## Turn 8 — Assistant

*[Produced this report folder: `transcript.md` (this file) + `README.md` (summary of findings), catalogued under "Identity, crypto & trust" in `docs/README.md`. Flagged a pre-existing merge-conflict-marker issue in `docs/README.md` (Runtime core area) as out-of-scope for separate resolution. The user's turn-9 message arrived mid-task and was folded into both files before delivery.]*

## Turn 9 — User

> what would happen if every peer defines their own value of work depending on space and time? Like a peer with a fast processor can do a lot of processing, and so takes less time, so their relative value for that kind of work is pretty low. Or if someone has low storage, that kind of work takes up a lot of value for them. Then the work isn't hard defined, but something each peer will measure for themselves.

## Turn 9 — Assistant

This is a strong idea with deep roots, and it gives `value()` a principled basis instead of an arbitrary one. What's being described is the **subjective / marginal theory of value** (Menger, 1871) applied to compute: a unit of work is worth what it *cost the provider at the margin, given their scarcity*. Naming the lineage: subjective value (Menger), **comparative advantage** (Ricardo — heterogeneous peers specialize and both gain from trade), **shadow prices** (Kelly's network-utility-maximization, 1998 — decentralized allocation via implicit per-resource prices), and **computational economies** (Spawn 1992, Tycoon ~2004). The contrast case is BOINC credit, which deliberately *normalizes out* hardware differences; this idea keeps them *in* and makes heterogeneity the engine.

**The clarifying question it forces — whose cost?** When A does work for B, there are two different numbers: A's *cost to provide* (doer-side) and B's *value received* (recipient-side). The phrasing leans doer-side ("their relative value for that kind of work is low"). The trust-minimal resolution: each peer values work in **its own units**, but —
- value *work I do* by my *measured* cost (I ran the CPU / spent the disk — I know it, no trust needed);
- value *work I receive* by my *own replacement cost* (what it would have cost *me* to produce that result myself) — **not** by the other peer's claimed cost, which I can't verify and they could inflate.

This neatly captures comparative advantage: if you give me something cheap for you but expensive for me (you stored data I had no room for), my replacement cost is high → you earn lots of standing. Gains from trade fall out for free. (Edge case: work I literally *couldn't* do myself → replacement cost undefined; needs a cap or market-price fallback.)

**The synthesis with turns 6–7.** This *seems* to conflict with the "module declares an objective, agreed value schedule" idea (objective vs. subjective). It actually unifies: the module declares the **objective resource recipe** (this action consumed 10 CPU-ms + 2 byte-hours — verifiable, agreed by both peers), and each peer applies its **own subjective shadow-prices** for those resources: `value_P(action) = resource_vector(action) · shadow_prices_P`. Objective quantities, subjective valuations — exactly how real markets work. Bonus: physical resource units (CPU-ms, byte-hours) are universal across *all* apps, so this shrinks the cross-app `convert()` residue from turn 6 even further.

**Determinism reinforcement.** Real-time resource cost (current CPU load, disk pressure) is non-convergent, peer-local, real-time data — it can *never* enter `state-apply`. So this is only viable *because* the ledger already lives in the per-peer, non-authoritative behavior component (layer 6). The idea and that placement are mutually reinforcing; it would be impossible under a converged global standing.

**One risk.** Gaming concentrates on the *crediting* side (valuing received work) — which the "value by my own replacement cost" rule defuses, since it never trusts the counterparty's self-reported number.

*[Recorded as Open fork #1 (valuation approach + doer-vs-recipient sub-question) and added "market-based resource allocation" as prior-art promotion candidate #3. Folded into `README.md` layers 4–5.]*

## Turn 10 — User

> add it to the leading value proposition, and lets also explore gaps in prior art with this new context in mind

## Turn 10 — Assistant

*[Elevated the resource-recipe × shadow-prices synthesis to the LEADING `value()` proposal in the report (layer 4; Open fork #1 downgraded to "leading candidate chosen, details open"). Ran a second 9-agent research workflow — a prior-art gap analysis through the market-based / subjective-pricing lens, deliberately hunting the critiques of market-based allocation. Findings folded into the report; key results below.]*

**The model is less novel than it looked — and that's good news (it has empirical validation).** **OurGrid's Extended Network of Favors (ExtNoF)** (HPDC 2004) is the closest published realization: token-free, local per-peer favor balances over *multiple heterogeneous services*, where peers swap cheap-for-them for scarce-for-them *because they value services differently* — `value_P = resource_vector·shadow_prices` in all but name. It was empirically shown to marginalize free-riders even when donation-cost ≈ utility-received, and its one open problem (deriving exchange rates) is precisely what replacement-cost crediting claims to answer. **GNUnet** (provider-cost half) and **Samsara** (SOSP 2003 — the closest replacement-cost-crediting precedent, but storage-for-storage only, which *destroys* specialization) round out the closest precedents. All absent from the corpus.

**A factual correction to my own earlier claim.** I'd said "Myrhiza already meters fuel + memory per module." Verified against code: false-ish. Fuel is a per-*profile* trap-on-exhaustion *limit*, consumed-but-never-read (no `get_fuel` in `crates/`); per-host-call costs are spec'd but dead; memory is a flat 64 MB cap; bandwidth/byte-hours have no counter. Per-module resource recipes don't exist. So `resource_vector` is **greenfield** — needs new instrumentation. Corrected in the report.

**The load-bearing challenge — DRF.** Dominant Resource Fairness (Ghodsi et al., NSDI 2011) has a worked *Asset-Fairness* counterexample proving that "price every resource and sum" (structurally our dot-product) can **violate sharing-incentive** — a peer ends up worse off than a static equal split. DRF achieves fairness over the resource *vector* with *no prices* and shipped at scale (Mesos/YARN). The fix: use the dot-product **only as a cost/credit metric, never as the allocation/fairness rule**, and pair it with a dominant-resource bottleneck rule.

**Other challenges** the model must answer (all folded into the report's new "model challenges" section): non-substitutability (Leontief — a fixed price vector misprices bottlenecked actions); "markets are overkill" (Waldspurger built Spawn the market, then abandoned it for price-free lottery/stride scheduling, which shipped — but *our* scheme is local crediting, not a live auction, so it dodges price-volatility/discovery/bid-UX); gaming self-reported cost (Gridcoin minted 72.4 coins from unauthenticated claims → the directional rule + kernel-metered actual consumption + signed-claim-inside-the-event); soundness honesty (local unreconciled shadow prices are a heuristic, not a proven NUM dual); and Coase granularity (price across trust boundaries, flat fair-share within).

**The model's strongest defenses are the constraints already locked:** no global token + non-authoritative per-peer ledger off the determinism path + social-graph admission as primary → these sidestep the price-volatility/bid-UX cluster that killed the live-auction systems *and* the idle-scrip-hoarding/collusion of global scrip.

**Prior-art gaps → promotion candidates:** three NEW folders (`p2p-resource-economics/` — ExtNoF/Samsara/GNUnet/Karma; `market-based-control/` — Spawn/Tycoon/Mirage-vs-Bellagio/Miller-Drexler, explicitly *not* an extension of `agoric-endo/`; `resource-pricing-theory/` — Kelly NUM/DRF/ConEx) + EXTEND `sybil-resistance/` with BOINC/Gridcoin verification-of-self-reported-cost. Research-next order: GNUnet → ExtNoF → DRF (off the primary PDF).
