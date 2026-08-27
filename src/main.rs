//! tupdatr — terminal update menu, rewritten from C/ncurses to Rust/ratatui.
//!
//! CLI surface mirrors the original `tu`: one long flag per action, plus
//! --all / --dry-run / --list / -h. With no flags it opens the TUI menu.

mod actions;
mod tui;

use clap::{Arg, ArgAction, Command};

use actions::ACTIONS;

fn build_cli() -> Command {
    let mut cmd = Command::new("tu")
        .about("Colourful terminal update menu (tupdatr)")
        .arg(
            Arg::new("all")
                .long("all")
                .help("run every update action")
                .action(ArgAction::SetTrue)
                .conflicts_with_all(ACTIONS.iter().map(|a| a.key)),
        )
        .arg(
            Arg::new("dry_run")
                .long("dry-run")
                .help("print commands instead of running them")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("list")
                .long("list")
                .help("list available actions and exit")
                .action(ArgAction::SetTrue),
        );

    for action in ACTIONS {
        cmd = cmd.arg(
            Arg::new(action.key)
                .long(action.key)
                .help(action.description)
                .action(ArgAction::SetTrue)
                .conflicts_with("all"),
        );
    }
    cmd
}

fn list_actions() {
    for action in ACTIONS {
        println!("{:<12} {}", action.key, action.description);
    }
}

fn requested_indices(matches: &clap::ArgMatches) -> Vec<usize> {
    ACTIONS
        .iter()
        .enumerate()
        .filter(|(_, a)| matches.get_flag(a.key))
        .map(|(i, _)| i)
        .collect()
}

fn main() {
    let matches = match build_cli().try_get_matches() {
        Ok(m) => m,
        Err(e) => {
            eprint!("{}", e.render());
            std::process::exit(1);
        }
    };

    if matches.get_flag("list") {
        list_actions();
        std::process::exit(0);
    }

    let dry_run = matches.get_flag("dry_run");
    let do_all = matches.get_flag("all");
    let requested = requested_indices(&matches);

    let code = if do_all {
        actions::run_all(&(0..ACTIONS.len()).collect::<Vec<_>>(), dry_run)
    } else if !requested.is_empty() {
        actions::run_all(&requested, dry_run)
    } else {
        match tui::interactive(dry_run) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("terminal error: {}", e);
                1
            }
        }
    };

    std::process::exit(code);
}
