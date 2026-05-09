**Date:** 2026-05-09
**Status:** active
**Subject:** Pears / Holepunch — Governance, license, contribution model, single-vendor risk

# Governance

This file is the consult-when-you-care-about-stewardship layer of the Pears
prior-art set. The technical artifacts are mature; the governance around them
is unusually concentrated. Myrhiza spec authors should not import a Hypercore
pattern without first asking "what happens if Holepunch stops shipping?"

## Holepunch as a Single-Vendor Steward

Holepunch Inc. ([`holepunchto`](https://github.com/holepunchto), org created
2021-03-25, 617 public repos as of 2026-05-09) is the sole technical steward of
the Hypercore / Hyperswarm / Bare / Pear stack. There is **no foundation, no
consortium, no neutral standards body**. This is materially different from how
the comparable substrates are governed:

| Stack | Governance |
|---|---|
| Pears / Hypercore | Holepunch Inc. (single vendor; no foundation) |
| WASM Component Model | Bytecode Alliance (multi-vendor 501(c)(6)) |
| Containerd / Kubernetes / etc. | CNCF (Linux Foundation; multi-vendor) |
| W3C web specs | W3C process + Community Groups (multi-stakeholder) |
| Spritely OCapN | Spritely Institute (501(c)(3) non-profit) |
| Iroh | Number 0 Inc. (single vendor, but explicit dual-license + neutral protocol intent) |

The `pear/CONTRIBUTING.md` file is unusually explicit about the single-vendor
posture. Verbatim:

> Pear is an Open Source Project, authored & maintained by Holepunch.
>
> Be aware that **any pull-request or issue may be closed without explanation**.

That is not a foundation-style governance promise. It is the "we own this, we
take patches at our discretion" posture. Read it as documentation, not as a
slight — it is a more honest framing than most single-vendor open-source
projects offer.

## License

| Repo | License | Notes |
|---|---|---|
| `holepunchto/hypercore` | **MIT** | Inherited from the original Dat-era codebase |
| `holepunchto/hyperswarm` | **MIT** | Inherited |
| `holepunchto/hyperbee` | MIT | Inherited |
| `holepunchto/hyperdrive` | Apache-2.0 | Holepunch-era relicense |
| `holepunchto/autobase` | Apache-2.0 | Holepunch-original |
| `holepunchto/pear` | Apache-2.0 | Holepunch-original |
| `holepunchto/bare` | Apache-2.0 | Holepunch-original |
| `holepunchto/protomux` | MIT | Older codebase |
| `holepunchto/keet-appling` | Apache-2.0 | Application shell only — not the Keet client |

The pattern: **Dat-era code is MIT; Holepunch-era code is Apache-2.0**. Both
are permissive and patent-grant-compatible enough for downstream use, but the
mix is worth noting if Myrhiza ever wants to vendor a subset — the `LICENSE`
file at the root of each repo is the source of truth, not the org default.

The Keet messenger client itself is **closed-source**. The infrastructure it
runs on is open; the application is not. The `keet-appling` repo is just the
desktop shell (Apache-2.0); `keet-mobile-releases` ships APK / IPA artifacts
but no source. See `critiques.md` for why this matters.

## Contribution Model

Outside contributors land PRs, but the technical direction sits inside
Holepunch. Evidence from contributor counts (`gh api repos/.../contributors`):

| Repo | Top contributor (commits) | Top 3 outside Holepunch? |
|---|---|---|
| `hypercore` | mafintosh — 1585 (founder, Holepunch CTO) | No, top 3 are all Holepunch employees |
| `pear` | davidmarkclements — 409, rafapaezbas — 213 | No, top 5 are all Holepunch |
| `bare` | kasperisager — leads architecture | No |

Outside contributors do appear (pfrazee — 14 commits to Hypercore, RangerMauve,
noffle — historical names from the Dat-era community), but every architectural
decision since 2021 has been made by Holepunch staff. There is no MAINTAINERS
file in the major repos and no public RFC process. Issues like
[hyperswarm#60](https://github.com/holepunchto/hyperswarm/issues/60) ("Protocol
documentation?", open since 2020-04-28, last activity 2026-04-26) sit open for
years when they are out-of-band of Holepunch's roadmap.

This is a working model. It is not a *neutral* one.

## Trademark

The `holepunchto/pear/NOTICE` file declares "Copyright 2024 Holepunch Inc". The
`Pear`, `Pears`, `Keet`, and `Holepunch` names are presumptively held by
Holepunch Inc. The historical `Hypercore` mark and the `Dat` mark have a more
tangled lineage (see "Dat Lineage" below) — `Hypercore` is now used under
Holepunch stewardship; whether anyone holds a registered mark is not findable
in the repos themselves. Treat all four names as Holepunch-controlled until
proven otherwise.

The Keet iOS App Store listing (bundleId `io.keet.app`) lists the seller as
**"Holepunch Inc"**. The Flatpak metadata in
`holepunchto/electron-forge-maker-flatpak` lists the developer as
**"Tether Data S.A. de C.V."** — which is the Tether legal entity, not
Holepunch. This is the visible seam between the Holepunch operating company
and Tether the parent funder. See `history.md`.

## Single-Vendor Risk

If Holepunch shuts down tomorrow, what happens?

- **Open-source code persists.** All major repos are MIT or Apache-2.0; forks
  are legally trivial.
- **The DHT bootstrap nodes are Holepunch-operated.** The default
  `BOOTSTRAP_NODES` constant in `hyperdht/lib/constants.js` lists three
  hardcoded Holepunch-controlled hosts: `node1.hyperdht.org`,
  `node2.hyperdht.org`, `node3.hyperdht.org`. If those go away, every Hyperswarm
  client falls back to LAN-only or to operator-supplied bootstrap lists.
  [hyperswarm#194](https://github.com/holepunchto/hyperswarm/issues/194)
  documents the LAN-only fallback failing in practice.
- **Keet — the consumer flagship — is closed-source.** No fork is possible.
- **The npm registry assets are owned by Holepunch / mafintosh.** A community
  fork would have to publish under a new namespace.
- **Protocol direction is set by mafintosh and a small set of Holepunch
  engineers.** A fork community would have to re-establish technical
  leadership. The Dat-era community (~2017–2020) had this experience and
  fragmented; see lineage section.

The realistic continuity story: a community fork of the open-source layer
under a new namespace, with the bootstrap-DHT layer needing volunteer
operators. Bandwidth, operations, and decision-making would all have to be
re-bootstrapped. The Hypercore protocol is durable; the project as a coherent
moving target is not.

## Dat Lineage (Why This Has Already Happened Once)

The Hypercore protocol predates Holepunch. The original *Dat Project* was
founded in 2013 by Max Ogden, Karissa McKelvey, and Mathias "mafintosh" Buus,
funded by [Code for Science & Society](https://codeforscience.org) with grants
from the Sloan Foundation. The Hypercore append-only-log structure was
*Dat's* core data primitive, shipped through tools like the `dat` CLI.

The `datproject/dat` repo (BSD-3-Clause, created 2013-06-27, archived
de-facto — last meaningful commit 2021-07-21, sporadic README touches since)
was the user-facing product. It accumulated 8.2k stars. Around 2020 the Dat
Project lost its institutional funding and the active development moved with
mafintosh to a new commercial vehicle — what became Holepunch in 2021. The
[Dat whitepaper repo](https://github.com/datprotocol/whitepaper) (archived
2024-03-02) carries the explicit handoff: **"DEPRECATED — see DEPs for similar
functionality. More info on active projects and modules at dat-ecosystem.org"**.

The `dat-ecosystem.org` site is now a maintained-but-quiet community
reference; the active codebase is `holepunchto/*`. Continuity of the protocol
worked; continuity of the *governance* did not — the foundation-funded
incarnation died, the venture-funded one took over, and the Hypercore wire
format kept advancing.

That is the precedent for what would likely happen if Holepunch dissolved
again: another lineage shift, another dormant period, possibly another
commercial successor. Useful as evidence that the *codebase* is durable across
governance changes; not reassuring as evidence that *coordinated
roadmap-level work* survives them.

## Tether as Funder

Holepunch is funded primarily by [Tether](https://tether.io), the
USD₮ stablecoin issuer. This is not standard VC money and not foundation
grant money — it is a single corporate parent with one large balance sheet.
The implications:

- **Single-investor concentration risk.** If Tether deprioritizes Holepunch,
  the funding stops in one decision. There is no syndicate to absorb the loss.
- **Strategic alignment risk.** Tether's interest in Holepunch is publicly
  framed around peer-to-peer infrastructure for financial use cases (see
  Pearpass / Tether's wallet ambitions in the `tetherto/*` org). Pure-research
  or non-financial directions live at Tether's discretion.
- **Reputational coupling.** Tether the stablecoin operator has its own
  regulatory and reputational exposure. Anything Holepunch ships inherits some
  of that surface.

For Myrhiza spec authors: when borrowing a Pears pattern, the *technical*
borrowing is fine. The *operational* assumption "this stack will be here in
five years" is a Tether-corporate-strategy bet, not a community-governance
bet. Plan accordingly.

## Cross-references

- [history.md](./history.md) — chronology of Dat → Holepunch transition
- [critiques.md](./critiques.md) — the closed-source-flagship problem
- [lessons.md](./lessons.md) — single-vendor avoidance patterns
- [Iroh](../iroh/) — single-vendor but with explicit neutral-protocol framing
- [WASM Component Model](../wasm-component-model/) — multi-vendor foundation alternative
- [Spritely OCapN](../spritely-ocapn/) — non-profit-stewarded comparison
