//! Interactive TUI: colourful animated action menu plus a post-run summary.
//!
//! Replaces the original ncurses `menu.c`. Update commands themselves run
//! attached to the real terminal (stdio inherited, TUI torn down first) so
//! interactive prompts keep working exactly like the C version.

use std::io::{self, Stdout};
use std::time::Duration;
use ratatui::{
    backend::CrosstermBackend,
    layout::Alignment,
    style::{Modifier, Style},

    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

use crate::actions::ACTIONS;

/// Palette used for borders, labels, and the rainbow title shimmer.
const RAINBOW: [ratatui::style::Color; 6] = [
    ratatui::style::Color::LightRed,
    ratatui::style::Color::Yellow,
    ratatui::style::Color::LightGreen,
    ratatui::style::Color::Cyan,
    ratatui::style::Color::LightBlue,
    ratatui::style::Color::LightMagenta,
];

const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

const FOOTER_TEXT: &str =
    "up/down move · space toggle · enter run selected · a all/none · q quit";

type Term = Terminal<CrosstermBackend<Stdout>>;

fn init_terminal() -> io::Result<Term> {
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(
        io::stdout(),
        crossterm::terminal::EnterAlternateScreen,
        crossterm::event::EnableMouseCapture
    )?;
    let mut term = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    term.hide_cursor()?;
    Ok(term)
}

fn restore_terminal() {
    let _ = crossterm::execute!(
        io::stdout(),
        crossterm::event::DisableMouseCapture,
        crossterm::terminal::LeaveAlternateScreen
    );
    let _ = crossterm::terminal::disable_raw_mode();
}

enum Outcome {
    Quit,
    Run(Vec<usize>),
}

/// Full interactive session: menu → run → summary. Returns the process exit
/// status (nonzero if any action failed).
pub fn interactive(dry_run: bool) -> io::Result<i32> {
    // --- menu phase -------------------------------------------------------
    let outcome = {
        let mut term = init_terminal()?;
        let mut selected = vec![false; ACTIONS.len()];
        let mut highlight = 0usize;
        let mut tick: u64 = 0;
        let mut outcome = Outcome::Quit;

        loop {
            term.draw(|f| draw_menu(f, &selected, highlight, tick, dry_run))?;

            // Poll briefly so animation keeps ticking even without input.
            if !crossterm::event::poll(Duration::from_millis(50))? {
                tick = tick.wrapping_add(1);
                continue;
            }
            let ev = crossterm::event::read()?;
            if let crossterm::event::Event::Key(key) = ev {
                use crossterm::event::{KeyCode, KeyEventKind};
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Up | KeyCode::Char('k') => {
                        highlight = (highlight + ACTIONS.len() - 1) % ACTIONS.len();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        highlight = (highlight + 1) % ACTIONS.len();
                    }
                    KeyCode::Char(' ') => selected[highlight] = !selected[highlight],
                    KeyCode::Char('a') => {
                        let all_set = selected.iter().all(|&s| s);
                        let v = !all_set;
                        for s in selected.iter_mut() {
                            *s = v;
                        }
                    }
                    KeyCode::Enter => {
                        if selected.iter().any(|&s| s) {
                            let idxs: Vec<usize> = selected
                                .iter()
                                .enumerate()
                                .filter_map(|(i, &s)| s.then_some(i))
                                .collect();
                            outcome = Outcome::Run(idxs);
                        } else {
                            outcome = Outcome::Run(vec![highlight]);
                        }
                        break;
                    }
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    _ => {}
                }
            }
        }
        drop(term);
        restore_terminal();
        outcome
    };

    let ran: Vec<usize> = match outcome {
        Outcome::Quit => return Ok(0),
        Outcome::Run(idxs) => idxs,
    };

    // --- run phase: real terminal, like the original ----------------------
    println!();
    let mut results = Vec::with_capacity(ran.len());
    let mut overall = 0;
    for &i in &ran {
        let status = crate::actions::run(&ACTIONS[i], dry_run);
        results.push((i, status));
        if status != 0 {
            overall = status;
        }
    }

    // --- summary phase -----------------------------------------------------
    {
        let mut term = init_terminal()?;
        let mut tick: u64 = 0;
        loop {
            term.draw(|f| draw_summary(f, &results, tick, dry_run))?;
            if !crossterm::event::poll(Duration::from_millis(50))? {
                tick = tick.wrapping_add(1);
                continue;
            }
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                use crossterm::event::KeyEventKind;
                if key.kind == KeyEventKind::Press {
                    break;
                }
            }
        }
        drop(term);
        restore_terminal();
    }

    Ok(overall)
}

// --- drawing -----------------------------------------------------------------

fn rainbow_line(text: &str, offset: usize) -> Line<'static> {
    Line::from(
        text.chars()
            .enumerate()
            .map(|(i, c)| {
                Span::styled(
                    c.to_string(),
                    Style::default().fg(RAINBOW[(i + offset) % RAINBOW.len()]),
                )
            })
            .collect::<Vec<_>>(),
    )
}

fn frame_block(tick: u64) -> Block<'static> {
    let border = RAINBOW[(tick as usize / 8) % RAINBOW.len()];
    let mut title = vec![Span::raw(" ")];
    title.extend(rainbow_line("tupdatr", (tick / 3) as usize).spans);
    title.push(Span::raw(" "));
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .title(Line::from(title))
}

