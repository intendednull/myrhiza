**Date:** 2026-05-22
**Status:** active
**Subject:** ETS, DETS, Mnesia — what each is, what each isn't

# Storage

BEAM ships three in-tree storage primitives. They are distinct, often confused, and the choice between them is load-bearing.

## ETS — Erlang Term Storage

In-memory tables of Erlang terms, **outside any specific process's heap**. Created by a process but persisting independently; access is via shared memory inside the BEAM node (the only place in the runtime where actual shared memory exists across processes).

**Properties:**

- **In-memory only.** Tables vanish on node restart.
- **Concurrent read/write.** Multiple processes can read and write simultaneously. Tunable concurrency: `{read_concurrency, true}` and `{write_concurrency, true}` per-table.
- **Multiple table types:** `set` (key uniqueness), `ordered_set` (sorted), `bag` (multiple values per key), `duplicate_bag` (multiset).
- **Lookups are fast.** O(1) for `set`, O(log N) for `ordered_set`.
- **No transactions.** Updates are per-key atomic; multi-key sequences are not.

ETS is everywhere in production: caches, session stores, hot-path lookup tables, process registries (`Registry` in Elixir is ETS-backed). It is the "shared mutable in-memory hashmap" the rest of the BEAM pretends doesn't exist.

**Footgun: tables are owned by a process.** When the owning process dies, the table dies too (default). Tables can be `:public` (anyone read/write), `:protected` (owner write, others read), `:private`. `give_away/3` transfers ownership. `heir` option declares a backup owner.

## DETS — Disk ETS

Disk-backed version of ETS. Same API, but tables live on disk and survive restarts.

**Properties:**

- **On disk, single-file per table.**
- **Limited size:** 2 GB per table (legacy 32-bit file format). Larger files possible with `{ram_file, true}` or by using DETS as cache-on-disk for an in-memory ETS.
- **Slower than ETS** by orders of magnitude for writes; comparable for reads if the OS page cache is warm.
- **No concurrent writers from multiple processes** in a clean way; serialisation is per-table.

DETS is the unloved middle child. It exists because Mnesia uses it as a disk backend, and because legacy code uses it for "small persistent registry." Most modern apps reach for SQLite (via `sqlite3` NIF) or external KV stores instead.

## Mnesia — distributed transactional database

The OTP-in-tree distributed database. Tables can be:

- `ram_copies` (ETS only, in-memory)
- `disc_copies` (both ETS and on-disk, transactional log)
- `disc_only_copies` (DETS only, on-disk)

Mnesia adds **transactions** (`mnesia:transaction(fun() -> ... end)`), **distributed replication** across cluster nodes, and **schema evolution**.

**The strengths:**

- **Native Erlang terms.** No marshalling, no schema language.
- **Transparent distribution.** A `disc_copies` table can have its replicas distributed across cluster nodes; Mnesia transparently handles read/write routing.
- **Transactional.** ACID inside a single transaction.
- **No external dependency.** Ships with OTP; one less daemon to operate.

**The famously gnarly parts** (these are real and have caused production incidents at multiple shops over decades):

- **Net-split recovery is the application's problem.** When a partition heals, Mnesia tells you "table inconsistent" and stops accepting writes until you pick a side. The merge story is bespoke per-app. The classic recipe is "shut down the losing side, copy data from the winner, restart." For a stateful production system this is operationally expensive.
- **No range-scan across distributed tables.** Each `disc_copies` table is replicated, but joins across nodes are not optimised. Mnesia is "transparent" for single-table CRUD; "opaque" for analytics.
- **Schema bootstrap is fiddly.** Setting up a fresh cluster with the right table copies on the right nodes is a multi-step manual procedure. Tools like `mnesia_cluster` exist; none are universally loved.
- **Storage limits.** Single-table size is bounded by the 2 GB DETS limit for `disc_only_copies` tables (mitigated for `disc_copies` which use a different on-disk format). Multi-TB Mnesia deployments exist but are rare and usually involve sharding.
- **No write-ahead log durability tunable.** Mnesia's durability story is "all-or-nothing"; you can't trade write latency for crash-recovery durability the way Postgres lets you.

**Production deployments using Mnesia in 2026:**

- **RabbitMQ** — uses Mnesia for cluster metadata (exchanges, queues, bindings, users). Famously has a "split-brain recovery handler" with multiple selectable strategies (autoheal, pause_minority, ignore). RabbitMQ is the most-cited "Mnesia in production at serious scale" example, and its docs are a goldmine of cautionary tales.
- **CouchDB** — uses Mnesia internally for cluster state. Mostly out of sight to users.
- **Many small/mid Erlang apps** — Mnesia is "good enough" for small state and one fewer thing to operate.

**Production deployments that *moved off* Mnesia:**

- **RabbitMQ's 4.x release line is moving cluster metadata off Mnesia to a Raft-based store (Khepri)** built on `ra` (the Raft library). Announced 2022; feature-flagged in RabbitMQ 3.13.0 (2024-03-11); the default for new deployments in RabbitMQ 4.2.0 (2024-10-27). Reason given: Mnesia's split-brain recovery is unacceptable at modern scale. The migration is genuinely the marquee item for RabbitMQ 4.x.

This is the load-bearing data point: the project most associated with "Mnesia in production" is leaving Mnesia. That's not Mnesia being broken (it isn't), but it is the field's verdict that **even with a decade of operational experience, Mnesia's net-split story is too costly**.

## Implications for Myrhiza

Myrhiza's state primitive is the event log (per `CLAUDE.md` and the `state-apply` profile). Mnesia is the wrong shape for what Myrhiza needs (peer-symmetric event log + deterministic state machine), but the lessons are still load-bearing:

- **"Built-in distributed DB" is alluring and operationally costly.** OTP shipped Mnesia for free; production shops mostly moved to external stores or Raft. Myrhiza should not ship a "free" distributed DB without a strong story for net-split recovery — the absence of such a story is exactly the problem Mnesia did not solve.
- **CRDT-shaped state is more peer-friendly than transactional state.** RabbitMQ's move to Raft (Khepri) is one direction; Riak's move to CRDTs is the other. Myrhiza's event-log + pure state-apply is closer to the Riak/CRDT direction — the cross-peer reconciliation is "merge two event logs and re-run state-apply," which has clearer semantics than "merge two transactional databases."
- **ETS-style in-memory hashmap is the right shape for hot-path lookups.** WASM Component Model components are individual heap instances; for hot-path lookups inside a Myrhiza app, an in-memory map is the right shape. ETS validates that giving the kernel/host a "shared-memory key/value cache that components can read but not write" is operationally tractable.

See [`lessons.md`](lessons.md) for synthesised borrows.

## Sources

- ETS docs (OTP 29): <https://www.erlang.org/doc/apps/stdlib/ets.html>
- DETS docs (OTP 29): <https://www.erlang.org/doc/apps/stdlib/dets.html>
- Mnesia docs (OTP 29): <https://www.erlang.org/doc/apps/mnesia/mnesia_chap1.html>
- "Mnesia And The Art of Remembering" (LearnYouSomeErlang): <https://learnyousomeerlang.com/mnesia>
- RabbitMQ network partition handling: <https://www.rabbitmq.com/docs/partitions>
- RabbitMQ Khepri (move off Mnesia): <https://www.rabbitmq.com/blog/2022/05/17/rabbitmq-mnesia-migration>
- Khepri repo (Raft-based replacement): <https://github.com/rabbitmq/khepri>
- `ra` Raft library: <https://github.com/rabbitmq/ra>
