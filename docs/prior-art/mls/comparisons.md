**Date:** 2026-05-09
**Status:** active
**Subject:** MLS vs Signal Sender Keys vs Megolm vs OTR — comparative landscape for group-shaped E2EE

Cross-references: [`protocol.md`](./protocol.md), [`glossary.md`](./glossary.md), [`openmls.md`](./openmls.md), [`production-users.md`](./production-users.md), [`lessons.md`](./lessons.md), [`open-problems.md`](./open-problems.md), [`critiques.md`](./critiques.md).

## 1. At-a-glance

| Protocol | Status | Standardized | Practical group size | FS | PCS | Open impl | Production users |
|---|---|---|---|---|---|---|---|
| MLS (RFC 9420) | Stable | Yes — IETF (Jul 2023) | 2 → thousands | Yes | Yes (continuous) | OpenMLS, mlspp, mls-rs | Webex, Wire, RCS UP 3.0 |
| Signal Sender Keys | Stable | No — proprietary spec | up to ~1024 (WhatsApp) | Yes | Weak (key refresh on roster change only) | libsignal | Signal, WhatsApp, Messenger |
| Megolm | Stable | Matrix spec, not IETF | thousands (per-sender) | Yes (on rotate) | Weak — per-sender ratchet, no group key | libolm/vodozemac | Matrix / Element |
| OTR | Legacy | IETF informational | 2 only | Yes | N/A (no groups) | libotr | Pidgin, Adium (legacy) |

FS = forward secrecy. PCS = post-compromise security.

## 2. MLS vs Signal Sender Keys

Sender Keys is a per-sender symmetric ratchet whose initial key is shipped to peers over the pairwise Signal Protocol. Each sender owns one chain and rotates only on roster change. WhatsApp's design doc describes the lifecycle: "*If someone leaves the group, all the group members refresh their Sender Key and start over*" ([Cryptography Engineering, 2018](https://blog.cryptographyengineering.com/2018/01/10/attack-of-the-week-group-messaging-in-whatsapp-and-signal/)).

The structural differences:

