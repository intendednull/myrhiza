**Date:** 2026-05-22
**Status:** active
**Subject:** DID Core standardization history — Rebooting the Web of Trust origins, the W3C DID-WG 2019–2022, Google/Mozilla formal objections, Director override, 1.0 Recommendation, WG rechartering 2026.

# DID standardization — the political history

The W3C standardization story of DIDs matters because (a) the standard nearly died over formal objections, and (b) the unresolved tensions from that fight still shape the ecosystem.

## Origins — Rebooting the Web of Trust (2015–2018)

The Decentralized Identifier idea emerged from the **Rebooting the Web of Trust** (RWoT) workshops, starting 2015. Key contributors included Christopher Allen (RWoT facilitator, former Certicom co-founder), Drummond Reed (Sovrin Foundation), Manu Sporny (Digital Bazaar / W3C VC), Daniel Hardman (Evernym), Joe Andrieu, Markus Sabadello, and others.

The initial framing was "self-sovereign identity" (SSI): a user controls their own identifiers, anchored in cryptography rather than centralized registries. By 2017–2018, this work crystallized into a W3C Community Group draft.

The DIF (Decentralized Identity Foundation) was incorporated in 2017 with founding members including Microsoft, IBM, Accenture, MasterCard, and uPort. DIF became the primary venue for DID method development (Sidetree → ION, peer-did, did-tdw→webvh, etc.).

## W3C DID Working Group (2019–2022)

The W3C DID Working Group was chartered in September 2019. Goal: produce DID Core 1.0 as a W3C Recommendation. The group included representation from:

- Digital Bazaar (Manu Sporny, chair).
- Microsoft (early; later representation diminished post-2022).
- Sovrin / Evernym.
- DIF / Mattr / Spruce / Transmute (smaller identity companies).
- Few from large-platform companies (Google, Apple, Mozilla, Amazon, Meta).

This composition imbalance — heavy SSI-vendor representation, light browser-vendor representation — became politically load-bearing.

## Candidate Recommendation transitions (March 2021, June 2021)

DID Core advanced to first Candidate Recommendation in March 2021, then to second Candidate Recommendation in June 2021. **Google, Apple, and Mozilla did not formally object at either transition**, but commenters from those companies raised concerns about method fragmentation and proof-of-work blockchain methods.

## Advisory Committee review (August 2021) — formal objections filed

The W3C AC review closed August 31, 2021. Three formal objections were filed:

### Google's objection

Google's representative argued that DID Core was insufficiently scoped without standardized methods. Quoting the [W3C DID 1.0 Formal Objections Report](https://www.w3.org/2022/03/did-fo-report.html):

> "DID-core is only useful with the use of 'DID methods'... none has made it past 'PROVISIONAL' status."

Google recommended "holding off on advancing did-core to REC status until at least 3 or more methods are also ready to advance to REC."

### Mozilla's objection

Mozilla raised technical interop concerns:

- "No practical interoperability" — the spec lacks demonstrated cross-implementation compatibility.
- Methods encourage fragmentation rather than convergence.
- The registry permits centralized methods, contradicting the "decentralized" framing.
- Proof-of-work blockchain methods (Bitcoin, Ethereum at that time pre-Merge) create environmental harm.

### `_Anonymized1_`'s objection

An anonymized AC member echoed Google's concerns and required "at least one, interoperable method that works out of the box" before advancing.

**Note on "Apple opposed":** The Myrhiza brief mentioned Apple as a formal objector. **The 2022-03 report does not name Apple.** The named objectors were Google, Mozilla, and `_Anonymized1_`. Apple may have raised concerns at the AC level without filing a formal objection; the visible record names three companies, with `_Anonymized1_` possibly being Apple but not confirmed in the public report. **Cite this carefully** — it's an easy fact to misattribute.

## Director override (June–July 2022)

