**Date:** 2026-05-29
**Status:** active
**Subject:** Verifying self-reported / self-measured contribution cost — BOINC, Folding@home, Gridcoin

# Verifying self-reported contribution cost

Every other file in this corpus measures contribution in units that are *hard to fake* because the asking peer observes them directly: BitTorrent counts *block bytes I received from you*; Bitswap counts *bytes you sent me*. The moment contribution becomes **self-measured and self-reported** — "I spent 400 CPU-ms and 12 byte-hours running your module" — the metric is no longer grounded in the observer's own experience, and a peer can simply *lie about its number*. This is the load-bearing problem for the reciprocity-economy brainstorm ([`reports/2026-05-29-reciprocity-economy-brainstorm/`](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md)), whose leading valuation model prices work as `resource_vector · shadow_prices`: a self-attested `resource_vector` is a **credit-stuffing vector**. The deepest body of deployed experience on this exact problem is **volunteer computing** (BOINC, Folding@home) and its blockchain descendant **Gridcoin** — systems that have paid out credit (and, in Gridcoin's case, real token value) for self-reported scientific work for two decades, and have the scar tissue to prove it.

The throughline: **the instant a self-reported cost becomes spendable, every unauthenticated or unverified component of that report is an attack surface.**

## The problem, stated as an invariant

Three distinct things must be true before a self-reported cost can safely become standing — and each of the systems below was broken by violating one of them:

