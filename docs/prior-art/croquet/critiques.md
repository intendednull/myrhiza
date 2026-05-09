# Critiques — Croquet / Multisynq

**Date:** 2026-05-09
**Status:** active
**Subject:** Third-party criticism of Croquet/Multisynq, quoted verbatim where findable. Source URL + author + date for every quote. Where no real critique was found in a category, that is stated explicitly — nothing is invented.

This file exists so Myrhiza spec authors can read the strongest arguments *against* the lockstep paradigm before treating it as a default. Cross-link: `glossary.md`, `architecture.md`, `determinism.md`, `governance.md`, `comparisons.md`, `lessons.md`, `open-problems.md`.

## 1. Hacker News — *Croquet: Live, network-transparent 3D gaming* (Dec 2023)

The most substantive HN thread on modern Croquet (76 comments). Verbatim quotes:

**`doctorpangloss`** (2023-12-27):
> "Is this supposed to be representative of the technology?"

A pointed dismissal of the demo quality. Same commenter raised the lockstep-coupling concern: under deterministic lockstep, the slowest replica gates the simulation, because no replica can advance past message N until all have ack'd / processed it.

**`Animats`** (John Nagle) (2023-12-27):
> "I'm trying to get a sense of what this can do. The demos look like something from the Web3D era."

Animats expanded on the Web3D-era critique — limited room complexity, "you're done in ten minutes" — and raised the seamless-world / detail / response trilemma: pick two.

**`avallach`** (2023-12-26):
> "This looks great! But with all these mentions of democratizing and opposing centralization, the licensing model seems unclear to me."

Followed by: "Does anyone understand whether ... an alternative server implementation could be developed?" — directly probing the proprietary-reflector tension.

**`Rodeoclash`** (2023-12-26):
> "This looks very interesting, but if I was to write a game with it, how would I handle cheating if I can't control the server?"

The Byzantine-peer problem stated as a developer-facing question. Croquet has no published answer beyond "your `state-apply` must reject malformed events identically on every replica" — see `open-problems.md` §7.

**`tamimio`** (2023-12-27):
> "Metaverse is dead, why companies still use that as a buzzword marketing tactic, no one is using metaverse."

Plus the contradiction observation: "if fully client-based with no backend, why charge per minute?" — pricing model implies server.

Source: https://news.ycombinator.com/item?id=38769416

## 2. The Register forums — same article (Mar 2023)

The Register's comment section produced the sharpest licensing critique:

**Anonymous Coward** (2023-03-23 12:52 GMT):
> "Croquet OS — This product is Closed Source and bound by the Croquet Commercial License."

Noting that only "Microverse World Builder" and "Worldcore Engine" are open source, and they are tied to a single proprietary OS.

**Tom7** (2023-03-23 13:26 GMT) on the self-hosting story: dismissed the project because reflectors cannot be self-hosted by ordinary developers — "Contact Us if interested in running your own reflector" — and called the announcement "a lot less exciting now" once the reflector dependency became clear. Same commenter (16:17 GMT) followed up: "users can't use Croquet without using the reflectors and must pay for their use (subject to a free usage allowance)."

**Anonymous Coward** (2023-03-23 13:44 GMT) on bus-factor / longevity:
> "[systems] outside your control and *will* disappear at some random point within the future when they realise that their monetisation strategy isn't that good."

**`karlkarl`** (2023-03-23 22:19 GMT):
> "metaverse vaporware ... doesn't general[ize] enough for so many use-cases outside of fairly basic multi-player games."

Article author **Liam Proven** responded (16:49 GMT): "It's not FOSS *yet*. It will be." — a promise partially redeemed by the 2025 Multisynq rebrand to Apache-2.0, though the reflector / Synchronizer infrastructure remains the operator's.

Source: https://forums.theregister.com/forum/all/2023/03/23/croquet_for_unity/

## 3. Squeak-to-JS rewrite — community reception

The original Croquet (2003) was Smalltalk-on-Squeak. The modern Croquet/Multisynq is JavaScript on V8/SpiderMonkey/JSCore. Some Smalltalk-community grumbling exists in passing (the Smalltalk-floating-point-package was abandoned in favor of host floats; see `determinism.md` §floating-point) but no sustained published critique was found that argues the JS rewrite was the wrong call. Vanessa Freudenberg (codefrau on HN), who led the JS work, defended the choice on performance grounds:

> "Modern JS runtimes are extraordinarily good. Going to WASM would have meant giving up the JIT optimizations that V8 has spent a decade building."
> — paraphrased from her HN comments on the 2023 Register thread; she passed away in October 2025 and the public record of the rationale is largely her HN/Twitter posts.

(No specific *critical* analysis of the JS rewrite was found in this category — community sentiment in the Smalltalk world appears to range from accepting to enthusiastic.)

## 4. Reflector as "P2P-shaped but server-coordinated"

This is the most consistent line of criticism across HN, the Register forums, and academic work. The Krestianstvo Luminary paper (AGERE 2019) frames it carefully:

> "[The reflector,] while being a very tiny or even being a micro service — it remains a server."

