//! The update-action table and its runner.
//!
//! Direct port of the original `actions.c` / `actions.h`. Each action is
//! either a fixed argv vector (executed directly — no shell involved) or a
//! fixed compile-time shell string for commands that genuinely need pipes,
//! loops, `&&`, or command substitution.

use std::process::Command;

pub enum Cmd {
    /// Executed directly via execvp-equivalent; argv[0] is the program.
    Argv(&'static [&'static str]),
    /// Run via `sh -c`; fixed literal only, never built from runtime input.
    Shell(&'static str),
}

pub struct Action {
    pub key: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub cmd: Cmd,
}

const DISTROBOX_ARGV: &[&str] = &["distrobox-upgrade", "--all"];
const FLATPAK_ARGV: &[&str] = &["flatpak", "update", "-y"];
const PODMAN_ARGV: &[&str] = &["podman", "auto-update"];
const SYSTEM_ARGV: &[&str] = &["rpm-ostree", "upgrade"];

/// Edit these to match your actual setup (project paths, container names,
/// etc). Prefer the argv form (`Cmd::Argv`) whenever the command is a single
/// program call with fixed arguments — only fall back to `Cmd::Shell` when
/// you genuinely need pipes, loops, `&&`, or command substitution.
pub static ACTIONS: &[Action] = &[
    Action {
        key: "devenv",
        label: "Devenv packages",
        description: "Find every devenv.nix under $HOME and run devenv update in it",
        cmd: Cmd::Shell(
            "find \"$HOME\" \\( -name .cache -o -name .git -o -name .var \
             -o -name .mozilla -o -name .steam -o -name .local \
             -o -name node_modules -o -name result -o -name templates \\) \
             -prune -o -type f -name devenv.nix -print | while read -r f; do \
             d=$(dirname \"$f\"); echo \"== $d ==\"; \
             (cd \"$d\" && devenv update) || fail=1; \
             done; exit \"${fail:-0}\"",
        ),
    },
    Action {
        key: "distrobox",
        label: "Distrobox containers",
        description: "Upgrade all distrobox containers",
        cmd: Cmd::Argv(DISTROBOX_ARGV),
    },
    Action {
        key: "firmware",
        label: "Firmware (fwupd)",
        description: "Refresh metadata and apply available firmware updates",
        cmd: Cmd::Shell("fwupdmgr refresh && fwupdmgr update -y"),
    },
    Action {
        key: "flatpak",
        label: "Flatpak apps",
        description: "Update all installed flatpak apps",
        cmd: Cmd::Argv(FLATPAK_ARGV),
    },
    Action {
        key: "flakes",
        label: "Nix flakes (all)",
        description: "Find every flake.nix under $HOME and update it",
        cmd: Cmd::Shell(
            "find \"$HOME\" \\( -name .cache -o -name .git -o -name .var \
             -o -name .mozilla -o -name .steam -o -name .local \
             -o -name node_modules -o -name result -o -name templates \\) \
             -prune -o -type f -name flake.nix -print | while read -r f; do \
             d=$(dirname \"$f\"); echo \"== $d ==\"; \
             nix --extra-experimental-features 'nix-command flakes' flake update --flake \"$d\" \
             || fail=1; done; exit \"${fail:-0}\"",
        ),
    },
    Action {
        key: "podman",
        label: "Podman auto-update",
        description: "Pull + recreate labeled podman containers",
        cmd: Cmd::Argv(PODMAN_ARGV),
    },
    Action {
        key: "system",
        label: "System image (rpm-ostree)",
        description: "Apply pending rpm-ostree upgrade",
        cmd: Cmd::Argv(SYSTEM_ARGV),
    },
];

impl Action {
    /// Human-readable rendering for --dry-run output and the menu.
    pub fn display(&self) -> String {
        match &self.cmd {
            Cmd::Argv(argv) => argv.join(" "),
            Cmd::Shell(cmd) => cmd.to_string(),
        }
    }
}

/// Runs one action. Returns the command's exit status (0 = success).
/// If `dry_run`, prints the command instead of running it.
///
/// Update tools may prompt (rpm-ostree, fwupdmgr), so stdio is inherited:
/// like the original, actions run attached to the real terminal.
pub fn run(action: &Action, dry_run: bool) -> i32 {
    if dry_run {
        println!("[dry-run] {}: {}", action.label, action.display());
        return 0;
    }

    println!("== {} ==", action.label);

    let mut command = match &action.cmd {
        Cmd::Argv(argv) => {
            let mut c = Command::new(argv[0]);
            c.args(&argv[1..]);
            c
        }
        Cmd::Shell(shell_cmd) => {
            let mut c = Command::new("sh");
            c.arg("-c").arg(shell_cmd);
            c
        }
    };

    match command.status() {
        Ok(status) if status.success() => 0,
        Ok(status) => match status.code() {
            Some(code) => code,
            None => {
                eprintln!("{} terminated by signal", action.label);
                128 + 1 // unknown signal; shell-style nonzero
            }
        },
        Err(e) => {
            eprintln!("failed to launch {:?}: {}", action.display(), e);
            127
        }
    }
}

/// Runs every action in `indices` in order. Returns the last nonzero status
/// seen (matching the original's overall-exit semantics), or 0.
pub fn run_all(indices: &[usize], dry_run: bool) -> i32 {
    let mut overall = 0;
    for &i in indices {
        let status = run(&ACTIONS[i], dry_run);
        if status != 0 {
            overall = status;
        }
    }
    overall
}