- **Key agreement.** Sender Keys has *no* continuous group key agreement (CGKA) — it has N independent sender ratchets glued by pairwise Signal sessions. MLS has a single group secret per epoch derived from a TreeKEM CGKA.
- **Add cost.** Sender Keys requires the joining member to receive a sender-key bundle from every existing member over pairwise channels: O(N) handshakes per add. MLS distributes a Welcome plus a single Commit: O(log N) work for the Committer, O(1) for receivers in the typical path.
- **PCS.** Sender Keys recovers from compromise only via roster change or full reset. MLS provides post-compromise security on every Update/Commit — a compromised member who keeps participating heals on the next epoch. The OpenMLS overview puts the asymptotic claim plainly: "*In a group with 1000 members, the number of required operations to calculate new group keys would only be 10 as opposed to 1000 with existing protocols*" ([phnx.im, 2023](https://blog.phnx.im/rfc-9420-mls/)).
- **Authority over membership.** Sender Keys leans on server-asserted roster changes. Matthew Green flagged this as the load-bearing weakness: "*since the group management messages are not signed by the administrator, a malicious WhatsApp server can add any user it wants into the group ... This undermines the entire purpose of end-to-end encryption*" ([Green, 2018](https://blog.cryptographyengineering.com/2018/01/10/attack-of-the-week-group-messaging-in-whatsapp-and-signal/)). MLS makes group state a cryptographic artifact — every member verifies the same membership view, signed by the proposer/Committer.

## 3. MLS vs Megolm (Matrix)

Megolm is Matrix's group ratchet: each sender maintains an outbound Megolm session (AES-256-CBC + HMAC-SHA-256 + Ed25519 signature) and ships the session secret to recipients over pairwise Olm channels ([Matrix spec, Megolm](https://spec.matrix.org/v1.17/olm-megolm/megolm/)). There is no aggregate group key — recipients hold N inbound sessions in parallel.

Matrix itself is moving toward MLS. Their July 2023 announcement frames the win as scaling: "*MLS is particularly useful for conversations with large numbers of participating users, thanks to algorithmic improvements over the Double Ratchet most systems use today*" and "*introduces new security guarantees, such as the ability for group members to cryptographically verify the recipients of a message*" ([Matrix.org, 2023](https://matrix.org/blog/2023/07/a-giant-leap-with-mls/)). Their open challenge is federation: "*continuing to develop best practices for MLS that will work without modification in a decentralized environment*" — directly relevant to Myrhiza.

Megolm has had real-world cryptographic breaks that MLS's invariants prevent. The 2022 Nebuchadnezzar audit found that "*matrix-js-sdk only required that such messages are encrypted, not that they were encrypted with an Olm channel*" — yielding key-impersonation and confidentiality breaks ([nebuchadnezzar-megolm.github.io](https://nebuchadnezzar-megolm.github.io/)). MLS's Welcome/Commit machinery binds key delivery to authenticated group context.

## 4. MLS vs OTR (legacy)

OTR is a two-party protocol — deniability + perfect-forward secrecy for 1:1 chat. It does not solve group messaging. Listed only because the literature still cites it as the conceptual ancestor of forward-secure messaging.

## 5. MLS vs PreKey-bundle / pairwise-fan-out

The naive composition of Signal Protocol for groups (run pairwise Double Ratchets between every pair, fan out N-1 ciphertexts) costs O(N) per message and O(N²) state per group. Sender Keys mitigates the per-message cost but inherits O(N) for every roster change. MLS's TreeKEM gets every group operation to O(log N) ciphertexts in the Commit. For Myrhiza: this is the only asymptotic argument that matters — pairwise composition collapses past a few hundred peers; MLS reaches thousands.

## 6. Encrypted-group-state vs encrypted-per-message

These are different design points and the choice has architectural consequences:

- **MLS** produces a *group key per epoch*. All messages in an epoch derive from the same exporter secret. Adding/removing a member opens a new epoch.
- **Megolm / Sender Keys** produce *per-sender ratchet keys*. Membership is a property of the application layer, not of the key derivation.

For Myrhiza state-apply components — which depend on a deterministic, agreed-upon group view — the MLS model maps cleanly: epoch = state version, Commit = state-changing event. The Megolm model would force Myrhiza to layer its own membership-agreement protocol on top.

## 7. Recommendation framing for Myrhiza

The threshold question: **at what N does MLS become worth its complexity budget?**

- **N ≤ ~10 long-lived peers, low churn:** pairwise Signal-style composition is viable; MLS's invariants are nice-to-have.
- **N in the 10–100 range with churn:** Sender-Keys-style designs work but the membership-authority problem (Green 2018) bites. MLS earns its keep here if you need cryptographically verified rosters.
- **N > 100 or unbounded:** MLS is the only standardized choice. There is nothing else with the same FS+PCS+log-N property package.
- **Federation across trust boundaries:** MLS is necessary but *not sufficient* — see [`open-problems.md`](./open-problems.md). MIMI is the IETF effort layered on MLS for cross-provider interop.

If group-shaped capabilities in Myrhiza are bounded to small dyadic-to-handful sizes, MLS is overkill. If capabilities form groups of unbounded size — which is the natural target for a P2P runtime — MLS is the correct dependency, with the federation gap being the load-bearing risk to evaluate.

## 8. Sources

- [RFC 9420: The Messaging Layer Security (MLS) Protocol](https://www.rfc-editor.org/rfc/rfc9420.html)
- [phnx.im — RFC 9420 aka MLS – An Overview](https://blog.phnx.im/rfc-9420-mls/)
- [Matrix.org — A giant leap forwards for encryption with MLS (2023)](https://matrix.org/blog/2023/07/a-giant-leap-with-mls/)
- [Matrix specification — Megolm group ratchet](https://spec.matrix.org/v1.17/olm-megolm/megolm/)
- [Matthew Green — Attack of the Week: Group Messaging in WhatsApp and Signal (2018)](https://blog.cryptographyengineering.com/2018/01/10/attack-of-the-week-group-messaging-in-whatsapp-and-signal/)
- [Nebuchadnezzar — Practically-exploitable Cryptographic Vulnerabilities in Matrix (2022)](https://nebuchadnezzar-megolm.github.io/)
- [Cohn-Gordon et al. — TreeKEM design](https://eprint.iacr.org/2019/1189.pdf)
- [Webex blog — How MLS Enables Scalable End-to-End Security](https://blog.webex.com/collaboration/hybrid-work/scalable-end-to-end-security-in-webex/)
- [WhatsApp Sender Keys analysis — eprint 2023/1385](https://eprint.iacr.org/2023/1385.pdf)
