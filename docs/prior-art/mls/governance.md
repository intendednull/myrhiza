**Date:** 2026-05-09
**Status:** active
**Subject:** IETF MLS WG, MIMI WG, post-quantum work, academic + industry lineage, and funding behind MLS.

> Cross-refs: [protocol.md](./protocol.md), [openmls.md](./openmls.md), [glossary.md](./glossary.md), [production-users.md](./production-users.md), [comparisons.md](./comparisons.md), [lessons.md](./lessons.md).

This file answers: *who governs MLS, where is the spec going, and what funding / research lineage backs it?* — so spec authors can judge whether a dependency on MLS is a dependency on a healthy standards process or a vendor-captured one.

## 1. IETF MLS Working Group

- **Charter (active):** https://datatracker.ietf.org/wg/mls/about/
- **Area:** Security Area (sec). **Area Director:** Christopher Inacio.
- **Chairs (2026-05):** Nick Sullivan, Sean Turner.
- **Mailing list:** mls@ietf.org.
- **State:** Active; maintains RFC 9420 and develops extensions: MIMI-supporting hooks, additional credential types, post-quantum, message-loss detection, individual-member messaging, virtual clients.
- **Active drafts (2026-05):**
  - `draft-ietf-mls-extensions-09` — *MLS Extensions* — IESG target Aug 2025 (Proposed Standard); awaiting implementations.
  - `draft-ietf-mls-pq-ciphersuites-04` — *ML-KEM and Hybrid Cipher Suites for MLS* — **WG Last Call**. Authors: Mahy, Barnes (Cisco). Intended status: **Informational** (registers ciphersuites, doesn't change protocol).
  - `draft-ietf-mls-combiner-02` — *Flexible Hybrid PQ MLS Combiner* — **Standards Track**. Authors: Alwen (AWS), Hale (NPS), Mularczyk (AWS), Tian (NPS). Runs two parallel MLS sessions (one PQ, one classical) with synchronized membership, amortizing PQ cost via PARTIAL/FULL update modes.
  - `draft-ietf-mls-partial-02` — *Partial MLS* — WG Last Call.
  - `draft-ietf-mls-ratchet-tree-options-00` — ways to convey the ratchet tree.
  - `draft-ietf-mls-targeted-messages-00`.
  - `draft-ietf-mls-virtual-clients-00`.
- **Milestones on the books:**
  - Aug 2025 — submit extensions to IESG.
  - Dec 2025 — submit Additional Credentials draft to IESG.
  - **Dec 2026 — Post-Quantum security for MLS** (the headline milestone).

## 2. IETF MIMI Working Group

- **Charter (active):** https://datatracker.ietf.org/wg/mimi/about/
- **Area:** Applications and Real-Time Area (ART) — note this is a *different area* from MLS itself.
- **Chairs (2026-05):** Alissa Cooper, Tim Geoghegan.
- **Goal:** specify the minimal set of mechanisms required to make modern Internet messaging services interoperable, with E2EE preserved. MLS is the underlying group-keying substrate; MIMI defines the message envelope, identity, room policy, and transport.
- **Driver:** **EU Digital Markets Act** (DMA) interoperability obligations on designated gatekeepers. As of 2026 the only DMA-designated messaging gatekeeper is **WhatsApp (Meta)**; the DMA forces Meta to enable cross-platform interop. The IETF MIMI charter avoids naming the DMA explicitly but the regulatory pressure is the open secret behind the WG's existence and timeline.
- **Active drafts (2026-05):**
  - `draft-ietf-mimi-content-08` — *MIMI message content*.
  - `draft-ietf-mimi-protocol-06` — *MIMI using HTTPS and MLS* — the substrate-binding doc.
  - `draft-ietf-mimi-room-policy-03` — *Room Policy for MIMI*.
- **Milestones on the books:** content/identifiers/protocol/room-policy → IESG by Mar 2025; user-discovery → IESG by Nov 2025. Both targets had slipped as of this writing — none is a published RFC yet.
- **Production status:** **none.** No shipped MIMI bridge. Treat as research-stage.

## 3. Post-quantum work

Two complementary lines, both in WG Last Call or active development as of 2026-05:

1. **`draft-ietf-mls-pq-ciphersuites-04`** — registers PQ and PQ/T-hybrid ciphersuites. Hybrid options span 128/192/256-bit security levels, mostly **ML-KEM-768 + X25519 / P-256** (128-bit) and **ML-KEM-1024 + P-384** (192-bit). Pure-PQ variants exist. Informational, not Standards Track.
2. **`draft-ietf-mls-combiner-02`** — runs a classical MLS session and a PQ MLS session in parallel, synchronizes membership, and amortizes PQ cost by mixing PARTIAL (classical-only key updates) with FULL (hybrid PQ) updates. Standards Track. Authored mostly out of AWS + Naval Postgraduate School.

Expected ratification window: **late 2026** for the ciphersuites doc, given the WG's December 2026 PQ milestone. The Combiner design is a separate optimization layer and will plausibly land later.

The choice to ship ML-KEM (NIST's selected ML-KEM-768/1024) plus optional classical fallback aligns with the broader IETF posture (TLS, SSH, X.509) — MLS is **not** doing anything PQ-exotic.

## 4. Academic and industry lineage

- **TreeKEM origin / Asynchronous Ratcheting Trees (ART):** Cohn-Gordon, Cremers, Garratt, Millican, Milner — *On Ends-to-Ends Encryption: Asynchronous Group Messaging with Strong Security Guarantees*, **CCS 2018** (not EuroS&P; the IEEE EuroS&P attribution is wrong). https://eprint.iacr.org/2017/666 / https://dl.acm.org/doi/10.1145/3243734.3243747. ART is the conceptual ancestor of TreeKEM; TreeKEM proper was proposed by **Bhargavan, Barnes, and Rescorla** for the MLS WG.
- **Continuous Group Key Agreement (CGKA):** Alwen et al. (CRYPTO '20) introduced the CGKA primitive and analyzed TreeKEM as an instance. Subsequent papers — *Continuous Group Key Agreement with Active Security* (Alwen, Coretti, et al.), *Server-Aided CGKA* (Alwen, 2021/1456), *On The Insider Security of MLS* (Alwen, Jost), *Tainted TreeKEM*, *Quarantined-TreeKEM* (CCS 2024) — form the formal-security spine that MLS rests on.
- **Formal verification / Cryspen:** Karthikeyan Bhargavan (founded Cryspen Dec 2021; ex-Inria) drives the formally verified Rust crypto stack `libcrux`. libcrux combines formally-verified Rust generated from the HACL* project (Inria/Cryspen verified-cryptography effort, originally written in F*) with additional Rust code verified directly via Cryspen's `hax` toolchain — a hybrid: HACL*-extracted primitives + hax-verified Rust, not a single F*-via-hax extraction. Cryspen contributed to the MLS standard and ships verified primitives consumable by MLS implementations.

## 5. Implementation organizations

Author / sponsor map for RFC 9420 and the active drafts:

| Org | People (RFC 9420 authorship + WG-active) | Role |
|---|---|---|
| Cisco | Richard Barnes, Rohan Mahy | RFC 9420 lead author org; PQ-ciphersuites authors; Webex deployment |
| Inria & Mozilla | Benjamin Beurdouche | RFC 9420 author; protocol/security analysis |
| Phoenix R&D (Berlin) | Raphael Robert | RFC 9420 author; OpenMLS founder; ex-Wire Head of Security; helped initiate MIMI |
| Meta Platforms | Jon Millican | RFC 9420 author; ART (CCS 2018) co-author — distinct from TreeKEM, which was Bhargavan/Barnes/Rescorla |
| University of Oxford | Katriel Cohn-Gordon | RFC 9420 author; ART (CCS 2018) lead author — distinct from TreeKEM, which was Bhargavan/Barnes/Rescorla |
| (independent) | Emad Omara | RFC 9420 author; now at Apple per public talks (RCS UP 3.0 lead) |
| AWS | Joël Alwen, Marta Mularczyk | CGKA/TreeKEM theory; PQ-MLS Combiner authors |
| Cryspen / Inria | Karthikeyan Bhargavan | TreeKEM contributor; formal verification (libcrux, hax) |
| Wire Swiss GmbH | (corporate sponsor, not author) | First production deployment; `core-crypto` (open-source) |
| Naval Postgraduate School | Britta Hale, Xisen Tian | PQ MLS Combiner co-authors |

Geographic spread: US (Cisco, AWS, Meta, NPS), Switzerland (Wire), Germany (Phoenix R&D, Berlin), France (Inria), UK (Oxford). **No single-vendor capture.** Mozilla provides browser-side academic-industry bridging via Beurdouche.

## 6. Funding

- **NLnet — NGI Assure Fund / *Open MLS Infrastructure*** (project page: https://nlnet.nl/project/OpenMLS-infra/). Recipient: **Almeos UG**. Period: **Oct 2021 – Aug 2023**. Funded under EC Next Generation Internet programme, grant agreement **No 957073**. Scope: secure, metadata-minimizing, modular, federation-friendly MLS infrastructure components. **Grant amount not disclosed on the public page.**
- **European Research Council (ERC) — CRYSPEN project** (Bhargavan). Underwrites formal verification of post-quantum primitives that flow into MLS via libcrux.
- **Inria** — ongoing institutional backing for Beurdouche, Bhargavan-era research, and academic CGKA work.
- **Cisco, Meta, AWS, Mozilla, Apple, Phoenix R&D** — corporate engineering time on author / WG participation.

The funding mix is healthy by IETF standards: a public-interest EU grant funded the reference open-source library (OpenMLS) during exactly the years the spec was being finalized, with corporate engineering time covering the long tail. There is **no single corporate sponsor** that could pull MLS.

## 7. Sources

- IETF MLS WG datatracker — https://datatracker.ietf.org/wg/mls/about/
- IETF MIMI WG datatracker — https://datatracker.ietf.org/wg/mimi/about/
- RFC 9420 — https://www.rfc-editor.org/rfc/rfc9420.html
- `draft-ietf-mls-extensions` — https://datatracker.ietf.org/doc/draft-ietf-mls-extensions/
- `draft-ietf-mls-pq-ciphersuites` — https://datatracker.ietf.org/doc/draft-ietf-mls-pq-ciphersuites/
- `draft-ietf-mls-combiner` — https://datatracker.ietf.org/doc/draft-ietf-mls-combiner/
- `draft-ietf-mls-partial`, `-virtual-clients`, `-targeted-messages`, `-ratchet-tree-options` — datatracker search `name=draft-ietf-mls`
- `draft-ietf-mimi-content`, `-protocol`, `-room-policy` — datatracker search `name=draft-ietf-mimi`
- Cohn-Gordon, Cremers, Garratt, Millican, Milner — *On Ends-to-Ends Encryption* (CCS 2018) — https://eprint.iacr.org/2017/666
- Alwen et al. — *Server-Aided Continuous Group Key Agreement* — https://eprint.iacr.org/2021/1456.pdf
- *Quarantined-TreeKEM* (CCS 2024) — https://dl.acm.org/doi/10.1145/3658644.3690265
- Cryspen / libcrux — https://cryspen.com/libcrux-library/
- NLnet OpenMLS infrastructure project — https://nlnet.nl/project/OpenMLS-infra/
- Phoenix R&D — https://phnx.im/about
- EU DMA designation context — https://en.wikipedia.org/wiki/Digital_Markets_Act
- Internet Society IETF Ornithology MIMI summary — https://internetsociety.github.io/IETF-Ornithology/IETF/ART/mimi.html
