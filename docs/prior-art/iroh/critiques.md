**Date:** 2026-05-08
**Status:** active
**Subject:** Iroh — third-party critiques and honest assessments

# Critiques & honest assessments

A consolidation of substantive third-party criticism of Iroh from HN, Reddit/Lemmy, blog posts, GitHub issues, and the team's own honest assessments. Soft-pedals nothing. Where the critique is verbatim, it is preserved verbatim. The corpus is for Myrhiza spec authors who need to walk in eyes-open.

## Relay centralization

The most persistent external critique. Iroh's design tradeoff — *"a little centralization for a lot of effectiveness"* — is the team's own framing, but how that translates in practice draws regular pushback.

**The team's own statement** ([FAQ](https://docs.iroh.computer/about/faq), verbatim):

> *"There are four public relay servers run by number 0 (two in the US, one in Europe, and one in Asia), free to use for development and testing… Throughput through public relays is rate-limited."*

**Honest follow-up from rklaehn on HN** ([44706595](https://news.ycombinator.com/item?id=44706595)):

> *"The current approach in iroh is semi-centralized, and not fully p2p. Relay traffic is e2ee so the relays can't see traffic, but relays do have a list of nodeIDs and list of connections they're facilitating, which is privileged information, and many of their more serious users are using private relays to avoid exposing that information."*

This is the substantive critique stated by the team itself: the relay sees the *social graph* (which NodeID is talking to which NodeID, when, how much) even though the *content* is encrypted. For threat models that include "the relay operator is hostile" or "the relay is subpoenaed," this is a real leak.

**HN [44702251 — DrillShopper](https://news.ycombinator.com/item?id=44702251):**

> *"It'd be nice if the Getting Started link on the n0des page went here instead of immediately asking me to sign up before I know what the hell I'm signing up for."*

A UX critique that doubles as a positioning critique: the marketing of Iroh Services pushes users into the n0-managed funnel before they've understood that self-hosting is supported.

## "Why not libp2p?" — pure-P2P pushback

The flip side of the libp2p comparison: libp2p partisans see iroh as a step *backward* on the centralization axis. The team's framing again ([Comparing iroh & libp2p](https://www.iroh.computer/blog/comparing-iroh-and-libp2p), Jan 2024):

> *"Most p2p projects end up defaulting into a boil-the-ocean stance where they try to ship one of everything: a DHT, transports, pubsub, RPC. Sometime last year we realized it just wouldn't be possible to ship all this stuff with the team we had, so we picked the transport layer, and are focused on integrating with other projects."*

**HN [44383072 — b_fiive (n0 team)] on the same:**

> *"less configuration. more reliable. less pure p2p (iroh uses relays)"*

This is the team's honest one-line positioning. It is also the line a libp2p purist will quote back against them.

## Production-readiness perception

Through 2024-2025 the recurring critique was "this is pre-1.0, breaking changes happen often, can we use it in production?" The team eventually addressed it directly with the [1.0 roadmap post (Oct 28, 2024)](https://www.iroh.computer/blog/road-to-1-0), the [0.90 "Canary Series" (Jun 27, 2025)](https://www.iroh.computer/blog/iroh-0-90-the-canary-series), and 1.0.0-rc.0 (May 7, 2026). A user request quoted in the roadmap post, verbatim:

> *"Do the thing reliably, and don't break the API."*

The ~25 minor releases in 2024-2025 each carried breaking API changes; that pace is unusual for a load-bearing dep. By 1.0-rc the team is committing to API stability, but a Myrhiza spec author should expect at least one or two more breaking minor releases before the API truly settles. The ~6-month slip from "1.0 in H2 2025" to "1.0-rc in May 2026" is consistent with the team's history of estimating optimistically.

## Relay outage post-mortem (Nov 2024)

The [post-mortem](https://www.iroh.computer/blog/relay-down-a-post-mortem) is itself the critique. Verbatim selections:

> *"Rust's memory safety guarantees do not mitigate memory leaks."*

> *"Traffic anomalies went unnoticed for nearly 12 hours."*

> *"The Asia node was underpowered with no dynamic scaling mechanisms."*

A 12-hour global degradation of new-connection establishment, root-caused to a memory leak that filled disks with logs. The honest post-mortem is to the team's credit; the underlying fact is that the entire default deployment of every iroh app on the planet went degraded for half a day, and the operational rigor of n0's infrastructure is "small startup" rather than "Cloudflare." Spec authors should plan around it.

## API churn pre-1.0

**HN [45727557 — vlovich123](https://news.ycombinator.com/item?id=45727557):**

> *"This screams for a more ergonomic API like not making Connection cloneable or doing as_ref instead of Deref."*

**HN [45727557 — edbaskerville](https://news.ycombinator.com/item?id=45727557):**

> *"Connection is Clone, so in principle there is nothing stopping you from cloning the wrapped connection and losing the lifetime tracking."*

API ergonomics critiques continued through 2025-2026. The team's general response has been "we know, 1.0 will fix it." Whether 1.0.0-rc.0 (May 7, 2026) actually does is verifiable now, not later — Myrhiza spec authors should audit the post-1.0 API surface against these specific complaints.

**HN [45727557 — hovering_nox](https://news.ycombinator.com/item?id=45727557) — Windows defender / relay reachability:**

> *"It uses a third server to facilitate initial p2p connections but I keep loosing/fail to connect to this server… Windows Defender nukes this from orbit."*

A reminder that "P2P that requires a relay fallback" has the operational fragility of *any* server-dependent system: the server's TLS endpoint, IP, ports, and threat-detection signature can all fail you on user machines you don't control.

## "What about IPFS?" — ex-IPFS users

The pivot from IPFS draws two opposite reactions. The first is from former IPFS users *agreeing* that IPFS-as-shipped wasn't working.

**HN [39027630 — tux3 (IPFS user)](https://news.ycombinator.com/item?id=39027630):**

> *"past some point it just falls over. It just doesn't work outside of small-scale tests"*

**HN [39028532 — rklaehn (n0 team)](https://news.ycombinator.com/item?id=39028532):**

> *"Most iroh developers have been active in the ipfs community for many years and have shared similar frustrations"*

**HN [39033100 — rklaehn](https://news.ycombinator.com/item?id=39033100) on IPNS:**

> *"IPNS is that the performance is… not great… to put it politely, so it is not really an useful primitive"*

The second reaction is the legitimate counter-question: if iroh broke from IPFS, what part of "the IPFS dream" still survives in iroh's design vs which parts were dropped? The honest answer is: BLAKE3-verified streaming and content addressing survive (in `iroh-blobs`); the global content-addressed network (kubo's design vision) does not — iroh is point-to-point, not a public content-addressed CDN.

## "Forking QUIC?" — noq scrutiny

The team's decision to fork Quinn (the Rust QUIC implementation) into `noq` was announced 0.97 (Mar 2026). HN reception was largely positive; the most visible criticism is the upstreaming-debt concern.

**HN [47443588 — Aurornis](https://news.ycombinator.com/item?id=47443588):**

> *"They admit they don't have the time to go back and re-submit all of their work as tiny incremental patches… They estimate it would be on the order of 100 PRs necessary to break it up and get it reviewed."*

The team frames this as respectful divergence ("the quinn maintainers are really lovely people"), which is true. The structural fact is: the Rust QUIC ecosystem now has a fork that diverges fast, with a small team behind it, and the upstreaming pathway is unrealistic. For Myrhiza this means our QUIC is functionally a Number-0-curated fork of Quinn until and unless the changes flow back. Spec authors should track this dependency.

## Discovery — how do strangers meet?

Iroh's [DNS-based discovery](https://www.iroh.computer/blog/iroh-dns) (`pkarr` — public-key-as-resource-record on Mainline DHT) is a real solution for the "I know a NodeID, find its addresses" problem. It is **not** a solution for "I don't know a NodeID, how do I find one." That second problem is structurally outside iroh's scope.

**HN [44706595 — quote, anonymous](https://news.ycombinator.com/item?id=44706595):**

> *"In the iroh world, you dial another node by its NodeId, a 32-byte ed25519 public key…"*

The implication: NodeID exchange happens out of band (QR code, share link, copy-paste, sturdyref-style ticket). Iroh does not solve discovery in the "discoverability" sense, only in the "address resolution" sense. For Myrhiza this is a specific upper-layer problem we will inherit.

## Performance — published benchmarks

Iroh has published *some* benchmarks (the [QAD post](https://www.iroh.computer/blog/qad), the [BLAKE3 hashing post](https://www.iroh.computer/blog/hashing-multiple-blobs-with-BLAKE3)) but **does not publish a head-to-head benchmark suite vs libp2p, vs Hypercore, vs Tailscale, vs gRPC**. The closest claim, repeated in interviews and the FAQ:

> *"It's working in production and has managed 200k concurrent connections and millions of devices on the same network with low service costs."*  
> ([LambdaClass interview](https://blog.lambdaclass.com/the-wisdom-of-iroh/), April 9, 2025)

Concurrent connections at the network level is not the same as TPS, latency under contention, or memory per connection. A spec author committing hard to iroh should not assume the published numbers describe Myrhiza's worst case. Generate Myrhiza-specific benchmarks before locking the spec.

## API complexity for non-Rust users

**HN [44383970 — throw10920](https://news.ycombinator.com/item?id=44383970):**

> *"I've been wanting something like what Syncthing does for peer discovery for a while - something like this. Too bad it's written in such a low-level language."*

The Rust-first ecosystem is a real adoption barrier even though FFI bindings exist (Node.js since 0.23, Python in progress, mobile via UniFFI). If Myrhiza's app developers compile to WASM Components and embed iroh under capabilities, this concern partially evaporates — but not entirely; the host runtime still has to build, link, and operate the iroh stack on every supported platform.

## "Identity is just a public key" — portability gap

The team published [Lose your device, but keep your keys (Oct 2024)](https://www.iroh.computer/blog/frost-threshold-signatures), exploring **FROST threshold signatures** as a way to share a NodeID across devices and recover from key loss. As of May 2026 this is **research, not shipped product**. The default, today, is: a NodeID is a single ed25519 keypair on a single device, and losing the device means losing the identity.

This is not a critique of iroh as a transport — it is a critique of treating NodeID as application-level identity. For Myrhiza's identity model, "NodeID = identity" is the wrong simplification; we will need a layer above (recovery, multi-device, rotation) that iroh does not provide.

## Sybil — none, by design

There is no Sybil resistance in iroh. Anyone can spin up arbitrarily many NodeIDs on arbitrarily many machines. This is correct for a transport library — Sybil resistance is an application-level concern — but a spec author should explicitly note: an iroh-based Myrhiza inherits no global Sybil floor. This is the same situation OCapN finds itself in (per the Spritely critiques file), and Myrhiza will have to address it at the kernel-policy / membership-proof layer, not by leaning on iroh.

## "Iroh is so complex I gave up"

A real-world example surfaced in casual searches:

> *"It uses a third server to facilitate initial p2p connections but I keep loosing/fail to connect to this server… Windows Defender nukes this from orbit."* ([HN 45727557](https://news.ycombinator.com/item?id=45727557))

> *"I found Iroh so complex I gave up on it for a simple IPC project."* (paraphrased from web search results referencing developer experience reports)

Iroh is *simpler than libp2p* and *more complex than `tokio::net::TcpStream`*. For projects that don't actually need NAT traversal across hostile networks, the complexity floor of pulling in iroh + relay infrastructure may be too high. Myrhiza is squarely in the "needs NAT traversal" camp, so this is mostly not our problem — but we should expect iroh to be a heavier dependency than "just a Rust crate," with operational obligations attached.

## The team's own honest blog posts

**[Consensus is Impossible (Feb 21, 2025)](https://www.iroh.computer/blog/consensus-is-impossible)** — verbatim:

> *"All iroh protocols run up against these laws of distributed systems physics. Some examples: strictly speaking, iroh docs isn't a consensus protocol, it's a 'sync' protocol."*

The team will say out loud that iroh-docs is not a consensus protocol. This is the correct positioning, and it is the honest framing many of iroh's competitors do not adopt. For Myrhiza: trust the spec when it says "no consensus," and design state-apply on the assumption that cross-peer convergence is *eventual*, not *immediate*.

**[Async Rust Challenges in Iroh (Jul 31, 2024)](https://www.iroh.computer/blog/async-rust-challenges-in-iroh)** — the team has been candid about the difficulty of writing a robust async Rust library, the impedance mismatch between sync storage backends (`redb`) and async network code (`quinn`), and the leaks that come from misusing tokio. The post is essentially "writing this stack is hard and we got it wrong before we got it right." Rare and valuable.

## Net assessment for Myrhiza

The critiques are real but not disqualifying. The pattern:

1. **The team is honest about what they shipped and what they didn't.** Pivot post, relay outage post-mortem, "consensus is impossible" essay, async Rust struggles. This is unusual transparency for a venture-backed company; weight it.
2. **The centralization is genuine but bounded.** Default relays are n0-operated; private relays are first-class. Apps that care can self-host. The default deployment is a soft single-point-of-failure, not a hard one.
3. **The scope is honestly narrow.** Iroh does not solve discovery (in the social sense), Sybil resistance, identity portability, durability, or capability semantics. These all become Myrhiza's problems at a higher layer. That is the *correct* division of concerns; it is also a reminder that depending on iroh does not absolve us of solving the hard problems.
4. **The pre-1.0 API churn is real and ongoing.** Plan for at least one more breaking change after 1.0.0-rc.0 before the API truly settles.

## Sources

- [iroh — A new direction for iroh (Feb 17, 2023)](https://www.iroh.computer/blog/a-new-direction-for-iroh)
- [iroh — Comparing iroh & libp2p (Jan 5, 2024)](https://www.iroh.computer/blog/comparing-iroh-and-libp2p)
- [iroh — Consensus is Impossible (Feb 21, 2025)](https://www.iroh.computer/blog/consensus-is-impossible)
- [iroh — Async Rust Challenges in Iroh (Jul 31, 2024)](https://www.iroh.computer/blog/async-rust-challenges-in-iroh)
- [iroh — Relay outage post-mortem (Nov 19, 2024)](https://www.iroh.computer/blog/relay-down-a-post-mortem)
- [iroh — Lose your device, but keep your keys (Oct 2024)](https://www.iroh.computer/blog/frost-threshold-signatures)
- [iroh — Dial by NodeID, no address required (May 2024)](https://www.iroh.computer/blog/iroh-dns)
- [iroh — QAD: Moving from STUN to QUIC Address Discovery (Sep 2025)](https://www.iroh.computer/blog/qad)
- [iroh — Roadmap to 1.0 (Oct 28, 2024)](https://www.iroh.computer/blog/road-to-1-0)
- [iroh — The Canary Series (Jun 27, 2025)](https://www.iroh.computer/blog/iroh-0-90-the-canary-series)
- [iroh FAQ](https://docs.iroh.computer/about/faq)
- [LambdaClass — The Wisdom of Iroh (Apr 9, 2025)](https://blog.lambdaclass.com/the-wisdom-of-iroh/)
- [HN 33376205 — Iroh: A New Implementation of IPFS (Nov 2022)](https://news.ycombinator.com/item?id=33376205)
- [HN 39027630 — Iroh comparison to IPFS (Jan 2024)](https://news.ycombinator.com/item?id=39027630)
- [HN 44379173 — Iroh: A library to establish direct connection between peers (Jul 2025)](https://news.ycombinator.com/item?id=44379173)
- [HN 44702251 — relay default critique](https://news.ycombinator.com/item?id=44702251)
- [HN 44706595 — NodeID semantics, relay metadata leak](https://news.ycombinator.com/item?id=44706595)
- [HN 45727557 — iroh-blobs (Oct 2025)](https://news.ycombinator.com/item?id=45727557)
- [HN 47443588 — Noq: n0's QUIC fork (Mar 2026)](https://news.ycombinator.com/item?id=47443588)
- [Lobsters — Async Rust Challenges in Iroh](https://lobste.rs/s/7rtvnp/async_rust_challenges_iroh)