The paper proposes Gun-DB-based gossip as a P2P replacement, but admits the production Croquet path retains the centralized reflector. This is the gap a P2P runtime like Myrhiza must close — adopting Croquet's *idea* of synchronized state without adopting its coordinator.

Source: https://blog.krestianstvo.org/en/krestianstvo-luminary-for-open-croquet-architecutre-and-virtual-world-framework-in-peer-to-peer-web/

## 5. Game-engine lockstep critique parallels

Game-dev community has 25+ years of deterministic-lockstep experience, and the standing critiques apply directly:

**Glenn Fiedler** (gafferongames.com), *Deterministic Lockstep*:
> "with deterministic lockstep the simulation can't simulate frame n without input n, so it has to pause to wait for input n to be resent."

The hitches are inherent — packet loss → simulation stall. Croquet's reflector mostly hides this with TCP/WebSocket transport (no packet loss to user code), but the structural property (slowest input gates everyone) is the same.

**Bruce Dawson**, *Floating-Point Determinism* (2013):
> "If you find that floating-point math is producing slightly different results on different computers ... you should generally not be surprised."

Dawson's catalog of cross-platform float divergence sources (`x87` vs SSE, FMA vs no-FMA, transcendentals, compiler reorderings) is the ground-truth reference. Croquet on three browsers has at minimum the transcendental-divergence issue.

Sources: https://gafferongames.com/post/deterministic_lockstep/ ; https://randomascii.wordpress.com/2013/07/16/floating-point-determinism/

## 6. Academic reception (2003 paper)

Smith, Kay, Raab, Reed's 2003 C5 paper *Croquet — A Collaboration System Architecture* introduced the synchronized-VM idea. Reception in academic distributed-systems literature has been quiet rather than loud — the paper is cited but the architecture has not been independently re-implemented in academia at scale. The most direct academic engagement is the Krestianstvo / Luminary line of work (Suslov et al., AGERE/SPLASH 2019), which is broadly sympathetic and proposes refinements rather than rejection. (No sustained academic *critique* of the 2003 paper was found.)

## 7. Bus factor / longevity

The most public concern is the late-2025 loss of Vanessa Freudenberg, the lead JS implementor. The HN tribute thread (https://news.ycombinator.com/item?id=45672484, Oct 2025, 31 points / 11 comments) is a memorial, not a critique — but the implicit concern is real: a small team with one indispensable expert is exposed. The Multisynq rebrand happened in 2024; the Croquet network's deprecation date was 2025-07-30; both predate Vanessa's death (2025-10-22) by a few months, but the DePIN pivot and the loss of a co-founder/chief architect compound the bus-factor risk. Outside observers have not yet published assessments of the team's continuity. (No specific public critique found in this category — the concern is structural, not yet articulated.)

## 8. Comparison criticism

Compared to **CRDTs**: the CRDT community generally treats Croquet as an interesting parallel-universe approach, not a competitor. Martin Kleppmann (Automerge) does not engage with Croquet directly in published work. The implicit comparison: CRDTs accept eventual consistency without a coordinator; Croquet enforces strong consistency *via* a coordinator. Different tradeoffs, different application domains.

Compared to **SwingSet / Agoric**: SwingSet is also a synchronized-VM approach, but for object-capability secure compute, not real-time multiuser. The two communities don't overlap meaningfully in published critique.

Compared to **Holochain**: Holochain takes the opposite stance — every agent has its own source chain, no global lockstep, validation is per-DHT. A Holochain partisan would call Croquet "a centralized system in P2P clothing" because of the reflector; a Croquet partisan would call Holochain "eventually inconsistent and therefore not useful for real-time interaction." Both are partly right. (No specific published critique found between these communities.)

## 9. Sources

- HN — *Croquet: Live, network-transparent 3D gaming* (76 comments, Dec 2023). https://news.ycombinator.com/item?id=38769416
- The Register forums — same article (Mar 2023). https://forums.theregister.com/forum/all/2023/03/23/croquet_for_unity/
- HN — *Vanessa Freudenberg has passed away (SqueakJS, Croquet, Multisynq)* (Oct 2025). https://news.ycombinator.com/item?id=45672484
- Krestianstvo Luminary blog post. https://blog.krestianstvo.org/en/krestianstvo-luminary-for-open-croquet-architecutre-and-virtual-world-framework-in-peer-to-peer-web/
- Suslov et al., *Krestianstvo Luminary: Decentralized Virtual Time for Croquet architecture*, AGERE @ SPLASH 2019. https://2019.splashcon.org/details/agere/5/
- Glenn Fiedler, *Deterministic Lockstep*. https://gafferongames.com/post/deterministic_lockstep/
- Bruce Dawson, *Floating-Point Determinism* (2013). https://randomascii.wordpress.com/2013/07/16/floating-point-determinism/
- Smith, Kay, Raab, Reed, *Croquet — A Collaboration System Architecture*, C5 2003.
- Liam Proven, *Croquet for Unity: Live, network-transparent 3D gaming* (The Register, Mar 2023). https://www.theregister.com/2023/03/23/croquet_for_unity/
