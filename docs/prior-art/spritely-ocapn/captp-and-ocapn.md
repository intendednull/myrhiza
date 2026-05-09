# CapTP and OCapN

CapTP — the Capability Transport Protocol — is the wire side of the same model. Two cooperating-but-suspicious vats exchange messages that look like *capability invocations*, not RPC calls. OCapN (Object Capability Network) is the umbrella under which Spritely standardizes CapTP plus the pieces around it (locators, netlayers) so different language implementations can interop. The standardization effort started with an [NLnet/NGI Assure grant](https://nlnet.nl/project/SpritelyOCapN/) running August 2022 – October 2023 and is still pre-1.0 in 2026 ([draft specs](https://github.com/ocapn/ocapn)).

## CapTP message types

The current draft is at [github.com/ocapn/ocapn/draft-specifications/CapTP Specification.md](https://github.com/ocapn/ocapn/blob/main/draft-specifications/CapTP%20Specification.md). Headline operations (the protocol prefix is `op:` because messages serialize as Syrup records):

| Op | Fields | Purpose |
|---|---|---|
| `op:start-session` | `captp-version`, `session-pubkey`, `acceptable-location`, `acceptable-location-sig` | Both peers send first. Session pubkey is the connection-level identity. |
| `op:deliver` | `to-desc`, `args`, `answer-pos`, `resolve-me-desc` | Delivers a message to an object. `answer-pos` (a positive integer or `#f`) is what enables pipelining: it allocates a slot in the *answers* table that other messages can target before resolution. |
| `op:listen` | `to-desc`, `listen-desc` | Subscribes to a promise's resolution (fulfill or break). |
| `op:get` / `op:index` / `op:untag` | `receiver-desc`, key, `new-answer-pos` | Pipelinable accessors against eventually-settled values. |
| `op:gc-exports` | `export-pos-list`, `wire-delta-list` | Reference-count signal: peer can drop these exports. Pluralized in 0.18 — earlier protocol used `op:gc-export`. |
| `op:gc-answers` | `answer-pos-list` | Same, for the answers table. |
| `op:abort` | `reason` | Tear down the session and break unresolved promises. |

Note the absence of `op:bootstrap` — it was [removed in v0.12.0](https://spritely.institute/news/spritely-goblins-v0-12-0-released-two-new-netlayers-join-the-ocapn-family-and-more.html). Bootstrapping now uses the well-known **export position 0**, which exposes the methods `fetch` (look up an object by swiss-num), `deposit-gift`, and `withdraw-gift` (third-party handoff). Descriptors come in three flavors: `desc:import-object` (a remote object reference), `desc:export` (one of the peer's previously imported objects), `desc:answer` (a slot in the answer table — *this is the pipelining handle*), and `desc:sig-envelope` (signed wrapper used for handoff certificates).

The four tables (per E tradition; see [erights.org/4tables](http://erights.org/elib/distrib/captp/4tables.html)) per session, per side: **imports** (refs received from the peer), **exports** (refs we sent and the peer holds), **questions** (promises we created when sending an `op:deliver` with `answer-pos`), **answers** (the peer's analog — promises they hold corresponding to our questions). Refcounting moves between these as messages flow, and the `op:gc-*` ops drive collection.

## Promise pipelining at the wire level

A vat sends `op:deliver` with `answer-pos: 7`. That allocates entry 7 in the answers table. *Before* the answer resolves, the vat can send another `op:deliver` whose `to-desc` is `desc:answer 7` — the peer queues that second invocation against the still-unresolved answer slot, so as soon as slot 7 fulfills, the queued message fires *on the peer's side* without round-tripping back. This is the "B → A → B" collapse of the [pipelining example](https://files.spritely.institute/docs/guile-goblins/0.10/Promise-pipelining.html). Traditional gRPC/JSON-RPC has no equivalent because there's no first-class identifier for "the not-yet-existent return value of call N."

## Sturdyrefs and Swiss numbers

A *sturdyref* is the persistent, shareable form of a capability — analogous to a URI. Per the [Locators draft](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Locators.md):

```
ocapn://<designator>.<transport>/s/<swiss-num>?hint1=value1
```

- The **designator** is whatever the netlayer treats as machine identity (Tor onion service ID, SHA-256D of a TLS cert, libp2p peer ID).
- The **transport** is a symbol naming the netlayer (`onion`, `tcp-tls`, `libp2p`, `uds`, `websocket`, `prelay`).
- The **swiss-num** is an unguessable string identifying one specific object on that machine.

In Syrup encoding the same data is a record `<ocapn-sturdyref <ocapn-peer transport designator hints> swiss-num>`. An older example URL form (pre-current-draft, surfaced in [a third-party intro](https://blog.bovid.space/conceptual-intro-to-spritely-goblins.html)) used a flatter shape:

```
ocapn:s.onion.wy46gxdweyqn5m7ntzwlxinhdia2jjanlsh37gxklwhfec7yxqr4k3qd:8080/78PukR-2EKkr2bmvVfG0RcNCsiNQEvWJgz1MDKAeQb8
```

The OCapN [Locators draft](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Locators.md) is the canonical reference; the URL grammar has not yet stabilized.

The **swiss-num** matters for capability discipline: because it's unguessable (sufficient entropy, treated as a secret), holding the URL is *itself* the capability to bootstrap a connection to that object. There's no separate ACL check. Sturdyref → live ref happens via the bootstrap object's `fetch(swiss-num)` method, returning a fresh import in the imports table.

## Netlayers

The netlayer is a pluggable transport with a three-method interface: bidirectional ordered messaging, session security (only the two endpoints can inject), and bytes ([Netlayers draft](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Netlayers.md)). The netlayer is responsible for confidentiality, integrity, and *machine identity* (the public key the session signs against). CapTP itself is transport-agnostic.

Implemented netlayers in Guile Goblins as of 0.16.1:

| Netlayer | Module | Identifier | Released |
|---|---|---|---|
| **Tor onion** | `(goblins ocapn netlayer onion)` | onion service ID | early — pre-0.10 |
| **TCP+TLS** | `(goblins ocapn netlayer tcp-tls)` | SHA-256D of self-signed X.509 cert | v0.12.0 (Jan 2024) |
| **Prelay** | `(goblins ocapn netlayer prelay)` | server-relayed identity | v0.12.0 (preview, unencrypted) |
| **libp2p** | `(goblins ocapn netlayer libp2p)` | libp2p peer ID, NAT hole-punching via Go daemon | v0.14.0 (Sep 2024) |
| **WebSocket** | — | for browser environments | v0.15.0 (Jan 2025) |
| **Unix domain socket** | `(goblins ocapn netlayer uds)` | socket-passing introduction server | v0.16.0 (Aug 2025) |

Per [the TCP+TLS docs](https://files.spritely.institute/docs/guile-goblins/0.13.0/TCP-_002b-TLS.html): "TLS is being used because of its ubiquity, not because it's preferred… all that's needed is a public/private key pair we can use to establish an encrypted channel." The X.509 PKI is sidestepped — sturdyrefs use the *fingerprint* of the cert, not the issuer chain.

## Three Vats Three Networks

The 2024 OCapN interop demo was less of a single set-piece and more an evolving test suite goal: three different CapTP implementations (Racket Goblins, Guile Goblins, [Alexander Bondarenko's Haskell port](https://spritely.institute/news/introducing-ocapn-interoperable-capabilities-over-the-network.html)) talking over three different netlayers (Tor + TCP-TLS + libp2p) and all passing the OCapN test suite. The [Shepherd × Goblins update](https://spritely.institute/news/shepherd-goblins-update.html) demonstrated three vats — `Server A`, `Server B`, `Carol` — coordinating over OCapN. Public materials don't seem to use the literal phrase "Three Vats Three Networks" as a fixed label; treat it as describing the interop class of demos rather than a named release.

## Sources

- [OCapN repository](https://github.com/ocapn/ocapn)
- [CapTP draft spec](https://github.com/ocapn/ocapn/blob/main/draft-specifications/CapTP%20Specification.md)
- [Netlayers draft spec](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Netlayers.md)
- [Locators draft spec](https://github.com/ocapn/ocapn/blob/main/draft-specifications/Locators.md)
- [erights.org — CapTP Four Tables](http://erights.org/elib/distrib/captp/4tables.html)
- [Promise pipelining (Goblins docs)](https://files.spritely.institute/docs/guile-goblins/0.10/Promise-pipelining.html)
- [Using the CapTP API](https://files.spritely.institute/docs/guile-goblins/latest/Using-the-CapTP-API.html)
- [TCP+TLS netlayer](https://files.spritely.institute/docs/guile-goblins/0.13.0/TCP-_002b-TLS.html)
- [v0.12.0 release notes (TCP+TLS, prelay, op:bootstrap removal)](https://spritely.institute/news/spritely-goblins-v0-12-0-released-two-new-netlayers-join-the-ocapn-family-and-more.html)
- [v0.14.0 release notes (libp2p)](https://spritely.institute/news/spritely-goblins-v0-14-0-libp2p-and-improved-persistence.html)
- [v0.15.0 release notes (WebSocket, browser)](https://spritely.institute/news/spritely-goblins-v0-15-0-goblins-in-the-browser.html)
- [v0.16.0 release notes (UDS netlayer)](https://spritely.institute/news/spritely-goblins-v0-16-0-released.html)
- [v0.18.0 release notes (op:gc-exports rename)](https://spritely.institute/news/spritely-goblins-v0-18-0-sleepy-actors.html)
- [Introducing OCapN](https://spritely.institute/news/introducing-ocapn-interoperable-capabilities-over-the-network.html)
- [Shepherd × Goblins update (three-vat demo)](https://spritely.institute/news/shepherd-goblins-update.html)
- [NLnet — Spritely OCapN](https://nlnet.nl/project/SpritelyOCapN/)
