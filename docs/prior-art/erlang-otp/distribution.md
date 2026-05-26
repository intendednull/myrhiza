**Date:** 2026-05-22
**Status:** active
**Subject:** Distributed Erlang — EPMD, cookie auth, the tight-trust assumption, global / pg / syn, net-splits

# Distribution

BEAM's "free" distribution story is one of its strongest sells *and* one of its most dangerous-if-misunderstood features.

## Distributed Erlang in one sentence

If two BEAM nodes share a cookie, can reach each other over TCP, and were started with `-name` or `-sname`, then on either node you can `Pid ! Msg` to a process on the other node and it Just Works. Sends, links, monitors, RPC — all transparent across nodes.

The transparency is real and is the production validation behind WhatsApp, Discord, RabbitMQ. The transparency also bakes in trust assumptions that don't survive contact with the open internet.

## EPMD — Erlang Port Mapper Daemon

Per-host process (default port 4369) that maps node names to TCP ports. When node `foo@hostA` wants to reach node `bar@hostB`, it first contacts EPMD on hostB:4369 to ask "what port is `bar` on?", then opens a direct TCP connection to that port.

EPMD has historically been a security headache:

- Open by default on all interfaces (was, in older OTP; current docs recommend `-kernel inet_dist_use_interface`).
- Lists all node names visible on a host to any querier. Information disclosure.
- Was a known foothold for the 2020-era cookie-bruteforce attack patterns.

Modern alternatives: `erl_epmd` is now pluggable (since OTP 23, replaced the hardcoded EPMD lookup); `epmdless` is a community module that registers nodes via DNS or service discovery and skips EPMD entirely. Most cluster-management tools (libcluster for Elixir, partisan for advanced topologies) sit on top of this.

## Cookie authentication

When two nodes handshake, they exchange a challenge-response based on a shared **cookie** — a 20-character atom stored in `~/.erlang.cookie` (or set via `-setcookie`) on each node.

