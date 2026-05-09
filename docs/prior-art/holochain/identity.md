# Identity + crypto

## Per-app keypairs

Each app install generates a new ed25519 keypair; the public key is the agent's address in that app's DHT and signs every source chain action and every network message ([Agent-centric Digital Identity](https://medium.com/h-o-l-o/agent-centric-digital-identity-5314d507f0ab)).

This is simple and works, but it's strictly per-app: the same human user has different identities in different DNAs. Cross-app identity, multi-device coordination, and key rotation are not solved by this primitive.

## DeepKey / DPKI: the seven-year saga

[**DeepKey / DPKI**](https://github.com/holochain/deepkey) was the planned solution for cross-app identity, multi-device key management, and key rotation: a system happ where users register all their per-app keys under a single keyset with M-of-N change rules. DeepKey was repeatedly attempted and shipped in 0.4 behind a feature flag.

**It was removed in 0.6.** The 0.5→0.6 upgrade explicitly removes the entire `dpki` config block and the `device_seed_lair_tag` field ([upgrade-holochain-0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)). After roughly seven years of effort, the maintainers concluded the DPKI design as built was wrong and pulled it.

**There is currently no canonical multi-device or key-rotation story.**

This is the most important cautionary tale in the Holochain corpus for any new P2P runtime: shipping identity is harder than shipping the rest of the runtime. Apps were built on DPKI and are now stranded.

## No built-in MLS / group key management

Group chats today (e.g. Relay) handle their own group key derivation per-app. There is no shared cryptographic primitive for forward-secret group messaging — each happ rolls its own.

This is a missed opportunity: every messaging-shaped app reinvents the same wheel, and most do it badly. A runtime-level MLS primitive (or equivalent) would have prevented years of duplicated effort.

## Warrants

[**Warrants**](https://developer.holochain.org/concepts/7_validation/) are the Sybil/malicious-author response: when a validation authority detects an invalid op, it publishes a signed warrant; warrants gossip to agent-activity authorities and then network-wide; receiving peers block the warranted author.

This works against integrity violators but not against general Sybil flooding — there's no proof-of-personhood or stake gate. A single attacker with infinite identities can still:

- Generate fresh keys faster than warrants can propagate
- Free-ride on the DHT without contributing storage (zero-arc participation)
- Replay or pre-validate to avoid emitting any "invalid op" event a warrant could attach to

Warrants are an important primitive (cryptographic proof of misbehavior, gossip-distributed) but they are not a Sybil-resistance answer. They're a "if we caught one, we can banish them" answer.

## Implications for Myrhiza

- **Decide identity scope before v1.** Per-app pubkey OR cross-app+multi-device identity, not "we'll figure it out." Half-shipped identity is worse than missing identity.
- **Group cryptography is a runtime opportunity.** MLS-as-a-host-import lets every app reuse one well-audited primitive. Holochain's per-app reinvention is a clear miss.
- **Warrants are worth borrowing for the misbehavior-response story.** Cleaner than reputation, harder to game. See [`lessons.md`](lessons.md).
- **Don't claim to solve Sybil via consensus-avoidance.** "Agent-centric" doesn't make Sybil go away; it relocates the problem. Be explicit that gating who can join a network is the app author's responsibility (membrane proofs are Holochain's pluggable answer to this; worth borrowing).

## Sources

- [Agent-centric Digital Identity (Friedman)](https://medium.com/h-o-l-o/agent-centric-digital-identity-5314d507f0ab)
- [DeepKey repo](https://github.com/holochain/deepkey)
- [Upgrade 0.5 → 0.6](https://developer.holochain.org/resources/upgrade/upgrade-holochain-0.6)
- [Concepts — Validation (warrants)](https://developer.holochain.org/concepts/7_validation/)
- [Sybil attack vulnerability trilemma (Tandfonline)](https://www.tandfonline.com/doi/full/10.1080/17445760.2024.2352740)
