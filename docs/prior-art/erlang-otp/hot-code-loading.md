**Date:** 2026-05-22
**Status:** active
**Subject:** Hot code loading — code server, two-version invariant, code_change/3, release_handler / appup / relup

# Hot code loading

This is the file `prior-art/willow/open-problems.md:131-140` points at. Read this if you are designing Myrhiza's hot-reload v2.

## What BEAM actually offers

BEAM lets a running node have **two versions of any module loaded simultaneously** — the *current* version and the *old* version. Newly invoked function calls go to *current*; in-flight calls from old code continue on *old*. The third `code:load_file/1` purges the old version (or kills any process still executing inside it, depending on flags); the formerly-current becomes old, and the new module becomes current.

This is the **two-version invariant** and it is the foundation of every higher-level hot-upgrade mechanism in OTP. The runtime guarantees it; the libraries (`code`, `release_handler`) drive it; the application has to handle the state migration. There is no "three versions live at once."

**Two-version reality check:** OTP gives you the in-flight-call safety. It does **not** give you "rolling apply this code change to all your running gen_servers' in-memory state." That is what `code_change/3` is for, and it is the part that is genuinely hard.

## The fully-qualified-call requirement

For a gen_server (or any long-lived process) to actually pick up a new module version on its next callback, the callback dispatch must be a **fully-qualified call** (`Module:fun(...)`) — not a local call (`fun(...)`). Local calls are bound at process start to a specific module version. The OTP behaviour boilerplate uses fully-qualified calls by design; user-level callbacks must do the same. Forgetting this is the classic "I deployed new code but it isn't running" bug.

## `code_change/3` — the state-migration callback

```erlang
-callback code_change(OldVsn, State, Extra) -> {ok, NewState} | {error, Reason}.
```

When the `release_handler` (or a manual upgrade script) is upgrading a running process:

1. Old module is loaded as `old`. Process is suspended via `sys:suspend/1`.
2. New module is loaded as `current`.
3. `Module:code_change(OldVsn, OldState, Extra)` is called. Inside, the callback transforms the old in-memory state record into the new shape.
4. Process is resumed. Its next callback uses the new state.

`OldVsn` is taken from the application's `.appup` file (see below). `Extra` is arbitrary, often `[]`.

**The hard parts `code_change/3` does not address:**

- **Schema migrations are the application's problem.** If state-record version 1 has 3 fields and version 2 has 4, the callback writes the migration. If you forget a field or migrate it wrong, the process crashes on its first callback after resume, and the supervisor restart-loops it. This is a real production failure mode and is documented in the OTP cookbook.
- **Long-lived references** (handles to ETS tables, file descriptors, socket pids, monitors) need to be re-validated by the callback. If schema v1 stored a pid that no longer exists in v2, the migration has to detect and replace it.
- **No replay.** OTP does not snapshot the old state, replay it through new code, and compare. `code_change/3` is one-shot — get it right or restart-loop.

`gen_statem` has `code_change/4` (one extra arg for the state name) — same shape, slightly more surface to migrate. See [`behaviours.md`](behaviours.md).

## `.appup` and `.relup`

For coordinated cross-module upgrades, OTP defines two file types:

- **`<app>.appup`** — author-written per-application. Declares, for each (OldVsn → NewVsn) pair, the sequence of low-level VM instructions to apply: load_module, update (with `code_change` callback), restart_application, supervisor child-spec changes, etc.
- **`<release>.relup`** — generated from per-app `.appup`s + the boot scripts of old and new releases via `systools:make_relup/3`. This is the executable upgrade plan the `release_handler` runs.

Generation is mechanical *if* the `.appup` files are correct. Authoring `.appup` is famously fiddly:

- Most module changes are `{update, Module, {advanced, Extra}}` (calls `code_change/3`) or `{load_module, Module}` (no state migration needed).
- Supervisor child-spec changes need `{update, Sup, supervisor}`.
- Application start/stop is yet another instruction.
- Order matters. Bad order = mid-upgrade crash.

The `appup` cookbook in the OTP docs is canonical reading. The `rebar3_appup_plugin` and Elixir's `Distillery` / `mix release` try to autogenerate the file from diffs; they handle the easy cases but punt on schema migration.

## `release_handler`

The OTP module that drives an upgrade:

1. `release_handler:set_unpacked/2` — unpack the new release tarball.
2. `release_handler:install_release/1` — run the `.relup` instructions. Suspend/migrate/resume each affected process.
3. `release_handler:make_permanent/1` — once you're convinced the new release works, mark it as the boot target so a node restart picks it up.
4. `release_handler:remove_release/1` — clean up the old release files.

The release_handler is itself a gen_server, lives in the `sasl` application, and is the canonical entry point.

## The honest part — most production OTP shops don't use this

