//! Terminal numbered-menu confirm loop: accept / edit-one / drop-one / abort.
//!
//! No prior pattern in this codebase to mirror — `AskUserQuestion` is MCP/agent-side, not a
//! terminal primitive. Built once here (ADR 0017 Decision D) and reused by both `vd-meeting`'s
//! meeting-input wizard and `vd-pipeline`'s single-file audio wizard (Decision E), so the two
//! interactive modes share one loop instead of two hand-rolled ones.
//!
//! `input` / `output` are injected (not raw stdin/stdout) so the loop is unit-testable without
//! a real terminal — callers wire `io::stdin().lock()` / `io::stdout()` at the CLI edge.

use std::io::{self, BufRead, Write};

/// One row the wizard proposes; `label` is the one-line summary shown in the menu and must be
/// refreshed by the caller (via `render`) after every edit.
#[derive(Debug, Clone)]
pub struct MenuItem<T> {
    pub value: T,
    pub label: String,
}

impl<T> MenuItem<T> {
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
        }
    }
}

/// Result of running the menu loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuOutcome {
    /// User accepted the (possibly edited/pruned) list — proceed with what remains in `items`.
    Accepted,
    /// User aborted (`q` / `quit`) — caller must not proceed.
    Aborted,
}

/// Run the confirm loop over `items` in place.
///
/// Each iteration prints `render(&item.value)` as `N. {label}` for every remaining item, then
/// reads one command line from `input`:
///
/// - blank / `y` / `yes` / `a` / `accept` → stop looping, [`MenuOutcome::Accepted`]
/// - a bare number `N` (1-based) → calls `edit_one(&mut items[N-1].value, input, output)`,
///   refreshes `items[N-1].label` via `render`, loops again
/// - `d N` / `drop N` → removes item `N`, loops again (dropping the last remaining item is
///   allowed — the caller decides whether an empty list is a usage error)
/// - `q` / `quit` → stop looping, [`MenuOutcome::Aborted`]
/// - anything else → prints a short usage hint, loops again without changing `items`
///
/// EOF on `input` (no more lines) is treated as `q` — a non-interactive/piped caller that
/// forgot to answer must not spin forever or silently accept an unconfirmed list.
pub fn run<T>(
    items: &mut Vec<MenuItem<T>>,
    render: impl Fn(&T) -> String,
    mut edit_one: impl FnMut(&mut T, &mut dyn BufRead, &mut dyn Write) -> io::Result<()>,
    input: &mut dyn BufRead,
    output: &mut dyn Write,
) -> io::Result<MenuOutcome> {
    loop {
        for (i, item) in items.iter().enumerate() {
            writeln!(output, "{}. {}", i + 1, item.label)?;
        }
        write!(
            output,
            "[Enter/y]=accept  N=edit  d N=drop  q=abort > "
        )?;
        output.flush()?;

        let mut line = String::new();
        let bytes_read = input.read_line(&mut line)?;
        if bytes_read == 0 {
            // EOF: never treat silence as consent.
            return Ok(MenuOutcome::Aborted);
        }
        let cmd = line.trim();

        if cmd.is_empty() || matches!(cmd.to_ascii_lowercase().as_str(), "y" | "yes" | "a" | "accept") {
            return Ok(MenuOutcome::Accepted);
        }
        if matches!(cmd.to_ascii_lowercase().as_str(), "q" | "quit") {
            return Ok(MenuOutcome::Aborted);
        }

        if let Some(rest) = cmd
            .strip_prefix("d ")
            .or_else(|| cmd.strip_prefix("drop "))
        {
            if let Some(idx) = parse_index(rest, items.len()) {
                items.remove(idx);
            } else {
                writeln!(output, "not a valid item number: {rest}")?;
            }
            continue;
        }

        match parse_index(cmd, items.len()) {
            Some(idx) => {
                edit_one(&mut items[idx].value, input, output)?;
                items[idx].label = render(&items[idx].value);
            }
            None => {
                writeln!(
                    output,
                    "unrecognized: {cmd:?} — Enter to accept, a number to edit, \"d N\" to drop, q to abort"
                )?;
            }
        }
    }
}

