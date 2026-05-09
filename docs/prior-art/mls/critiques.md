**Date:** 2026-05-09
**Status:** active
**Subject:** Academic critiques and production-experience reports on MLS — verbatim where available

Cross-references: [`protocol.md`](./protocol.md), [`glossary.md`](./glossary.md), [`openmls.md`](./openmls.md), [`production-users.md`](./production-users.md), [`comparisons.md`](./comparisons.md), [`open-problems.md`](./open-problems.md), [`lessons.md`](./lessons.md).

Method note: every claim below is either a verbatim quote with attribution, or marked `(paraphrased)`. Where no substantive critique was located in a category, that is stated explicitly rather than padded.

## 1. Formal-verification critiques (Bhargavan et al.)

Bhargavan's group has produced the most rigorous academic scrutiny of MLS, including Inria-led work that explicitly *found* attacks during the draft process. From the paper "Formal Models and Verified Protocols for Group Messaging: Attacks and Proofs for IETF MLS" (Inria, 2019): the work "*found new attacks and proposed verified fixes [that were] incorporated into MLS*" — i.e. multiple draft-era flaws were caught only by formal analysis ([Bhargavan, 2019](https://inria.hal.science/hal-02425229)).

The TreeSync analysis (USENIX Security 2023) decomposes MLS authentication and surfaces the "double join attack" affecting earlier drafts. The line of work continues with TreeKEM modular machine-checked analyses ([eprint 2025/410](https://eprint.iacr.org/2025/410.pdf)).

Substantive takeaway for Myrhiza: MLS owes much of its current correctness to *post-hoc* formal verification on a still-mutating draft. The standardization process did not start from a verified design.

## 2. Cremers et al. — TreeKEM weaknesses and PCS limits

The 2021 USENIX paper "The Complexities of Healing in Secure Group Messaging" found that ART, TreeKEM, and MLS Draft-11 "*never fully heal authentication*" if new users can be created dynamically, and that protocols based on group keys "*provide significantly weaker PCS guarantees*" than commonly believed ([Cremers et al., USENIX 2021](https://www.usenix.org/conference/usenixsecurity21/presentation/cremers)).

The 2025 follow-up — "ETK: External-Operations TreeKEM and the Security of MLS in RFC 9420" — is the most damaging recent academic finding. It is the first analysis to cover external proposals/commits *as shipped*, and concludes (paraphrased from authors' summary): MLS *does not* realize the FCGKA security claimed for it when used with EUF-CMA-only signatures such as ECDSA ([Cremers, Günther, Wallez, Zhao, 2025](https://eprint.iacr.org/2025/229.pdf); [CISPA listing](https://cispa.de/en/research/publications/84666-etk-external-operations-treekem-and-the-security-of-mls-in-rfc-9420)). This is a security-property gap *in the published RFC*, not a draft-era issue. ECDSA is in MLS's required ciphersuite set.

Quarantined-TreeKEM (CCS 2024) identifies a different gap: "*inactive users — who remain offline for long periods — do not update their encryption keys and therefore represent a vulnerability for the entire group*" ([eprint 2023/1903](https://eprint.iacr.org/2023/1903.pdf)). Particularly relevant to a P2P runtime where offline peers are the common case.

## 3. Wire's deployment-experience reports

Wire is the most candid production deployer. From their MLS launch post: "*No other collaboration platform has implemented MLS in production, and we're proud to be the first to do so*" — but the same post acknowledges: "*we have observed instances of heavy loading on our cloud back-end servers as users and devices transition to MLS, and we have been steadily addressing client issues that arise*" and "*There's no lab big enough to pre-test everything. We're navigating new terrain*" ([Wire, *Redefining Secure Collaboration with MLS*](https://wire.com/en/blog/redefining-secure-collaboration-with-mls)).

The migration plan also forced a hard compatibility break on legacy devices: "*Apple devices that cannot upgrade to iOS 16 (like the iPhone 7 and below) will no longer be able to use Wire ... Android devices running version 7 and below will also lose compatibility*" ([Wire, *MLS is coming to Wire App*](https://wire.com/en/blog/mls-is-coming-to-wire-app-learn-more)). Notable signal: even a security-first vendor needed to drop a non-trivial install base to ship MLS.

## 4. HN / IETF-list discourse

Substantive HN technical critiques surfaced via the RFC announcement thread and Lobsters mirror ([HN 36815705](https://news.ycombinator.com/item?id=36815705); [Lobsters](https://lobste.rs/s/3fta7g/rfc_9420_aka_messaging_layer_security_mls)). Recurring concerns reported (paraphrased — direct thread fetch returned 429 at time of writing): the *quantity* of asymmetric-crypto operations is large; cipher and version agility is viewed as attack surface and several commenters recommend deployers pin a single ciphersuite; the spec carries "*a large number of explicit and distinguishable error cases*" that could yield oracle attacks; and varint-encoding rules permit overlong encodings that the spec does not always reject. (paraphrased, multiple commenters)

The Matrix MSC threads ([MSC4244](https://github.com/matrix-org/matrix-spec-proposals/pull/4244), [MSC4256](https://github.com/matrix-org/matrix-spec-proposals/pull/4256)) flag a subtler structural critique: in systems with application-level access control, "*certain scenarios require subverting MLS to remain secure — for instance, when a user cannot remove a compromised member but must communicate without providing decryption keys to that member, which breaks the MLS philosophy of 'everyone knows who can decrypt which messages'*" (paraphrased from MSC discussion). MLS's egalitarian membership model is not a fit for asymmetric-authority groups.

## 5. Federation / MIMI critiques

The "One Protocol to Rule Them All?" paper (arXiv 2303.14178) argues, against MIMI's framing, that interop *expands* the trust set: "*The resulting complexity of the system may inherently compromise the level of security due to the increased number of moving parts, just as key escrow mechanisms endanger cryptography*" ([arxiv 2303.14178](https://arxiv.org/html/2303.14178v3)). The same paper isolates the relevant MLS scope limit: "*MLS does not tackle the storage or distribution of keys, only how they might be used to send and receive cross-platform messages*."

Meredith Whittaker (Signal Foundation president) has framed the pragmatic objection (paraphrased from public statements): Signal supports interop in principle but is unwilling to add cross-provider trust without guarantees against backend "tricks" — i.e. interop only works if every participant's AS is itself trusted to the same standard.

## 6. Performance / large-group criticism

The 10K-member regime is mostly aspirational. Webex publicly demonstrates "*hundreds of people joining and leaving*" with sub-second key rolls ([Webex blog](https://blog.webex.com/collaboration/hybrid-work/scalable-end-to-end-security-in-webex/)) but production claims at the 10K+ scale are unverified in public sources. Welcome message size for large groups remains a known concern (see [`open-problems.md`](./open-problems.md) §5).

## 7. Implementation-bug history

OpenMLS has shipped at least two security-fix releases. v0.7.1 addressed **GHSA-qr9h-x63w-vqfm**; v0.7.2 included additional patches discovered through external security research. **GHSA-8x3w-qj7j-gqhf** fixed "*a bug due to which a wrong credential could be retrieved for validation of messages from past epochs*" — a length-comparison bug on tag verification ([OpenMLS releases](https://github.com/openmls/openmls/releases)). For Megolm, the 2022 Nebuchadnezzar audit found "*Practically-exploitable Cryptographic Vulnerabilities in Matrix*" — concrete impersonation and confidentiality breaks shipped in production matrix-js-sdk ([nebuchadnezzar-megolm.github.io](https://nebuchadnezzar-megolm.github.io/)). MLS's improved authenticated-membership model is in part a *response* to this class of bug.

## 8. Comparison-protocol / Marlinspike-style criticism

No specific Marlinspike public critique of MLS was located. His broader argument that "*centralized, unfederated protocols evolve more rapidly than decentralized ones*" ([Wikipedia summary](https://en.wikipedia.org/wiki/Moxie_Marlinspike)) is the structural objection: standardization slows iteration. MLS being IETF-blessed is a feature for interop and a cost for agility — which Marlinspike has historically chosen the opposite of for Signal.

(No specific quoted Marlinspike-on-MLS source found; do not invent one.)

## 9. Sources

- [Bhargavan et al. — Formal Models and Verified Protocols for Group Messaging (Inria, 2019)](https://inria.hal.science/hal-02425229)
- [Wallez et al. — TreeSync (USENIX Security 2023)](https://www.usenix.org/system/files/sec23fall-prepub-372-wallez.pdf)
- [Cremers et al. — Complexities of Healing in Secure Group Messaging (USENIX 2021)](https://www.usenix.org/conference/usenixsecurity21/presentation/cremers)
- [Cremers, Günther, Wallez, Zhao — ETK (eprint 2025/229)](https://eprint.iacr.org/2025/229.pdf)
- [Quarantined-TreeKEM (eprint 2023/1903 / CCS 2024)](https://eprint.iacr.org/2023/1903.pdf)
- [TreeKEM Modular Machine-Checked (eprint 2025/410)](https://eprint.iacr.org/2025/410.pdf)
- [Wire — Redefining Secure Collaboration with MLS](https://wire.com/en/blog/redefining-secure-collaboration-with-mls)
- [Wire — MLS is Coming to Wire App](https://wire.com/en/blog/mls-is-coming-to-wire-app-learn-more)
- [HN 36815705 — RFC 9420 a.k.a. Messaging Layer Security](https://news.ycombinator.com/item?id=36815705) (direct fetch rate-limited; commentary paraphrased)
- [Lobsters — RFC 9420 aka MLS overview thread](https://lobste.rs/s/3fta7g/rfc_9420_aka_messaging_layer_security_mls)
- [Matrix MSC4244](https://github.com/matrix-org/matrix-spec-proposals/pull/4244), [MSC4256](https://github.com/matrix-org/matrix-spec-proposals/pull/4256)
- [arXiv 2303.14178 — One Protocol to Rule Them All? On Securing Interoperable Messaging](https://arxiv.org/html/2303.14178v3)
- [OpenMLS releases (GHSA-qr9h-x63w-vqfm, GHSA-8x3w-qj7j-gqhf)](https://github.com/openmls/openmls/releases)
- [Nebuchadnezzar — Practically-exploitable Cryptographic Vulnerabilities in Matrix (2022)](https://nebuchadnezzar-megolm.github.io/)
- [Webex blog — Scalable E2E Security in Webex](https://blog.webex.com/collaboration/hybrid-work/scalable-end-to-end-security-in-webex/)