1. **Bound to the work.** The number must correspond to work that actually happened. *(BOINC's replication/quorum re-execution; F@h's benchmarked baseline.)* — break it and you mint credit for nothing.
2. **Bounded against history.** A single claim must not be allowed to spike standing arbitrarily. *(CreditNew's anomaly cap + probation.)* — break it and one inflated report dominates.
3. **Bound to an authenticated owner, inside the committed structure.** Whoever the credit pays out to must have cryptographically produced the claim, and that binding must live *inside* the hashed/committed event. *(F@h passkeys; the exact thing Gridcoin got wrong.)* — break it and anyone copies a stranger's claim and redirects the payout.

Each section below is one deployed system's answer to one or more of these.

## BOINC credit — twenty years of an anti-gaming arms race

BOINC ([boinc.berkeley.edu](https://boinc.berkeley.edu/)) grants **credit** for completed work units. Its credit machinery is the most-evolved deployed answer to "how do you score self-reported compute fairly across wildly heterogeneous hardware without being gamed?"

- **The cobblestone normalizes hardware variance *out*.** One cobblestone is 1/200 of a day of work on a reference machine doing 1,000 double-precision MFLOPS on the Whetstone benchmark (≈1 GFLOPS); "200 cobblestones are awarded for one day of work" on that reference (Wikipedia, *BOINC Credit System*). The explicit goal is that a fast box and a slow box earn **comparable credit per unit of *useful* scientific work** — the FLOPS rating cancels hardware differences out.
  - **Contrast with the reciprocity model, and note this is a *deliberate objective choice*, not an error.** Myrhiza's `resource_vector` keeps hardware variance *in* as the cost signal — a peer's *own* scarcity (slow CPU → compute is expensive *to it*) is exactly what produces comparative-advantage gains from trade (brainstorm layer 4). BOINC wants a *fair scientific scoreboard* (hardware should not change the reward for the same science); Myrhiza wants a *trade-minimal cost metric* (hardware reality *is* the price). Different objectives — neither is wrong. Cite BOINC's normalization to a spec reader as the canonical "normalize-out" pole, so the "keep-in" decision is made consciously.
- **CreditNew + replication/quorum cross-check.** Under replication, the same work unit goes to multiple hosts; the validator "grant[s] the average of their claimed credit" over the agreeing instances (BOINC wiki, *CreditNew*). The Wikipedia summary describes the historical quorum form bluntly: "the top and bottom claimed credits are dropped and an average of the remaining is taken." This is a **verification quorum**: a peer's self-claim is cross-checked against independent re-execution by others, and outliers are discarded before any credit is granted. *This is structurally the same idea as Myrhiza re-running a candidate event through the existing `state-apply` dry-run.*
- **Claimed-vs-granted credit.** The number a host *claims* and the number it is *granted* are distinct quantities. Granting is derived from the quorum, never taken at face value from a single self-report.
- **Anomaly-bounding against a running average.** CreditNew caps how far a single sample can move the average: "samples after the first are capped at X times the current average. X depends on the entity: maybe 10 for hosts, 100 for app versions." *(The "~10×" multiplier is the value floated in the wiki text — note the source's own hedge "maybe 10"; treat it as illustrative of the mechanism, not a fixed constant.)* A PFC sanity check assigns a "default PFC" when a claim exceeds `wu.fpops_bound`.
- **Probation + cherry-picking defense.** Hosts that submit anomalous or inconsistent claims go on **probation** (host scaling disabled until validated jobs accrue), defending against "cherry picking" — discarding long jobs to inflate the host scaling factor.

The lesson BOINC paid for over two decades: a self-reported cost is only trustworthy after **independent replication, outlier rejection, and bounding against history** — and even then it is approximate.

## Folding@home — benchmark baseline + Quick-Return-Bonus + passkeys

Folding@home ([foldingathome.org](https://foldingathome.org/)) takes a similar shape with its own anti-fraud additions:

- **Benchmark baseline.** Before issuing a project's work units, F@h benchmarks them on a dedicated reference machine; "the points for your system are relative to this benchmark machine; a faster system will get proportionately more points." *(The specific reference CPU has changed across F@h's history and is not pinned to a model in current docs — treat "the benchmark machine" as illustrative of the method, not a fixed spec.)*
- **Quick-Return-Bonus (QRB), 2010.** A non-linear bonus rewards donors who return work units rapidly *and consistently*. Qualifying requires a passkey, ≥10 returned bonus-eligible WUs, an ≥80% successful-return rate, and return before the timeout — an incentive structure that is *hard to fake without actually doing sustained, fast work*.
- **Passkeys — binding the claim to an identity.** A passkey "uniquely identifies you as an individual donor and is associated with results that you have completed." Its anti-cheating payoff: when fraud is detected, F@h zeroes out only the offender (and unkeyed users), not everyone sharing a username. This is the volunteer-computing version of **authenticating the producer of a cost claim** — the same lesson Gridcoin learned the hard way.

## Gridcoin — putting *token value* on externally-reported BOINC credit

Gridcoin ([gridcoin.us](https://gridcoin.us/)) is the cautionary apex: its **Distributed Proof of Research (DPOR)** mints a real, spendable cryptocurrency (GRC) in proportion to a user's *externally-reported BOINC credit*. The moment self-reported scientific work became spendable money, the unverified parts of the report became a direct theft vector.

Researchers at the **Chair for Network and Data Security, Ruhr-University Bochum** (Tobias Niemann, Juraj Somorovsky, Martin Grothe) demonstrated the attack in 2017. *(All figures and the quote below are verified against the discoverers' blog, [web-in-security.blogspot.com](https://web-in-security.blogspot.com/2017/08/gridcoin-good.html); the underlying work was presented at WOOT'17.)*

- **The attack.** An attacker extracted a victim's email and CPID (the identifier linking a researcher to their BOINC account) from the **public** blockchain, computed a valid CPID value over the current block hash, and minted blocks claiming the **victim's** Research Age while advertising the **attacker's** payout address — collecting DPOR rewards for work the victim performed. The credentials needed were sitting in the public record.
- **The flaw, verbatim:** *"It is important to note that the signature value is not part of the Merkle tree, and thus does not change the blockheader."* The reward-claim signature was appended to the block in a separate field but lived **outside the hashed blockheader** — so the signature did not actually bind the claim to an authenticated owner *inside* the committed, tamper-evident structure. Invariant #3, violated.
- **The damage.** Over **approximately three weeks** of testing, **nine illegitimate blocks** were minted, confirmed, and accepted into the live chain, wrongfully rewarding the attacker **72.4 GRC** with **zero BOINC work performed**. *(72.4 GRC / 9 blocks / ~3 weeks — confirmed verbatim against the discoverers' writeup.)*

**The lesson:** a spendable cost claim must be **cryptographically signed by, and bound to, the authenticated owner — and that signature must live *inside* the hashed event**, not bolted on outside the committed structure. And valuation must not depend on unbounded, replay-able external history that anyone can copy out of the public record.

## Synthesis for the reciprocity model

These three systems, read together, validate the precise shape the brainstorm arrived at for defending the `resource_vector` against credit-stuffing ([brainstorm "model challenges" #4](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md)):

1. **Directional crediting (the core fix).** Value work *I receive* by *my own* replacement cost — never the counterparty's self-reported number. This removes the credit-stuffing surface at its root: Gridcoin paid out *because* it trusted an externally-supplied figure. Myrhiza should trust only what it can re-derive.
2. **Kernel-side metering of *actual* consumption.** BOINC/F@h benchmark and re-measure rather than accept the volunteer's word. Myrhiza's kernel can meter real fuel/byte consumption (the brainstorm flags this as greenfield instrumentation), making the cost an *observed* quantity, not a *reported* one.
3. **Re-execution as a verification quorum.** BOINC's replication + outlier-drop is the volunteer-computing form of Myrhiza re-running a candidate event through the existing **`state-apply` dry-run** — the runtime already owns a deterministic re-execution primitive that can serve as the quorum cross-check.
4. **Sign the claim *inside* the hashed event.** Gridcoin's "signature not in the Merkle tree" is the exact mistake to avoid: any cost claim that becomes spendable standing must be signed and committed *inside* the event hash, bound to the producing peer.
5. **Anomaly-bound against history.** CreditNew caps a sample at ~10× the running average; the reciprocity ledger's decay/grace machinery should similarly refuse to let a single self-favorable claim spike standing.

This file is the verification-of-contribution counterpart to the unit-of-account open questions the deployed-reciprocity files raise but do not resolve: [`bittorrent.md`](bittorrent.md) (block-bytes is hard to fake *because the observer sees it*; arbitrary maintenance work is not) and [`ipfs-bitswap.md`](ipfs-bitswap.md) ("ledger exists ≠ ledger enforced"). See also [`open-problems.md` §2 + §5](open-problems.md) (gameable participation metric; collusion on useless work) and [`lessons.md`](lessons.md).

## Sources

- [BOINC project](https://boinc.berkeley.edu/) and [BOINC `CreditNew` wiki (GitHub mirror)](https://github.com/BOINC/boinc/wiki/CreditNew) — claimed-vs-granted credit, replication/quorum averaging, the "samples capped at X times the current average … maybe 10 for hosts, 100 for app versions" anomaly bound, cherry-picking/probation, host normalization. *(The canonical `boinc.berkeley.edu/trac/wiki/CreditNew` URL 404'd at access time; the GitHub wiki mirror carries the same content.)*
- [BOINC Credit System (Wikipedia)](https://en.wikipedia.org/wiki/BOINC_Credit_System) — cobblestone definition (200/day on a ~1 GFLOPS Whetstone reference) and the "top and bottom claimed credits dropped, average the rest" quorum form.
- [Folding@home — Points](https://foldingathome.org/faq/points/), [Determine points per platform](https://foldingathome.org/faqs/smp/determine-points-platform/), [Passkey purpose](https://foldingathome.org/faqs/what-is-the-purpose-of-a-passkey/), [QRB qualifications](https://foldingathome.org/faqs/points/bonus-points/what-are-the-qualifications-for-the-qrb/) — benchmark baseline, Quick-Return-Bonus (2010), passkey anti-cheating.
- [Niemann / Somorovsky / Grothe, "Gridcoin — The Good," On Web-Security and -Insecurity (Ruhr-University Bochum), Aug 2017](https://web-in-security.blogspot.com/2017/08/gridcoin-good.html) — the 72.4-GRC / 9-block / ~3-week DPOR attack and the "signature value is not part of the Merkle tree" quote (verified verbatim against this primary source). Companion: ["Gridcoin — The Bad"](https://web-in-security.blogspot.com/2017/08/gridcoin-bad.html) (a separate signature-verification flaw).
- Cross-references: [`bittorrent.md`](bittorrent.md), [`ipfs-bitswap.md`](ipfs-bitswap.md), [`open-problems.md`](open-problems.md), [`lessons.md`](lessons.md), [`reports/2026-05-29-reciprocity-economy-brainstorm/README.md`](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md).
