# Opus

A capsule in the menu bar with two menus behind it: **Network** and **Xproxy**.

```text
   ┌─────────┐
   │ 12 ms ⏸ │   ← the capsule: latency, and a glyph when the gateway is paused
   └─────────┘
        │ click
   ┌────────────────────────────┐
   │ Network                  ▸ │   ping, loss, throughput, speed test, target
   │ Xproxy                   ▸ │   serving, where, models, pause, copy, refresh
   ├────────────────────────────┤
   │ Quit Opus                  │
   └────────────────────────────┘
```

Opus owns nothing of its own: it is the composition of two crates.

| Half | Crate | What it brings |
| --- | --- | --- |
| Network | [`pingbar`](../pingbar) (MIT) | ICMP monitor, throughput test, and the capsule that paints the status item |
| Xproxy | [`xproxy`](../xproxy) (AGPL) | the GPROXY gateway engine, its assembly, and the management operations |

## Running it

```sh
cargo run
```

The icon appears immediately showing `…`; latency arrives on the first probe
and the gateway is ready a few seconds later (keychain, SQLite schema, socket).
Nothing needs configuring: the gateway starts with whatever accounts the
instance already has.

## What each menu does

**Network**

- latency and packet loss of the current target
- download / upload throughput from the last speed test
- **Speed test** — run one now
- **Target** — router, Google DNS, Cloudflare, Quad9

**Xproxy**

- whether the gateway is serving, and where (`http://127.0.0.1:8787`)
- how many models the catalog knows, and whether credentials are sealed
- **Pause / Start** — stop and restart the listener without quitting
- **Copy Base URL**, **Copy Gateway Key** — what a client needs
- **Refresh Tokens** — the check the engine does not schedule for itself

Configuration is not here on purpose. Providers, accounts, routes and client
keys belong to the command line:

```sh
cd ../xproxy
cargo run -p xproxy -- provider add --channel openai --name work --api-key sk-...
cargo run -p xproxy -- models --refresh
cargo run -p xproxy -- status
```

## How the two halves meet

Both are libraries, and neither knows about the other:

- `pingbar::Monitor` runs on its own thread and publishes snapshots on a
  `watch` channel; the event loop polls it on a 200 ms tick.
- `xproxy::Gateway` runs on a tokio runtime that Opus owns. The menu reads a
  mutex-protected copy of the last status rather than awaiting anything on the
  drawing thread — a 200 ms poll there would be a menu that stutters.

The capsule is `pingbar`'s, re-exported so there is one implementation of the
pill rather than two that drift.

## Dependencies

Both halves are **path dependencies** on sibling checkouts:

```toml
pingbar = { path = "../pingbar", features = ["menubar"] }
xproxy  = { path = "../xproxy/crates/xproxy" }
```

`xproxy` needs its engine checkout (`../xproxy/scripts/fetch-core.sh`) before
the first build, and Opus repeats the engine's `[patch.crates-io]` block,
because patches are workspace-local and do not travel through a path
dependency. Without it the build fails inside `gproxy-seaorm` with
`no method named execute_batch`.

## Known issue

The linker reports a duplicate symbol for `tray_icon`'s `TaoTrayTarget`:

```
ld: duplicate symbol '___IVAR_OFFSET_TaoTrayTarget'
```

Two versions of `tray-icon` are in the graph — 0.26 from `pingbar` and `opus`,
0.25 from the Tauri that `xproxy` links through `gproxy-host-tauri`. Both
register the same Objective-C class, so the linker keeps one copy and warns.
The app runs; the fix is to align on one version, which means either moving
`pingbar` to 0.25 (Tauri's) or moving the gateway to a separate process so
`opus` links no Tauri at all. The second is the better architecture and the
larger change.

## Licence

AGPL-3.0-or-later, because it links `xproxy`, which links GPROXY. `pingbar` is
MIT and compatible; the other direction would not be.
