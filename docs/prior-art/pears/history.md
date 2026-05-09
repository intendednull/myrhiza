**Date:** 2026-05-09
**Status:** active
**Subject:** Pears / Holepunch — Chronology from Dat 2013 through Pear Runtime 2026

# History

The Pears stack is older than it looks. The visible "Pear Runtime" is from
2024; the data-layer code underneath it is from 2013. Mistaking the runtime's
youth for the protocol's youth is the most common error people make about this
stack.

This chronology is the load-bearing one for understanding *what is mature*
(Hypercore, Hyperswarm, Hyperdrive — 10+ years), *what is recent* (Bare from
late 2022, Pear from early 2024), and *what is closed* (Keet's mobile and
desktop binaries from 2022 onward).

## 2013 — Dat Project Founded

[`datproject/dat`](https://github.com/datproject/dat) repo created
2013-06-27, BSD-3-Clause, by **Max Ogden**, **Karissa McKelvey**, and
**Mathias "mafintosh" Buus**. Initial grant funding from the **Sloan
Foundation** via **Code for Science & Society** (a 501(c)(3) US non-profit).
Initial framing: science-data versioning and distribution — a "git for data"
positioned at the open-data and research-reproducibility community.

The Hypercore append-only-log primitive does not yet exist as a separate
project; it lives inside the `dat` codebase as the underlying storage
abstraction.

## 2014–2017 — Hypercore Modularization

Two milestones from this period:

- **2015-11-18** — `hyperdrive` repo created on GitHub. The filesystem-as-app
  abstraction is split out as its own npm package.
- **2015-12-20** — `hypercore` repo created. The append-only-log primitive
  becomes a standalone module. The `v1.1.0` tag from 2015-12-21 is the
  earliest published release. (Tag history visible via
  `gh api repos/holepunchto/hypercore/tags`.)

By 2017 the architecture has settled: Hypercore = append-only log, Hyperdrive =
filesystem layer over Hypercore, Discovery via DNS+DHT-based mechanisms that
will later become Hyperswarm.

The 2017 [Dat whitepaper](https://github.com/datprotocol/whitepaper) (now
archived) is the last formal protocol specification of the Dat-era stack.
Updates after that are tracked through the Dat Enhancement Proposals
([DEPs](https://github.com/dat-ecosystem-archive/DEPs)).

## 2018 — Hyperswarm Split Out

`holepunchto/hyperswarm` repo created **2018-09-17** (during the Dat-era; the
repo was later transferred into `holepunchto`). The DHT-based peer discovery
becomes its own project, separable from the data layer. This is the split
that lets the discovery layer evolve independently of Hypercore versioning.

## 2019–2020 — Dat Project Decline

The Sloan-funded incarnation runs out of runway. Code for Science & Society
restructures; the `datproject/dat` repository's commit cadence drops sharply
from late 2019 onward. The last meaningful merge to `master` is the
`removed request from dev dependencies` commit (`b5fa5a6`) from 2021-07-21;
post-2021 commits are README-only.

The community archives most of the surrounding tooling under
`dat-ecosystem-archive`. `datprotocol/whitepaper` is later (2024-03-02)
explicitly archived with a "DEPRECATED" note pointing readers to the active
ecosystem at `dat-ecosystem.org` and the `holepunchto` GitHub org.

## 2020-11-06 — Autobase Created

`holepunchto/autobase` repo created. This is the multi-writer linearization
primitive that turns the single-writer Hypercore into something capable of
group-edited state. **It pre-dates Holepunch as a public entity.** Read this
as evidence that the technical work was already moving in mafintosh's hands
before the corporate vehicle existed.

## 2021-03-25 — `holepunchto` GitHub Org Created

The new commercial vehicle's public footprint appears. Existing Hypercore /
Hyperswarm / Hyperdrive repos are moved or re-homed here over the following
months. The org description today reads: **"The Peer to Peer Company"**.

## 2022 — Tether Funding Announcement, Keet Launch

Tether publicly announces its investment in Holepunch in early 2022 [paraphrased — verbatim quote not findable in the GitHub-accessible artifacts]. The
investment is positioned as funding for peer-to-peer communication and
financial-application infrastructure; Keet is announced as the flagship
demonstration application.

The Keet client itself is closed-source from this point onward. The
infrastructure libraries (`hypercore`, `hyperswarm`, `hyperdrive`,
`autobase`, `protomux`) remain open-source.

The visible seam between Tether and Holepunch shows up later in places like
`holepunchto/electron-forge-maker-flatpak/README.md`, where the Flatpak
manifest lists the developer as `Tether Data S.A. de C.V.` — i.e., Tether's
operating entity, not Holepunch Inc. This is the on-paper relationship.

## 2022-08-15 — Hypercore v10

`hypercore@10.0.0` tagged 2022-08-15 (commit `80025e7`). The
README explicitly notes: *"Version 10 is not compatible with earlier versions
(9 and earlier), but is considered LTS, meaning the storage format and wire
protocol is forward compatible with future versions."* — i.e., earlier
Hypercores need to be re-encoded; future ones won't.

This is a load-bearing breaking change for any application that stored
Hypercore data prior to 2022. Dat-era cores need migration. See `critiques.md`
on protocol-version churn.

## 2022-12-18 — Bare Runtime Begins

`holepunchto/bare` repo created. The earliest commit dates to 2022-12-18.
Bare is positioned as a small, embeddable JavaScript runtime — Node-shaped
but mobile-first, with explicit support for iOS, Android, macOS, Linux, and
Windows. Architecturally it sits on `libjs` (Holepunch's V8 abstraction
layer) and `libuv` (the Node.js async I/O loop).

This is the substrate that lets a Hypercore-based app run on a phone. Before
Bare, the only realistic deployment surface for the JS-based Hyperstack was
desktop / Electron. After Bare, the same code runs in mobile binaries.

## 2024-02-03 — Pear Runtime Repo Created

`holepunchto/pear` created 2024-02-03 (commit `~08:01:02Z` UTC). Pear ties
together: Bare (the runtime), Hyperdrive (the application distribution
medium), Hyperswarm (the network), and a CLI / lifecycle layer. An "app" in
Pear is a JS project published into Hyperdrive and addressed by a public-key
URL of the form `pear://<key>`.

The repo's `CHANGELOG.md` shows continuous shipping from this point: the
current release (as of 2026-05-06) is `v2.6.5`. The CLI gets `pear stage`,
`pear seed`, `pear run`, `pear build`, and `pear multisig` for production
release coordination over the next two years.

## 2024-09-17 — Keet Mobile Releases Repo

`holepunchto/keet-mobile-releases` created 2024-09-17. This is the public
artifact distribution path for Keet's iOS and Android binaries. The repo
contains releases (no source). The most recent published release at time of
writing is `4.14.0` (2026-04-29) — visible in both the GitHub release page
and the iOS App Store listing for `io.keet.app`.

iOS App Store metadata for Keet (verified via `itunes.apple.com/lookup`):

- Track name: **Keet — Private Encrypted Chat**
- Bundle ID: `io.keet.app`
- Seller: **Holepunch Inc**
- First released: 2023-01-30
- Current version: 4.14.0 (2026-04-29)
- Average user rating: **4.59** across **99 ratings**

The 99-rating count is small for a 3+ year old app — Keet is shipped, but at
a community-scale userbase, not a mass-market one. See `critiques.md` for the
"shipping but how widely?" question.

## 2025-01-13 — Hypercore v11

`hypercore@11.0.0` tagged 2025-01-13 (commit dated 2025-01-13T21:07:51Z).
Another major-version breaking change — v10 stores need migration to v11.
The v10 LTS promise from 2022 was *forward-format-compatible*, not
*upgrade-free*. By v11.30.1 (2026-05-06) the line has stabilized at >100
patch releases.

This is the second major migration the userbase has had to absorb in three
years. Specs that depend on Hypercore should plan for a v12 transition before
2028 — not as a prediction, but as a base rate.

## 2024–2026 — Pear Runtime Stabilization

Continuous-shipping cadence visible in commits and changelog. Notable
trajectory items:

- `pear-runtime` extracted from `pear` as an embeddable module (2024
  mid-year per CHANGELOG).
- `pear multisig` added (v2.5.0+) for coordinated production releases —
  i.e., multiple parties have to sign before a published key updates the
  app artifact. This is the closest thing the stack has to a release-control
  story.
- `pear build` for multi-architecture deployment folders (v2.5.0).
- `pear install` (v2.6.x, May 2026 — very recent).

The runtime is shipping but still evolving its CLI surface in 2026. Read
this as "stabilizing", not "stable".

## 2026-05-09 — Current State

Snapshot at time of writing (verified via GitHub API):

- `holepunchto/pear` — Apache-2.0, 241 stars, last updated 2026-05-06
- `holepunchto/bare` — Apache-2.0, 1072 stars, last commit 2026-05-06
- `holepunchto/hypercore` — MIT, 2794 stars, latest release v11.30.1 (2026-05-06)
- `holepunchto/hyperdrive` — Apache-2.0, 1986 stars
- `holepunchto/hyperswarm` — MIT, 1261 stars
- `holepunchto/autobase` — Apache-2.0, 145 stars
- Holepunch org: 617 public repos
- Keet iOS: v4.14.0 shipped 2026-04-29, 4.59★ / 99 ratings

## What This Chronology Tells You

1. **The data-layer is mature.** Hypercore is older than Kubernetes. Treat the
   protocol's correctness story as load-bearing-tested by 10+ years of use.
2. **The runtime layer is young.** Pear is two years old. Bare is three.
   Treat their API surfaces as still settling.
3. **There has been a governance discontinuity already.** Dat → Holepunch was a
   working transition for the *code*; not a frictionless one for the
   *community*.
4. **The flagship app is closed-source.** Keet's existence proves that
   consumer-mobile shipping on this stack is *possible*; it does not prove
   that other teams can replicate the recipe with what's open.

## Cross-references

- [governance.md](./governance.md) — the Dat → Holepunch transition as
  governance precedent
- [pear-runtime.md](./pear-runtime.md) — current Pear state in detail
- [hypercore-stack.md](./hypercore-stack.md) — the data-layer protocol depth
- [bare-runtime.md](./bare-runtime.md) — the mobile substrate
- [keet-and-apps.md](./keet-and-apps.md) — the app ecosystem
- [critiques.md](./critiques.md) — protocol-version churn details
