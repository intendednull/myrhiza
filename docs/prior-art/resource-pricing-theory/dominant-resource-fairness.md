**Date:** 2026-05-29
**Status:** active
**Subject:** DRF — the load-bearing challenge to priced summation. The *Asset Fairness* counterexample proves "price every resource and sum" violates sharing-incentive; DRF allocates over a resource vector with no prices and shipped at scale.

# Dominant Resource Fairness — why the dot-product is not the allocation rule

## The paper

Ghodsi, Zaharia, Hindman, Konwinski, Shenker & Stoica, **"Dominant Resource Fairness: Fair Allocation of Multiple Resource Types,"** *8th USENIX Symposium on Networked Systems Design and Implementation (NSDI 11)*, March 2011. (Authors, title, venue, year verified off the primary NSDI PDF at usenix.org.) DRF is the multi-resource fairness mechanism that shipped in **Apache Mesos** and **Hadoop YARN** — i.e., it is *deployed at scale*, not research-grade. It is the single most load-bearing challenge to `value_P = resource_vector · shadow_prices`, because the paper's own *Asset Fairness* baseline **is** that dot-product, and the paper proves it inferior.

## The four properties

DRF is derived from four properties the authors argue any multi-resource allocator should satisfy (quoted/paraphrased from §3 of the primary PDF):

1. **Sharing incentive** — each user should be no worse off than under a static equal partition: with `n` users on identical nodes, a user should not be able to allocate more tasks in a `1/n` partition of the cluster than under the shared policy.
2. **Strategy-proofness** — "users should not be able to benefit by lying about their resource demands." (Exact wording read off the primary PDF — a prior research pass found an HTML mirror garbled this; the primary text is "benefit by lying about their resource demands," providing incentive compatibility.)
3. **Envy-freeness** — no user prefers another user's allocation.
4. **Pareto efficiency** — cannot increase one user's allocation without decreasing another's.

DRF's mechanism: each user's **dominant share** is the largest share-of-total it holds across resources (a CPU-bound job's dominant resource is CPU; an I/O-bound job's is bandwidth). DRF applies **max-min fairness to the dominant shares** — *equalize the smallest dominant share*. Crucially, this uses **no prices and assumes no commensurability** between resources. It never asks "how many GB equals one CPU." It works on the *vector* directly.

## Asset Fairness — the dot-product, named

The paper's *Asset Fairness* baseline is exactly the priced-summation construction. From §5.1 (primary PDF): "The idea behind Asset Fairness is that equal shares of different resources are worth the same … 1% of all CPUs [is] worth the same as 1% of memory." It assigns a price to each resource and **equalizes the aggregate dollar value** allocated to each user — `Σ_j (price_j · share_{i,j})`, i.e. `resource_vector · price_vector` with a *shared* price vector. **This is structurally `value_P = resource_vector · shadow_prices` used as an allocation rule.**

### The §5.1 pricing illustration (numbers verified off primary PDF)

In the running example, the cluster has 9 CPUs and 18 GB RAM. Because RAM is twice as plentiful, "one CPU is worth twice as much as one GB of RAM." Setting **1 GB = \$1 and 1 CPU = \$2**: user A's task demand makes A spend **\$6 per task** and user B spend **\$7 per task**. The asset-fair LP (`max(x,y)` s.t. CPU `x+3y ≤ 9`, mem `4x+y ≤ 18`, equal-spend `6x = 7y`) yields `x = 2.52`, `y = 2.16`. **These exact \$2/\$1 and \$6/\$7 numbers are present in the primary NSDI PDF §5.1** — but they are the *pricing illustration*, not the sharing-incentive proof. (The reciprocity brainstrom's gap analysis attached "B ends up worse off" to these numbers; the primary PDF proves the violation with a *different, separate* example, below. Both are real; they are two different examples in the paper.)

### The Theorem 1 sharing-incentive violation (numbers verified off primary PDF)

The actual proof that Asset Fairness **violates sharing incentive** is **Theorem 1 (§6.1.1, Figure 5)**, and it uses a *different* example with **no dollar prices at all**:

> Two users, total resources `⟨30, 30⟩`, demand vectors `D₁ = ⟨1, 3⟩` and `D₂ = ⟨1, 1⟩`. Asset Fairness allocates user 1 six tasks and user 2 twelve tasks. User 1 receives `⟨6, 18⟩`; user 2 uses `⟨12, 12⟩`. Each gets an equal aggregate share (24/60), **but user 2 gets less than half (12 < 15) of *both* resources** — so user 2 would be strictly better off statically partitioning the cluster and owning half the nodes. Sharing incentive is violated.

