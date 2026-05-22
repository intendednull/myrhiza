**Date:** 2026-05-22
**Status:** active
**Subject:** Anonymity-transport survey — Tor / arti / Veilid / I2P / Nym / HOPR, plus Loopix and Chaum mixnet roots. Reference material for Myrhiza's netlayer plug-in API and topic-ID-rotation child specs.

# Anonymity transports

Five live transports (Tor, arti, Veilid, I2P, Nym, HOPR) plus their two academic ancestors (Chaum 1981, Loopix 2017). This folder exists because two open-problems files cite anonymity-transport mechanics by name:

- `prior-art/willow/open-problems.md:207-218` — *"Tor's hidden-service descriptor rotation — unlinkability via rotated descriptors with member-only knowledge of the next descriptor ID. Closest analogue."* (topic-ID rotation through dumb relays)
- `prior-art/iroh/lessons.md:34` — *"route apps over iroh-on-Tor or iroh-on-Veilid"* (netlayer plug-in framing)

For Myrhiza, anonymity transports are a **pluggable layer, not a default**. The kernel exposes a custom-transport API; apps that need anonymity opt in. This folder gives spec authors the cost/anonymity-set/threat-model data needed to pick which transport(s) to expose.

## Key facts at a glance

| Transport | Family | Founded | Current version | Language | License | Steward | Users (daily) |
|---|---|---|---|---|---|---|---|
| **Tor** | Onion routing | 2002 | 0.4.9.8 | C | BSD-3 | Tor Project (501c3) | ~2–4M |
| **arti** | Onion routing | 2020 | 2.0.0 (2026) | Rust | MIT/Apache-2.0 | Tor Project (501c3) | (uses Tor net) |
| **Veilid** | Onion + DHT | 2023 (DEF CON 31) | 0.5.3 | Rust | MPL-2.0 | Veilid Foundation (501c3, Cult of the Dead Cow) | low thousands |
| **I2P** | Garlic routing | 2003 | 2.12.0 (Java) / 2.60.0 (i2pd) | Java + C++ | Mixed PD/BSD/GPL/MIT | Volunteer | hundreds of thousands |
| **Nym** | Loopix mixnet | 2018 | mainnet 2022+ | Rust | Apache-2.0 | Nym Technologies AG + NYM token | low hundreds of thousands |
| **HOPR** | Loopix mixnet | ~2018 | mainnet 2021+ | TypeScript + Rust | GPL-3.0 | HOPR Association + HOPR token | low tens of thousands |

Common ancestors: **Chaum (1981)** "Untraceable Electronic Mail, Return Addresses, and Digital Pseudonyms" (CACM) and **Loopix** (Piotrowska et al., USENIX Security 2017). Both covered in [`mixnets.md`](mixnets.md).

## Reading order

1. [`README.md`](README.md) (this file) — overview + key-facts table
2. [`comparisons.md`](comparisons.md) — cross-cutting table; latency/anonymity-set/stewardship side-by-side
3. [`lessons.md`](lessons.md) — the decision file (validates/avoid/borrow)
4. [`open-problems.md`](open-problems.md) — what no transport solves
5. Per-system files: [`tor.md`](tor.md), [`veilid.md`](veilid.md), [`i2p.md`](i2p.md), [`mixnets.md`](mixnets.md) (Nym + HOPR + Loopix lineage)

## Why this folder exists for Myrhiza

The **load-bearing finding**: Tor v3's hidden-service descriptor-rotation construction — `blinded-id = ed25519-scalar-mult(root-key, H(period, srv))` — is the smallest cryptographic primitive that delivers unlinkability across rotation periods. Just hashing `(topic, period)` is insufficient: a member who knows the topic can recompute the next descriptor and so can anyone observing the rotation. The blinded-key construction means **only members can derive the next descriptor; observers see only unrelated `.onion`-shaped strings**. Myrhiza's topic-ID rotation child spec should lift this construction directly. See [`tor.md`](tor.md) §"Hidden-service descriptor rotation".

The **second finding**: anonymity is a **layer of layers** (transport / sealed-sender / E2E content / pattern-traffic). No transport alone is "anonymity." Myrhiza compositions must name **which adversary** each layer defeats. See [`lessons.md`](lessons.md) §"The single most important lesson".

**Framing disclosure.** This folder is written from a **Myrhiza-as-netlayer-pluggable-runtime** stance: the "Implications for Myrhiza" framings assume anonymity transports are *one* of several optional transports apps may select. A different design (anonymity-by-default, single-transport runtime) would weight the same facts differently. Future readers auditing whether netlayer-pluggability is itself the right primitive should weigh the corpus accordingly. The folder also leans toward **donation-funded / foundation-stewarded** transports (Tor, Veilid, I2P) over **token-funded** (Nym, HOPR). The token-funded entries are documented honestly but the "Implications for Myrhiza" framings flag the token-narrative bias explicitly so spec authors can evaluate the claims independently.

## Sources

- Tor: https://www.torproject.org, https://gitlab.torproject.org/tpo/core/arti
- Veilid: https://veilid.com, https://gitlab.com/veilid/veilid, DEF CON 31 unveiling (2023-08-11)
- I2P: https://geti2p.net, https://github.com/PurpleI2P/i2pd
- Nym: https://nymtech.net, https://github.com/nymtech/nym
- HOPR: https://hoprnet.org, https://github.com/hoprnet/hoprnet
- Loopix: Piotrowska et al., "The Loopix Anonymity System", USENIX Security 2017
- Chaum: David Chaum, "Untraceable Electronic Mail, Return Addresses, and Digital Pseudonyms", CACM 24(2), February 1981
- Cross-references: [`prior-art/iroh/lessons.md`](../iroh/lessons.md), [`prior-art/signal/identity.md`](../signal/identity.md), [`prior-art/willow/open-problems.md`](../willow/open-problems.md), [`prior-art/spritely-ocapn/lessons.md`](../spritely-ocapn/lessons.md)
