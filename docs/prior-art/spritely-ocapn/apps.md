# Goblins Applications

Honest assessment up front: there is **no production end-user application running on Goblins with a meaningful user base** as of May 2026. The shipping-app story is a thin set of demos, jam entries, and chat prototypes. This is roughly comparable to Holochain's situation — a capable runtime with vibrant tech demos but a near-empty production-app shelf — though Spritely has been more honest about framing things as demos rather than overselling.

## Brassica Chat — flagship demo

[Brassica Chat](https://codeberg.org/spritely/brassica-chat) is Spritely's experimental peer-to-peer chat application and the most polished thing currently riding the full stack. It is written in Scheme on Guile Goblins, communicates over OCapN (Tor onion / Prelay / TCP-TLS netlayers), and is compiled to WebAssembly via Hoot for browser use. Chat rooms are CRDT-backed (operation-based with causal delivery), with editing, deletion, and emoji reacts. The demo runs locally — `make demo` for the simulated terminal version, `make server` for the web UI on `localhost:8088` — and ships sturdyrefs for three test identities (Alice, Bob, Carol). It is **a demo, not a hosted product**; Spritely does not run a public Brassica instance for end users. See [Composing capability security and CRDTs](https://spritely.institute/news/composing-capability-security-and-conflict-free-replicated-data-types.html).

## Goblin Chat — older demo, used as the OCapN interop conformance app

[Goblin Chat](https://codeberg.org/spritely/goblin-chat) is the older, simpler Guile Goblins chat demo. Its main role today is as the **canonical interop test**: when an OCapN implementation wants to prove it works, it implements enough of the Goblin Chat protocol to chat with the existing implementations. The Racket variant is at [racket-goblin-chat](https://codeberg.org/spritely/racket-goblin-chat). In April 2026 the team got Goblin Chat working across Guile Goblins, Ridley's Dart **DObjects** implementation, and EndoJS — the closest thing to the long-discussed "Three Vats Three Networks" milestone (see [interop thread](https://community.spritely.institute/t/ocapn-interoperability-progress/810)).

## Cirkoban — first browser Goblins+Hoot app

[Cirkoban](https://davexunit.itch.io/cirkoban) is a Sokoban-meets-Wireworld puzzle game by David Thompson, Juliana Sims, and Christine Lemmer-Webber, built in 10 days for the Spring Lisp Game Jam 2024. It was the first publicly accessible app from Spritely featuring Goblins running in a web browser via Hoot ([blog post](https://spritely.institute/news/cirkoban-sokoban-meets-cellular-automata-written-in-scheme.html)). The Goblins port to Hoot was partial — Cirkoban exercised the local actor model but not full distributed CapTP in-browser.

## Goblinville — first multiplayer Goblins-in-browser demo

[Goblinville](https://davexunit.itch.io/goblinville) was Spritely's Spring Lisp Game Jam 2025 entry: a multiplayer virtual world (walking around, chat, planting and harvesting crops) running in browser via Hoot, with full Goblins as the networking substrate. From the [retrospective](https://spritely.institute/news/goblinville-a-spring-lisp-game-jam-2025-retrospective.html): out of 26 jam entries, 7 used Guile Scheme but only Goblinville used Goblins. The team frames it candidly as "more of a tech demo than a true game."

## Mandy — ActivityPub on Goblins

[Mandy](https://spritely.institute/news/mandy-activitypub-on-goblins.html) is a prototype ActivityPub server implemented on Guile Goblins, by Jessica Tallon. The motivation: ActivityPub and Goblins are both actor-based, so Mandy bridges Goblins-native programs to the existing Fediverse, with the longer-term hope of layering ocap-grade abuse mitigation on top of ActivityPub semantics. As of January 2026 Mandy is described as a prototype, not a deployable Mastodon-class server.

## Distributed-systems / sysadmin demos

The [Shepherd integration](https://spritely.institute/news/spritely-nlnet-grants-december-2023.html) ("Distributed System Daemons: More Than a Twinkle in Goblins' Eye") is bringing Goblins-style ocap remoting to GNU Shepherd so that system administration across machines can be done capability-secure. NLnet-funded; covered in a FOSDEM 2025 talk (Juliana Sims). Status: research-grade, not yet a turn-key sysadmin tool.

## Toy demos in the docs

The Goblins documentation walks readers through building actors, mints (the [`simple-mint.rkt`](https://gitlab.com/spritely/goblins/-/blob/master/goblins/actor-lib/simple-mint.rkt) capability-secure currency demo), CRDT counters, sealed/unsealed pairs, and small chat-style examples. These are pedagogical, not product seeds.

## Anything in production with end users?

**No, not really.** Brassica is the closest, and it is a self-host-it demo. Mandy is a prototype. Cirkoban and Goblinville are jam games. The Shepherd work is research. The closest production-adjacent users are *other implementations* of OCapN consuming the spec — Endo/Agoric's blockchain stack and MetaMask Snaps both build on related capability-transport ideas (via Endo's `@endo/captp`), but those products do not "run on Goblins."

This is roughly the same shape of story as Holochain: strong runtime, charismatic demos, vivid documentation, real protocol work — but the gap from "demo runs locally" to "product with users" remains uncrossed.

## Sources

- [Brassica Chat repo](https://codeberg.org/spritely/brassica-chat)
- [Composing capability security and CRDTs](https://spritely.institute/news/composing-capability-security-and-conflict-free-replicated-data-types.html)
- [Goblin Chat repo](https://codeberg.org/spritely/goblin-chat)
- [Racket Goblin Chat repo](https://codeberg.org/spritely/racket-goblin-chat)
- [OCapN interoperability progress](https://community.spritely.institute/t/ocapn-interoperability-progress/810)
- [Cirkoban (itch.io)](https://davexunit.itch.io/cirkoban)
- [Cirkoban announcement](https://spritely.institute/news/cirkoban-sokoban-meets-cellular-automata-written-in-scheme.html)
- [Goblinville (itch.io)](https://davexunit.itch.io/goblinville)
- [Goblinville retrospective](https://spritely.institute/news/goblinville-a-spring-lisp-game-jam-2025-retrospective.html)
- [Mandy: ActivityPub on Goblins](https://spritely.institute/news/mandy-activitypub-on-goblins.html)
- [Shepherd + Goblins NLnet writeup](https://spritely.institute/news/spritely-nlnet-grants-december-2023.html)