fn parse_index(raw: &str, len: usize) -> Option<usize> {
    let n: usize = raw.trim().parse().ok()?;
    if n == 0 || n > len {
        return None;
    }
    Some(n - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn items() -> Vec<MenuItem<String>> {
        vec![
            MenuItem::new("alice".to_string(), "alice (participant)"),
            MenuItem::new("mix".to_string(), "mix (room)"),
        ]
    }

    fn no_edit(_v: &mut String, _r: &mut dyn BufRead, _w: &mut dyn Write) -> io::Result<()> {
        Ok(())
    }

    #[test]
    fn blank_line_accepts() {
        let mut it = items();
        let mut input = Cursor::new(b"\n".to_vec());
        let mut output = Vec::new();
        let outcome = run(&mut it, String::clone, no_edit, &mut input, &mut output).unwrap();
        assert_eq!(outcome, MenuOutcome::Accepted);
        assert_eq!(it.len(), 2);
    }

    #[test]
    fn yes_accepts() {
        let mut it = items();
        let mut input = Cursor::new(b"yes\n".to_vec());
        let mut output = Vec::new();
        let outcome = run(&mut it, String::clone, no_edit, &mut input, &mut output).unwrap();
        assert_eq!(outcome, MenuOutcome::Accepted);
    }

    #[test]
    fn quit_aborts() {
        let mut it = items();
        let mut input = Cursor::new(b"q\n".to_vec());
        let mut output = Vec::new();
        let outcome = run(&mut it, String::clone, no_edit, &mut input, &mut output).unwrap();
        assert_eq!(outcome, MenuOutcome::Aborted);
    }

    #[test]
    fn eof_aborts_instead_of_hanging() {
        let mut it = items();
        let mut input = Cursor::new(Vec::new());
        let mut output = Vec::new();
        let outcome = run(&mut it, String::clone, no_edit, &mut input, &mut output).unwrap();
        assert_eq!(outcome, MenuOutcome::Aborted);
    }

    #[test]
    fn drop_removes_item_then_accepts() {
        let mut it = items();
        let mut input = Cursor::new(b"d 1\ny\n".to_vec());
        let mut output = Vec::new();
        let outcome = run(&mut it, String::clone, no_edit, &mut input, &mut output).unwrap();
        assert_eq!(outcome, MenuOutcome::Accepted);
        assert_eq!(it.len(), 1);
        assert_eq!(it[0].value, "mix");
    }

    #[test]
    fn drop_long_form_works_too() {
        let mut it = items();
        let mut input = Cursor::new(b"drop 2\ny\n".to_vec());
        let mut output = Vec::new();
        run(&mut it, String::clone, no_edit, &mut input, &mut output).unwrap();
        assert_eq!(it.len(), 1);
        assert_eq!(it[0].value, "alice");
    }

    #[test]
    fn edit_replaces_value_and_refreshes_label() {
        let mut it = items();
        let mut input = Cursor::new(b"1\ny\n".to_vec());
        let mut output = Vec::new();
        let edit = |v: &mut String, _r: &mut dyn BufRead, _w: &mut dyn Write| -> io::Result<()> {
            *v = "Alice".to_string();
            Ok(())
        };
        run(&mut it, |v| format!("edited:{v}"), edit, &mut input, &mut output).unwrap();
        assert_eq!(it[0].value, "Alice");
        assert_eq!(it[0].label, "edited:Alice");
    }

    #[test]
    fn out_of_range_number_reprompts_without_changes() {
        let mut it = items();
        let mut input = Cursor::new(b"9\ny\n".to_vec());
        let mut output = Vec::new();
        let outcome = run(&mut it, String::clone, no_edit, &mut input, &mut output).unwrap();
        assert_eq!(outcome, MenuOutcome::Accepted);
        assert_eq!(it.len(), 2);
        let printed = String::from_utf8(output).unwrap();
        assert!(printed.contains("unrecognized"));
    }

    #[test]
    fn garbage_input_reprompts_without_changes() {
        let mut it = items();
        let mut input = Cursor::new(b"whatever\ny\n".to_vec());
        let mut output = Vec::new();
        let outcome = run(&mut it, String::clone, no_edit, &mut input, &mut output).unwrap();
        assert_eq!(outcome, MenuOutcome::Accepted);
        assert_eq!(it.len(), 2);
    }
}
