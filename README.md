# tupdatr — terminal update menu

A colourful, animated terminal menu for running your system-update routines in
one place: devenv projects, Nix flakes, distrobox containers, flatpak apps,
firmware, podman auto-updates, and rpm-ostree system upgrades.

Rewritten in Rust with [ratatui](https://github.com/ratatui/ratatui) from the
original C/ncurses version — same actions, same CLI, much shinier.


## Dependencies

- Rust toolchain (stable) via [rustup](https://rustup.rs)
- That's it — ratatui, crossterm, and clap are fetched automatically by cargo.

Install Rust:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Build

```sh
cargo build --release   # binary at target/release/tu
```

## Usage

With no arguments, `tu` opens the interactive menu:

```sh
./target/release/tu
```

Keys in the menu:

| Key        | Action                                    |
|------------|-------------------------------------------|
| up / down  | Move the highlight (`k` / `j` also work)  |
| space      | Toggle selection of highlighted entry     |
| a          | Select all, or clear all if all selected  |
| enter      | Run every selected entry                  |
| q / esc    | Quit without running anything             |

Pressing enter with nothing selected runs only the highlighted entry.
Actions run attached to the real terminal (so interactive prompts still
work), then a summary screen shows each action's result. If any action
fails, `tu` exits nonzero.

### Command line

Every action is also exposed as a long flag:

```sh
tu --list                 # list available actions
tu --dry-run --all        # print what would run, run nothing
tu --flatpak --system     # run specific actions
tu --all                  # run everything
tu --help
```

Options:

- `--all` — run every action. Cannot be combined with individual flags.
- `--dry-run` — print each command instead of executing it.
- `--list` — list actions and exit.
- `-h`, `--help` — usage text.

Exit status: `0` on success; otherwise the failing command's exit status
(or nonzero if a command could not be launched).

## Adding or changing actions

All actions live in the `ACTIONS` table in `src/actions.rs`. Each entry has:

- `key` — flag name (`--key`) and menu identity
- `label` — menu display text
- `description` — one-line explanation shown next to the label
- either a `Cmd::Argv` vector (preferred; executed directly, no shell
  involved) or a `Cmd::Shell` string for cases that genuinely need pipes,
  loops, or `&&`

Shell strings must stay fixed compile-time literals; never build them from
runtime input.

<!-- agent-sandbox:begin (managed by sandbox-note.sh, edits are overwritten) -->
## Agent sandboxes: no devenv

`devenv.nix` is for local development only. **If `devenv` is not on PATH,
you are in an agent sandbox** (a tmakr pod or a local `agents-sandbox`
container). These instructions then take precedence over any devenv, direnv
or `devenv shell` / `devenv up` steps elsewhere in this file:

- Don't install or bootstrap Nix, devenv or direnv. Use the tools already on
  PATH: Python 3.14 with `uv` (`uv sync`, `uv run …`; Python downloads are
  disabled), Node 24 with pnpm/bun, Go, Rust, C/C++ toolchains, ruff, eslint,
  Tailwind v4, git.
- Postgres: connection settings come from the environment. In a tmakr pod,
  if `PGHOST` is unset, run `set -a; . /etc/agent-db/database.env; set +a`.
  Don't start your own database server.
- Services that `devenv up` would start aren't running. Start what you need
  in the foreground (for example `uv run manage.py runserver`), or say what
  is missing.
- If something in `devenv.nix` has no equivalent here, say so and stop rather
  than working around it.
<!-- agent-sandbox:end -->