This is the unflattering fact. Hot code loading was Erlang's marquee feature for decades. In 2026, the dominant deployment pattern across production OTP shops is **rolling restart** — kill a node, bring up a new one with the new release, repeat across the cluster. Reasons cited:

- **`.appup` authoring is genuinely hard.** Most teams stop authoring them after a few production incidents.
- **Kubernetes / orchestration changed the economics.** A k8s rolling deploy of a stateful app is a solved problem with predictable rollback. A relup is a snowflake with no rollback story unless you authored a downgrade `.appup` too.
- **The crash-resistance story already covers most "rolling apply" use cases.** Supervisors restart with fresh state; the application-level recovery story tolerates the brief restart blip.
- **State-migration bugs cause cascading failures.** A `code_change/3` that crashes on resume puts the supervisor into a restart loop, and now the upgrade has degraded a clean release roll into a node outage.

Notable holdouts who still use hot-loading: telecom switches (the original use case, where downtime is a tariff-breach), some financial trading systems, some database engines (e.g., Riak's old appup story). WhatsApp is documented to have used hot-loading historically; current practice is reportedly mixed.

The article ["Hot Code Reloading with Erlang and Rebar3"](https://medium.com/@kansi/hot-code-loading-with-erlang-and-rebar3-8252af16605b) and the LearnYouSomeErlang chapter ["Leveling Up in The Process Quest"](https://learnyousomeerlang.com/relups) both explicitly recommend "consider whether you actually want this before paying the complexity cost."

## What survived in practice — the lightweight pattern

Even shops that abandoned `.appup`/`.relup` still use the **bare `code:load_file/1`** primitive for two things:

- **Development.** `c(my_module).` in a REPL during dev compiles + loads in-place. Iteration loop is ~1 second; this is BEAM's killer ergonomics feature and remains universally used.
- **Live ops surgery.** A senior engineer fixing a bug in a production gen_server's behaviour by hot-loading a patched module via the REPL. Considered an emergency tool, not a deployment mechanism.

The mid-tier pattern: **`:code.purge` + `:code.load_file` from a deploy script, without `.appup`**. You give up state migration (gen_servers get restarted by their supervisors using the new module on next message), but you keep the "no node restart" property. Discord has historically described their deploys in roughly these terms.

## Implications for Myrhiza

This is the meat of the borrow / don't-borrow analysis. Synthesised in [`lessons.md`](lessons.md); the raw observations:

**The "two version invariant + fully-qualified calls + code_change callback" pattern translates to WASM Component Model in shape, not substrate.** WASM components have no equivalent of `code:load_file/1` — the runtime owns instance creation and there is no in-place module-swap primitive. The Wasmtime instance-pre-instantiation feature is closer to "spin up a new instance fast" than "swap modules under a running instance." The shape Myrhiza could borrow:

1. **Two-version invariant at the kernel.** The kernel can hold two instances of a component (`v1` and `v2`) simultaneously. New incoming events go to v2; in-flight calls on v1 drain.
2. **`code_change`-style state migration callback** as a required export on the `state-apply` component when its version bumps. `(prior_state_v1) -> Result<prior_state_v2, MigrationError>`. Pure function; can be pre-validated by the kernel before activating.
3. **Restart-as-default** — copy the "most shops use rolling restart" lesson. Myrhiza's v1 is restart-only ([`prior-art/willow/open-problems.md:126`](../willow/open-problems.md)); resist the temptation to ship in-place module swap until restart's costs are demonstrated.

**What not to borrow:**

- `.appup` / `.relup`'s instruction-list shape. Too imperative, too easy to author wrong. If Myrhiza ever needs cross-component upgrade coordination, it should be a declarative manifest (what versions of what components are now active) and a kernel-driven planner, not a hand-authored instruction list.
- "Hot loading as a marketing feature." OTP shipped this and then spent thirty years finding out most shops don't use it. Myrhiza should not lead with hot-reload as a selling point until there is clear evidence the v2 design has fewer footguns than `.appup`.

## Sources

- Code replacement docs (OTP 28): <https://www.erlang.org/doc/system/code_loading.html>
- Release handling (OTP 28): <https://www.erlang.org/doc/system/release_handling.html>
- Appup cookbook (OTP 27): <https://www.erlang.org/doc/design_principles/appup_cookbook>
- gen_server `code_change/3`: <https://www.erlang.org/doc/apps/stdlib/gen_server.html#Module:code_change/3>
- LearnYouSomeErlang relups chapter: <https://learnyousomeerlang.com/relups>
- "Hot code reloading with Erlang and Rebar3" (Singh, 2018): <https://medium.com/@kansi/hot-code-loading-with-erlang-and-rebar3-8252af16605b>
- rebar3_appup_plugin: <https://github.com/lrascao/rebar3_appup_plugin>
- HN discussion of hot-reload pragmatics: <https://news.ycombinator.com/item?id=10669131>
