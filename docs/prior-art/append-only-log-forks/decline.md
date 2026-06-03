**Date:** 2026-05-29
**Status:** active
**Subject:** SSB's decline and fragmentation — honest framing of a deployed-but-fading system

# SSB shipped, was used, and is now fading

This folder treats SSB as **research-grade-but-deployed** prior art. The
"deployed" half is real — SSB had working clients and active communities for
years. The honest half is that the ecosystem is **fragmenting and in decline**,
and a Myrhiza spec author should weigh its lessons knowing that.

## Timeline of the wind-down

| When | Event |
|---|---|
| 2014 | Dominic Tarr creates SSB; offline-first signed feeds |
| 2016– | André Staltz starts Manyverse (mobile SSB client) |
| 2019 | Academic paper at ACM ICN (Tarr, Lavoie, Meyer, Tschudin — note: Aljoscha Meyer, the Bamboo/Willow author, is a co-author) |
| ~2019–2021 | Multiple flagship clients: Patchwork (desktop), Manyverse (mobile), Planetary (iOS), Patchbay, Oasis |
| ~2021 | **Patchwork deprecated** (v3.18.1 the last release) — README: "Patchwork makes some architectural decisions that make it hard to maintain, and even harder for *new* developers to get into the codebase"; "it makes more sense to deprecate Patchwork and focus our efforts on projects like #oasis or #manyverse"; users pushed to Oasis / Manyverse |
| 2023 | **Planetary team pivots to Nostr**, building `nos.social`; active Planetary development ends; cloud services left running "as long as they're able" |
| 2024-04-05 | **André Staltz's "last update"** — ends active work on SSB, Manyverse, and the experimental successor protocol **PPPPP** after ~7 years / ~6,400 hours: "my time to build up SSB, Manyverse, and the new PPPPP protocol is over." Manyverse handed to Jacob (with Mix Irving's support) |

## What killed momentum (the structural reasons)

These are the durable lessons, not gossip — each is a thing Myrhiza must not
repeat:

- **The fork problem was never solved in production.** A decade of deployment and
  the canonical recovery story for a forked feed remained "your feed is
  corrupted." Multi-device support — the *accidental* fork cause — was a
  perennial open issue. This is the exact gap 2P-BFT-Log
  ([fork-proof-construction.md](fork-proof-construction.md)) fills and the exact
  gap Myrhiza §4.4.1 currently shares.
- **Replicate-everything didn't scale.** Classic SSB pulled whole feeds of all
  followed identities; performance degraded as graphs grew. Meta-feeds
  ([meta-feeds.md](meta-feeds.md)) and Bamboo
  ([bamboo-lipmaa-links.md](bamboo-lipmaa-links.md)) were the late attempts to
  fix this, but they landed after the ecosystem had begun fragmenting.
- **Format churn and stack fragmentation.** Classic feeds, Bendy Butt, meta-feeds,
  Bamboo, then PPPPP — successive formats, none cleanly migrating the installed
  base, each splitting effort. (Compare Pears' Hypercore version churn,
  [pears/critiques.md](../pears/critiques.md).)
- **Thin specs, JS-centric implementations.** Like the Hypercore stack,
  re-implementing SSB outside JS meant reading source, not an RFC — including the
  V8-`JSON.stringify` signing surface ([ssb-feed-format.md](ssb-feed-format.md)).
- **Volunteer / single-maintainer concentration.** Patchwork's README describes
  "multiple iterations of developers coming in, trying to change things in a
  structural way, then burning out on it"; Manyverse was effectively one person
  for years. When the key maintainer's priorities shifted, momentum collapsed.
  (Same governance smell as `pears/governance.md` single-vendor concentration.)
- **Competing destinations matured.** By 2023–2024 Mastodon, Bluesky/atproto, and
  Nostr offered easier on-ramps; the Planetary team and others migrated to Nostr.

## What this means for the corpus's credibility

The borrows in [lessons.md](lessons.md) are **not** "do what the winners did" —
SSB is not a winner. They are: SSB *proved the data shape works* (a decade of
real use), and SSB *proved which problem will bite you if unsolved* (forks,
scaling, format migration). The single strongest borrow — the irrefutable fork
proof — comes from **2P-BFT-Log, the academic follow-on**, precisely because
deployed SSB never produced it. Read the decline as evidence of *what to
prioritize*, not as a reason to dismiss the lineage.

## Sources

- Manyverse "My last update" — [manyver.se/blog/2024-04-05](https://www.manyver.se/blog/2024-04-05/) (Staltz ends work; PPPPP; handoff).
- "Pivoting Protocols, from SSB to Nostr" — [nos.social/blog/pivoting-protocols](https://www.nos.social/blog/pivoting-protocols) (Planetary → Nostr).
- Patchwork deprecation — [github.com/ssbc/patchwork](https://github.com/ssbc/patchwork) (README / releases).
- SSB academic paper — Tarr, Lavoie, Meyer, Tschudin, ACM ICN 2019 ([dl.acm.org/doi/10.1145/3357150.3357396](https://dl.acm.org/doi/10.1145/3357150.3357396); [conference PDF](https://conferences.sigcomm.org/acm-icn/2019/proceedings/icn19-19.pdf)).
- Secure Scuttlebutt — [Wikipedia](https://en.wikipedia.org/wiki/Secure_Scuttlebutt).