The W3C Director (Tim Berners-Lee, with input from the Process oversight) considered the objections, found them insufficient to block the Recommendation, and authorized advancement. The Register (UK tech press) reported [W3C overrules Google, Mozilla's objections to identifiers](https://www.theregister.com/2022/07/01/w3c_overrules_objections/) on July 1, 2022.

**DID Core 1.0 was published as a W3C Recommendation on July 19, 2022.**

## Post-Recommendation period (2022–2026)

The DID-WG continued maintenance work but slowed. Working group activity tapered through 2023–2025. Key developments:

- **2022-09:** First test-suite results published.
- **2023-12:** Microsoft removed `did:ion` as a trust system from Entra Verified ID — a significant ecosystem retreat.
- **2024:** Bluesky's `did:plc` scaled to 12M+ identifiers, providing the only at-scale production data point.
- **2025-07:** Spruce archived `didkit`, redirecting users to the `ssi` crate.
- **2025-mid:** `did:tdw` → `did:webvh` rename, signaling the spec's identity reset.

## Rechartering (2026-03 → 2026-10)

The original DID-WG was archived on 2026-03-24. **A new DID Working Group was chartered, running until 2026-10-28**, focused on:

1. DID Core 1.1 — a refinement Recommendation, not a redesign. Issues like #927 ("DID v1.1 Implementation Gaps for Non-Human Identity," opened 2026-03-30) signal the 1.1 scope.
2. Tightened resolution conformance — addressing Mozilla's "no practical interoperability" objection from 2021.
3. Better alignment with W3C VC Data Model 2.0 and JOSE/COSE registries.

**No new method-level normative requirements are expected.** The 1.0 architecture (each method picks everything) survives.

## Unresolved tensions

The 2021–2022 objections never went away — they were overruled, not addressed. As of 2026:

| Original objection | Status |
|---|---|
| "No interop between methods" (Mozilla) | Still true. Each method is its own protocol. Universal-resolver mitigates but doesn't fix. |
| "Registry permits centralization" (Mozilla) | Still true. `did:plc` is operated by Bluesky PBC, and it's the most-deployed method. |
| "PoW blockchain methods are environmentally harmful" (Mozilla) | Partially addressed by Ethereum's Merge (Sep 2022 → PoS). `did:ion` on Bitcoin remains PoW-anchored but is operationally moribund post-2023. |
| "Need 3+ methods at REC status" (Google) | Not addressed. Zero DID methods are at W3C Recommendation status. All major methods (`did:web`, `did:key`, `did:plc`, etc.) are CCG drafts, DIF specs, or vendor specs. |

## Implications for Myrhiza

Two lessons matter:

1. **The W3C blessing isn't a quality signal for methods.** DID Core 1.0 is a Recommendation; *no method* is. When the corpus says "`did:web` is the most-used method," the meaning is "most-used method," not "most-standardized method." All methods are draft-quality from a process perspective.
2. **The "decentralized" framing is contested.** Mozilla's objection that the registry permits centralization went unanswered, and `did:plc`'s success-by-centralization validates the criticism. Myrhiza, which has actual P2P architecture, should not lean on DID-format adoption as a signal of "decentralization" — the DID brand has been weakened by centralized-method success.

## Sources

- W3C DID 1.0 Formal Objections Report — <https://www.w3.org/2022/03/did-fo-report.html>.
- W3C DID 1.0 Formal Objection FAQ — <https://www.w3.org/2019/did-wg/faqs/2021-formal-objections/>.
- The Register coverage of Director override — <https://www.theregister.com/2022/07/01/w3c_overrules_objections/>.
- W3C DID Core 1.0 Recommendation announcement — <https://www.w3.org/2022/07/pressrelease-did-rec.html.en>.
- W3C DID Working Group (current, chartered to 2026-10-28) — <https://www.w3.org/groups/wg/did/>.
- W3C DID Working Group (archived 2026-03-24) — <https://github.com/w3c/did-wg>.
- DID Core GitHub issues, #927 v1.1 non-human identity — <https://github.com/w3c/did/issues/927>.
- Rebooting the Web of Trust — <https://www.weboftrust.info/>.
- DIF founding members — <https://identity.foundation/about/>.