**This is the load-bearing result.** "Price every resource and sum, then equalize value" can leave a user worse off than just taking a fixed equal slice — the exact outcome a reciprocity economy is trying to prevent. DRF, on the same inputs, does *not* do this, because it equalizes dominant *shares* rather than priced sums.

The paper's summary table (§6.1, Table 2, verified): **Asset Fairness satisfies strategy-proofness, envy-freeness, Pareto efficiency, single-resource fairness, population monotonicity — but FAILS sharing incentive, bottleneck fairness, and resource monotonicity. DRF satisfies all of the first group plus sharing incentive and bottleneck fairness** (DRF, like all of them, lacks resource monotonicity — the paper proves *no* policy can have resource monotonicity without sacrificing sharing incentive or Pareto efficiency).

## Non-substitutability — the Leontief point

DRF assumes users have **Leontief (fixed-proportions) demands**: a task needs CPU *and* RAM *and* bandwidth in a fixed recipe, and extra of a non-bottleneck resource has *zero* marginal value. A CPU-blocked job gains nothing from more RAM. This is the realistic model for running a module on a peer's behalf — and it is precisely the case a **fixed price vector misprices**: priced summation values the slack (cheap) resources a job happens to touch, inflating or deflating the cost relative to the one resource that actually *binds*. Production clouds bundle prices (you rent an *instance type*, a fixed CPU+RAM recipe, not à-la-carte resources) for exactly this reason. The reciprocity model's mitigation (brainstorm #2): price against the **binding** resource (a max-over-resources, dominant-resource flavor) or carry the vector and reconcile per-resource, rather than collapsing to a scalar dot product prematurely.

## DRFH — heterogeneous servers

Wang, Liang & Li, **"Multi-Resource Fair Allocation in Heterogeneous Cloud Computing Systems,"** *IEEE Trans. Parallel and Distributed Systems* **26**(10):2822–2835, 2015 (conference version: INFOCOM 2014; preprint arXiv:1308.0083). **DRFH** generalizes DRF from a single server to a pool of *heterogeneous* servers (different CPU/RAM/storage configs — the paper uses real Google cluster traces). It preserves envy-freeness, Pareto efficiency, and a coalition-resistant strategy-proofness ("no coalition of misreporting users can benefit all its members"). The relevance to Myrhiza: peers *are* heterogeneous servers, so DRFH — not vanilla DRF — is the closer structural analogue if a dominant-resource allocation rule is adopted. (Note: the seed citation's author list "Wang/Liu/Li" is **wrong**; the authors are **Wang, Liang, Li** — there is no "Liu." Verified off the TPDS PDF.)

## Implications for Myrhiza

- **The dot-product is fine as a cost/credit *metric*; it is unsafe as the *allocation/fairness* rule.** Asset Fairness *is* the dot-product as an allocation rule, and it provably violates sharing incentive — the property a reciprocity economy most needs.
- **Pair the metric with a dominant-resource bottleneck rule for allocation.** Use `value_P = resource_vector · shadow_prices_P` to credit/debit the per-peer ledger; use a DRF/DRFH-style dominant-resource max-min rule to decide *what to actually serve* under contention (the Open-fork-#6 enforcement side).
- **DRF shipped at scale with no prices and no commensurability** — strong evidence that priced summation is neither necessary nor the fairest way to allocate a resource vector. Default to share-based; reserve pricing for the cross-trust-boundary crediting where Myrhiza wants comparative-advantage gains.

## Sources

- [Ghodsi, Zaharia, Hindman, Konwinski, Shenker & Stoica, "Dominant Resource Fairness: Fair Allocation of Multiple Resource Types," NSDI 2011](https://www.usenix.org/legacy/events/nsdi11/tech/full_papers/Ghodsi.pdf) — **primary PDF; all property definitions, the §5.1 \$2/\$1 & \$6/\$7 pricing illustration, the Theorem 1 `⟨30,30⟩`/`D₁=⟨1,3⟩`/`D₂=⟨1,1⟩` sharing-incentive counterexample, and Table 2 read off it directly.**
- [Wang, Liang & Li, "Multi-Resource Fair Allocation in Heterogeneous Cloud Computing Systems," *IEEE TPDS* 26(10):2822–2835, 2015](https://www.comm.utoronto.ca/~liang/publications/TPDS_DRFH.pdf) (preprint [arXiv:1308.0083](https://arxiv.org/abs/1308.0083)) — author order verified off PDF.
- Cross-refs: [`network-utility-maximization.md`](network-utility-maximization.md), [`open-problems.md`](open-problems.md), [`lessons.md`](lessons.md), [reciprocity brainstorm §"What the leading `value()` must answer"](../../reports/2026-05-29-reciprocity-economy-brainstorm/README.md).
