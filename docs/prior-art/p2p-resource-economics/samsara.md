**Date:** 2026-05-29
**Status:** active
**Subject:** Samsara (SOSP 2003) — fairness by forced symmetric storage claims; the replacement-cost precedent and the anti-comparative-advantage degenerate case.

# Samsara — fairness through symmetric storage claims

Samsara is the closest published precedent for the *crediting* half of Myrhiza's directional rule: **pay/credit in units of the same resource you would have to give up** (replacement cost), token-free, with no trusted third party. It is also a cautionary boundary marker — Samsara achieves fairness by *forcing symmetry*, which destroys the gains-from-specialization that the heterogeneous model exists to capture. It is the degenerate case the leading model deliberately steps away from.

## Citation

- Landon P. Cox and Brian D. Noble, "Samsara: Honor Among Thieves in Peer-to-Peer Storage," SOSP 2003, pp. 120–132 (also SIGOPS OSR 37(5)). Both authors at the University of Michigan, EECS. Verified against the SOSP 2003 PDF.

## The mechanism

Samsara enforces storage fairness **without** trusted third parties, symmetric storage *relationships* (in the sense of fixed pairings), monetary payment, or certified identities — explicitly the centralized-overhead features peer-to-peer storage is meant to avoid.

The core trick is the **storage claim**:

- When peer A asks peer B to store A's data, B may demand that A hold a **claim** in return — an **incompressible placeholder** of equivalent size. The claim is a block of (effectively) random bits A cannot compress away, so holding it genuinely costs A the storage it represents.
- This turns an asymmetric request ("store my data") into a **symmetric storage contract**: A consumes X bytes on B, so A must surrender X bytes of its own to B's claim. Each side periodically **challenges** the other to prove it still holds the agreed bytes; a peer that fails a challenge has its data dropped.
- Net effect: every node provides at least as much storage as it consumes. Fairness is structural, checkable locally, and needs no currency.

**Storage overhead.** In the simple case where a node never recycles its claims, Samsara's overhead is equal to the data stored — i.e. on the order of **~100% overhead** (you store your data *and* a same-sized claim for each peer holding it). The paper reduces this with **claim forwarding** (B can forward A's claim down a chain rather than physically holding it), trading storage for a higher risk of correlated failure along the chain.

## Why it is the replacement-cost precedent

Myrhiza's directional rule says: *value work I receive by my own replacement cost — what it would cost me to reproduce it.* Samsara is the literal storage instance of this. The "price" B charges A is not a token and not B's cost — it is **the same resource, in the same units, that B is giving up**: a byte of storage costs a byte of storage. The claim *is* the replacement cost, denominated in the resource itself. No exchange rate, no valuation function, no trust in a self-reported number — because the unit is identical on both sides.

That identity-of-units is exactly what makes it trust-minimal *and* exactly what makes it rigid.

## The anti-comparative-advantage degenerate case

Samsara is what the leading model collapses to **when there is only one resource and you force symmetry**:

- It works *because* storage-for-storage means there is nothing to value subjectively — a byte is a byte. There is no scope for "this is cheap for me but scarce for you," so no gains from trade.
- A peer with abundant disk but scarce bandwidth, and a peer with the opposite, **cannot** make a mutually beneficial uneven trade under Samsara: every contract is 1:1 in the same resource. Specialization is structurally impossible.
- This is the *point* of including Samsara: it shows the floor. Replacement-cost crediting in a single homogeneous resource is clean, verifiable, and token-free — but it is precisely the regime where comparative advantage is zero and the heterogeneous model has nothing to add. The leading model's value comes *entirely* from leaving this regime (multiple resources, subjective prices). Samsara marks where that value begins.

## Honest limits

- **Storage only.** Samsara prices the one resource where "give up an equivalent unit" is well-defined. CPU-ms and bandwidth are flows, not held bytes; you cannot make a peer "hold a claim" against compute it already spent. Replacement-cost crediting generalizes to flows only loosely.
- **~100% overhead** in the simple case; forwarding chains reduce it at the cost of fragility.
- **No social graph, no admission control, no heterogeneity.** Like OurGrid, Samsara is pure local reciprocity; the Sybil story is just the symmetric contract (a Sybil must surrender real storage per byte consumed, so multiplication buys nothing — a property worth noting alongside GNUnet's d ≤ c + ε).

## Implications for Myrhiza (framing-disclosed — see [`README.md`](README.md))

1. **Replacement-cost crediting is real and trust-minimal — in one resource.** Samsara validates the *form* of "credit received work at my own cost." Borrow the form; do not assume it generalizes across heterogeneous resources for free. See [`lessons.md`](lessons.md).
2. **Same-unit crediting needs no valuation function.** Where two peers exchange the *same* resource, Myrhiza needs no shadow prices at all — the unit is the value. Reserve the dot-product for *cross-resource* trades. This is the Coase firm-vs-market boundary in miniature; see [`open-problems.md`](open-problems.md) §4.
3. **Forced symmetry is the anti-pattern.** Samsara's symmetry is why it cannot exploit specialization. Myrhiza's whole reason for a resource vector + subjective prices is to *avoid* this — state explicitly that the model is the asymmetric generalization of Samsara.
4. **Challenge-response proves possession cheaply.** Samsara's periodic challenge to prove bytes are still held is a lightweight, token-free verification primitive worth borrowing for any "is the provider actually doing the work" check.

## Sources

- [Cox & Noble, "Samsara: Honor Among Thieves in Peer-to-Peer Storage," SOSP 2003, pp. 120–132](https://www.cs.rochester.edu/meetings/sosp2003/papers/p135-cox.pdf) — verified against PDF: incompressible-placeholder claims, symmetric contract, no trusted third party / no certified identities / no payment, periodic challenges, overhead = data stored, claim forwarding.
- [ACM record, DOI 10.1145/1165389.945458](https://dl.acm.org/doi/10.1145/1165389.945458) — venue.
- [dblp record](https://dblp.uni-trier.de/rec/conf/sosp/CoxN03.html) — authors, year.
- Cross-references: [`README.md`](README.md), [`open-problems.md`](open-problems.md) §1, §4, [`lessons.md`](lessons.md), [`reports/2026-05-29-reciprocity-economy-brainstorm/README.md`](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md).