fn draw_menu(f: &mut ratatui::Frame, selected: &[bool], highlight: usize, tick: u64, dry_run: bool) {
    let outer = frame_block(tick);
    let area = outer.inner(f.area());
    f.render_widget(outer, f.area());

    let n_selected = selected.iter().filter(|&&s| s).count();

    let mut header = vec![Span::styled(
        format!(" {} update routine", if dry_run { "DRY RUN —" } else { "" }).trim_start().to_string(),
        Style::default().fg(if dry_run {
            ratatui::style::Color::Yellow
        } else {
            ratatui::style::Color::Gray
        }),
    )];
    if dry_run {
        header.push(Span::styled(
            "  nothing will actually run",
            Style::default().fg(ratatui::style::Color::DarkGray),
        ));
    }
    header.push(Span::raw("   "));
    header.push(Span::styled(
        format!("{}/{} selected", n_selected, selected.len()),
        Style::default()
            .fg(ratatui::style::Color::Black)
            .bg(RAINBOW[(tick as usize / 4) % RAINBOW.len()])
            .add_modifier(Modifier::BOLD),
    ));

    let label_col = ACTIONS
        .iter()
        .map(|a| a.label.len())
        .max()
        .unwrap_or(8)
        .max(8);

    let mut lines: Vec<Line> = vec![Line::from(header), Line::from("")];
    for (i, action) in ACTIONS.iter().enumerate() {
        let hl = i == highlight;
        let sel = selected[i];
        let accent = RAINBOW[i % RAINBOW.len()];
        let base = if hl {
            Style::default().bg(ratatui::style::Color::DarkGray)
        } else {
            Style::default()
        };
        let mark = if sel { "✔" } else { "·" };
        let mark_style = if sel {
            Style::default().fg(ratatui::style::Color::LightGreen)
        } else {
            Style::default().fg(ratatui::style::Color::DarkGray)
        };

        let row = Line::from(vec![
            Span::styled(
                if hl { "▸ " } else { "  " },
                base.fg(accent).add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("{} ", mark), base.patch(mark_style)),
            Span::styled(
                format!("{:<width$} ", action.label, width = label_col),
                base.fg(if hl {
                    ratatui::style::Color::White
                } else {
                    accent
                })
                .add_modifier(if hl { Modifier::BOLD } else { Modifier::empty() }),
            ),
            Span::styled(action.description, base.fg(ratatui::style::Color::DarkGray)),
        ]);
        lines.push(row);
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        FOOTER_TEXT,
        Style::default().fg(ratatui::style::Color::DarkGray),
    )));

    f.render_widget(Paragraph::new(lines), area);
}

fn draw_summary(f: &mut ratatui::Frame, results: &[(usize, i32)], tick: u64, dry_run: bool) {
    let outer = frame_block(tick);
    let area = outer.inner(f.area());
    f.render_widget(outer, f.area());

    let failed = results.iter().filter(|(_, s)| *s != 0).count();
    let (verdict, verdict_color) = if failed == 0 {
        ("all done", ratatui::style::Color::LightGreen)
    } else {
        ("finished with failures", ratatui::style::Color::LightRed)
    };

    // Animated progress bar: every slot fills as its action completed.
    const BAR_W: usize = 24;
    let done = results.len();
    let mut bar: Vec<Span> = Vec::with_capacity(BAR_W);
    for i in 0..BAR_W {
        let filled = i * results.len() < done.saturating_mul(BAR_W);
        let color = if filled && failed == 0 {
            ratatui::style::Color::LightGreen
        } else if filled {
            ratatui::style::Color::LightRed
        } else {
            ratatui::style::Color::DarkGray
        };
        bar.push(Span::styled(
            if filled { "█" } else { "░" },
            Style::default().fg(color),
        ));
    }

    let spinner = SPINNER[(tick as usize / 2) % SPINNER.len()];

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled(
                format!(" {} ", spinner),
                Style::default().fg(RAINBOW[(tick as usize / 2) % RAINBOW.len()]),
            ),
            Span::styled(
                verdict.to_string(),
                Style::default()
                    .fg(verdict_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(bar),
        Line::from(""),
    ];

    for &(idx, status) in results {
        let action = &ACTIONS[idx];
        let accent = RAINBOW[idx % RAINBOW.len()];
        let (glyph, glyph_style, note) = if dry_run {
            ("→", Style::default().fg(ratatui::style::Color::Yellow), "would run".to_string())
        } else if status == 0 {
            ("✔", Style::default().fg(ratatui::style::Color::LightGreen), "ok".to_string())
        } else {
            (
                "✘",
                Style::default().fg(ratatui::style::Color::LightRed),
                format!("failed, exit {}", status),
            )
        };
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", glyph), glyph_style),
            Span::styled(
                format!("{:<width$}", action.label, width = label_col_width()),
                Style::default().fg(accent),
            ),
            Span::styled(
                note,
                Style::default().fg(if status == 0 || dry_run {
                    ratatui::style::Color::DarkGray
                } else {
                    ratatui::style::Color::LightRed
                }),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "press any key to exit",
        Style::default()
            .fg(ratatui::style::Color::DarkGray)
            .add_modifier(if tick % 20 < 12 { Modifier::BOLD } else { Modifier::empty() }),
    )));

    f.render_widget(
        Paragraph::new(lines).alignment(Alignment::Left),
        area,
    );
}

fn label_col_width() -> usize {
    ACTIONS
        .iter()
        .map(|a| a.label.len())
        .max()
        .unwrap_or(8)
        .max(8)
}