**What cookie auth gives you:** prevents accidental cluster mixing (a dev node won't join prod).

**What cookie auth does NOT give you:**

- **Confidentiality** — distribution traffic is in cleartext by default. Term-encoded over TCP.
- **Integrity** — no MAC; an active attacker on the wire can splice in messages.
- **Strong authentication** — the cookie is shared, so any node that knows the cookie is fully trusted (can RPC any module, including `os:cmd/1` for remote shell).
- **Defense against compromised peers** — once a peer is in the cluster, it can `rpc:call(OtherNode, os, cmd, ["rm -rf /"])`. There is no permission model.

The official secure-coding guidance (linked below) is unambiguous: **never run distributed Erlang over an untrusted network without TLS distribution and proper PKI**. The default cookie-only mode is for "machines on a private network you fully control."

## TLS distribution

Available via `-proto_dist inet_tls` plus cert configuration. Adds mutual-TLS handshake on top of the cookie. Recommended for any production deployment crossing trust boundaries.

Adoption: spotty. Many production OTP shops run on private networks (VPC, k8s pod network) and rely on the network boundary instead of TLS-dist. This is defensible if the network boundary is genuinely tight; it is a footgun when the boundary leaks (a misconfigured ingress, a compromised neighbour, a debug node accidentally exposed).

**Implications for Myrhiza:** Myrhiza is **peer-to-peer over the open internet**. The Distributed Erlang trust model (private network, shared cookie, full-trust on join) is exactly the wrong shape for our threat model. Don't borrow the wire protocol or the trust model. The behaviours (`gen_server`, `gen_statem`) are reusable shapes; the cluster is not. See [`lessons.md`](lessons.md) "Avoid."

## Net-splits

Two nodes in a cluster lose connectivity (network partition). Each side now thinks the other has died. They keep operating independently. Connectivity restores. **Now what?**

OTP's answer: nothing automatic. You receive `nodedown` notifications. Both sides may have written different state. Reconciliation is the application's problem.

**This is a real production failure mode** and is why Mnesia (the in-tree distributed DB; see [`storage.md`](storage.md)) has its famously gnarly "merge after net-split" story. Most production Erlang deployments either:

- **Avoid clustering Mnesia across availability zones**, and use external DBs (PostgreSQL, Cassandra) for cross-AZ state. RabbitMQ does this.
- **Use external consensus** (etcd, ZooKeeper, Consul) for "which side won the split." Discord and many large Elixir deployments use this pattern.
- **Use CRDTs / eventually-consistent state** (Riak built its own CRDT lib for this; the `riak_dt` repo is a canonical reference).

`pg` (see below) is documented to be **eventually consistent**, "best effort," and explicitly does not promise consistency under net-split. This is a feature; the previous `pg2` tried to be consistent and the cost was severe scalability problems at moderate cluster sizes.

## `global` — strongly consistent global name registry

`global:register_name/2` registers a process under a name visible to every node in the cluster. Uses a distributed lock + a global name table. Conflicts (two nodes register the same name simultaneously) trigger a user-supplied resolver function.

**Properties:**

- **Strong consistency.** Locks involve all nodes; a registration is acknowledged only after all nodes confirm.
- **Slow.** Scales poorly past a few dozen nodes.
- **Net-split-fragile.** Registrations during a split need resolution on merge.

In practice: `global` is fine for small clusters (singleton coordinators, leaders); painful in large ones.

## `pg` (new) and `pg2` (gone)

**`pg2`** was the historical "process groups" module: distributed many-to-many name → pid mapping. **Deprecated in OTP 23 (2020) and removed in OTP 24** — not deprecated-in-place. Code targeting OTP 24+ must use `pg`.

**`pg` (the new one)** was rewritten in OTP 23 by the WhatsApp/Meta team, based on Andrew Bennett's `cpg` (CloudI Process Groups). The redesign:

- **Eventually consistent** by design. No global locks; registrations propagate asynchronously.
- **Local-first reads.** `pg:get_members/1` reads from a local replica of the table; no cross-node round trip.
- **Conflict-free.** A pid can be in many groups; many pids can be in one group; the merge function is set-union and commutes naturally.
- **Net-split-tolerant.** Each side keeps its local view; on merge, the union is the new view (with both sides' processes included if both are alive).

**`pg`'s honest limitation:** it does not solve **uniqueness** (one process per name globally). It is a group manager, not a leader-election registry. If you want "exactly one process named `:order_book` in the whole cluster," you need `global`, an external consensus store, or a leader-election library on top.

**Implications for Myrhiza:** `pg`'s shape — eventually consistent, local-first, set-union merge — is the right model for Myrhiza behaviour-coordination (`prior-art/willow/open-problems.md:303`). Myrhiza peers cannot do a global lock and the lock would be the wrong shape anyway. Borrow the model; the in-network registry of behaviour instances per topic should be CRDT-shaped, locally-readable, async-propagating. See [`lessons.md`](lessons.md) "Borrow."

## `syn` — third-party alternative

A widely-used third-party registry by Roberto Ostinelli. Originally written because `pg2` was unscalable; relevance reduced (but not eliminated) by the new `pg`.

Features `syn` has and `pg` does not:

- **Conflict-resolution callbacks** on net-split merge.
- **Per-process metadata** that propagates with the registration.
- **Unique-name registration** (with a tunable resolver).

Adoption: notable in some Elixir codebases. `pg` is the in-tree, supported, default-recommended choice as of OTP 23+; `syn` is the choice when its specific features (metadata, callbacks) are required.

## Leader election in BEAM — what's not in-tree

There is **no in-tree leader-election primitive** in OTP. The patterns in production:

- **`global:register_name/2`** with the single-node lock as a poor-man's leader. Works at small scale.
- **`riak_ensemble`** (Basho, archived): a Raft implementation for BEAM. Used in Riak KV.
- **`ra`** (RabbitMQ): a Raft library for BEAM, well-maintained, MIT/MPL. Underlies RabbitMQ's "quorum queues" feature shipped in 2019.
- **External consensus** (etcd / ZooKeeper / Consul) accessed via NIFs or HTTP.

The community's reluctant consensus: when you need real consensus, use a Raft library (`ra`) or external coordinator. BEAM does not give you consensus for free; its sweet spot is "many peers, eventual consistency, supervised crash-restart."

**Implications for Myrhiza:** Myrhiza's behaviour-coordination work needs to pick a posture: eventual-consistency (the `pg` shape) or strong-consistency (Raft). The OTP world has both available; the lesson is that the choice is load-bearing and not free either way.

## Sources

- Distributed Erlang docs (OTP 29): <https://www.erlang.org/doc/system/distributed.html>
- Distribution protocol details: <https://www.erlang.org/doc/apps/erts/erl_dist_protocol.html>
- TLS distribution (ssl v11.5): <https://www.erlang.org/doc/apps/ssl/ssl_distribution.html>
- EEF Security WG distribution hardening: <https://security.erlef.org/secure_coding_and_deployment_hardening/distribution.html>
- "Who wants cookies?" cookie threat-model: <https://blog.voltone.net/post/4>
- `pg` module (OTP 29): <https://www.erlang.org/doc/apps/kernel/pg.html>
- `global` module: <https://www.erlang.org/doc/apps/kernel/global.html>
- `syn`: <https://github.com/ostinelli/syn>
- `ra` Raft library (RabbitMQ): <https://github.com/rabbitmq/ra>
- "An evaluation of Erlang global process registries: meet Syn": <https://www.ostinelli.net/an-evaluation-of-erlang-global-process-registries-meet-syn/>
- "OTP 23, PG2, and Outages" — pg2-removal field report: <https://malloc.dog/blog/2022/11/07/otp-23-pg2-and-outages/>
