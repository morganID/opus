# Opus

A capsule in the menu bar with two menus behind it: **Network** and **Xproxy**.

```text
   ┌─────────┐
   │ 12 ms ⏸ │   ← the capsule: latency, and a glyph when the gateway is paused
   └─────────┘
        │ click
   ┌──────────────────────────────────┐
   │ Network                        ▸ │   ping, loss, throughput, speed test
   │ Xproxy                         ▸ │   serving, models, pause, copy, refresh
   ├──────────────────────────────────┤
   │ Quit Opus                        │
   └──────────────────────────────────┘
```

Opus *runs* the gateway; the bundled `xproxy` command line *configures* it.
Two interfaces, one instance, one data folder.

Opus owns nothing of its own: it is the composition of two crates.

| Half | Crate | What it brings |
| --- | --- | --- |
| Network | [`pingbar`](../pingbar) (MIT) | ICMP monitor, throughput test, and the capsule that paints the status item |
| Xproxy | [`xproxy`](../xproxy) (AGPL) | the GPROXY gateway engine, its assembly, and the management operations |

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/morganID/opus/main/scripts/install.sh | sh
```

No clone, no compiler, no Rust. The script reads the latest release, downloads
`Opus.app.tar.gz`, drops it in `~/Applications`, signs it ad-hoc, and stops. Add
`--login` to start it at login:

```sh
curl -fsSL https://raw.githubusercontent.com/morganID/opus/main/scripts/install.sh | sh -s -- --login
```

Then `open ~/Applications/Opus.app`. The icon appears immediately showing `…`;
latency arrives on the first probe, and the gateway is ready a few seconds later
(keychain, SQLite schema, socket). Nothing needs configuring — it starts with
whatever accounts the instance already has.

`DEST=/Applications` installs for every user.

### From source

Only if you are working on Opus. It needs Rust and the two sibling checkouts
(`../pingbar`, `../xproxy`), because both halves are path dependencies:

```sh
git clone https://github.com/morganID/opus && cd opus
git clone https://github.com/morganID/xproxy ../xproxy   # and its engine:
../xproxy/scripts/fetch-core.sh
./scripts/install.sh --from-source
```

## Update

Re-run the same line:

```sh
curl -fsSL https://raw.githubusercontent.com/morganID/opus/main/scripts/install.sh | sh
```

It replaces the bundle in place, so the icon keeps its position and the data
folder is untouched. There is no auto-updater: an update is a download you ask
for, not a background process you did not.

`scripts/uninstall.sh` removes the bundle and the login item and leaves
`~/Library/Application Support/dev.xproxy` alone.

### Cutting a release

```sh
./scripts/release.sh            # tags v<version>, builds, publishes the asset
```

The asset is built for the machine that ran it (`uname -m`); an Intel Mac needs
its own release, or `--from-source`.

One thing every update does: **macOS asks for keychain access once more.**
The signature is ad-hoc, so each build has a new hash and the previous grant no
longer applies. A Developer ID would fix it; that needs a paid account and is
not set up here.

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

**Configure in Terminal…** opens a shell on the command line that configures
the gateway. A menu cannot type, and configuring is typing — a channel id, a
base URL, an API key:

```sh
xproxy provider add --channel openai --name work --api-key sk-...
xproxy models --refresh
xproxy status
```

That command line is **its own product**, installed separately:

```sh
curl -fsSL https://raw.githubusercontent.com/morganID/xproxy/main/scripts/install.sh | sh
```

Opus does not carry it. The gateway has its own release, its own version and
its own lifecycle; a menu bar app that shipped a second copy would be a second
copy to keep in step. Opus looks for `xproxy` on `PATH`
(`/opt/homebrew/bin`, `/usr/local/bin`, `~/.local/bin`, `~/.cargo/bin`), and if
it is missing the menu says so instead of failing quietly. `OPUS_XPROXY` points
it at a specific binary, which is what a development build uses.

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
