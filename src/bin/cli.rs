use clap::Parser;
use std::{io::IsTerminal, path::PathBuf};

/// pique (pq) — jsonl formatter, highlighter, querier, and interactive explorer
///
/// stdin is always consumed if present, treated as an unnamed input file.
/// all inputs are concatenated into a single jsonl stream. no slurp mode.
///
/// Examples:
///   pq                           → print this help and exit
///   pq .                         → open interactive starlark console
///   pq 'expr'                    → format + query stdin
///   pq -f json.json              → format + highlight
///   pq -f json.json 'expr'       → query file
///   pq -f json.json .            → interactive console over file
///   pq -f a.json -f b.json       → merged stream, format + highlight
///
///   cat f.json | pq              → format + highlight stdin
///   cat f.json | pq .            → interactive console over stdin
///   cat f.json | pq 'expr'       → query stdin
///   cat f.json | pq -f json.json → merged stream, format + highlight
#[derive(Parser, Debug)]
#[command(
    name = "pq",
    about = "pique — jsonl formatter, highlighter, querier, and interactive explorer",
    // Disable the default --help short flag so `-h` can remain free if desired,
    // but keep long --help. Clap will still show help on bare `pq` via override below.
    disable_help_flag = false,
    // Don't auto-exit on `pq` with no args — we handle that ourselves so we can
    // distinguish "no args" (print help) from other modes.
    arg_required_else_help = false,
)]
pub struct Cli
{
    /// Input files to read. May be repeated. All files and stdin (if present)
    /// are concatenated into a single JSONL stream.
    #[arg(
        short = 'f',
        long = "file",
        value_name = "FILE",
        value_hint = clap::ValueHint::FilePath,
    )]
    pub files: Vec<PathBuf>,

    /// Query expression or "." for interactive mode.
    ///
    /// Omit entirely to format + highlight with no filtering.
    /// Pass "." to open the interactive Starlark console.
    /// Pass any other string to evaluate it as a Starlark query expression.
    #[arg(
        value_name = "QUERY",
        // Only one positional argument is accepted.
        num_args = 0..=1,
    )]
    pub query: Option<String>,
}

// ---------------------------------------------------------------------------
// Resolved mode — what the runtime should actually do.
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub enum Mode
{
    /// `pq` with no args and no stdin: print help and exit.
    Help,

    /// Format + syntax-highlight the JSONL stream, no filtering.
    /// Triggered when no query is given (files and/or stdin present).
    FormatHighlight,

    /// Evaluate a Starlark expression over the JSONL stream.
    Query(String),

    /// Open the interactive Starlark console over the JSONL stream.
    Interactive,
}

impl Cli
{
    /// Resolve the parsed CLI arguments into a concrete [`Mode`].
    ///
    /// `has_stdin` should be `true` when the process is connected to a pipe
    /// (i.e. `!std::io::stdin().is_terminal()`).
    pub fn mode(&self, has_stdin: bool) -> Mode
    {
        let has_input = has_stdin || !self.files.is_empty();

        match &self.query
        {
            // No query arg at all.
            None =>
            {
                if has_input
                {
                    Mode::FormatHighlight
                }
                else
                {
                    // `pq` bare — no files, no stdin, no query → help.
                    Mode::Help
                }
            }

            Some(q) if q == "." => Mode::Interactive,

            Some(expr) => Mode::Query(expr.clone()),
        }
    }
}

// ---------------------------------------------------------------------------
// Example main showing how to wire it all up.
// ---------------------------------------------------------------------------

// * Invoke with:
// * bat json.json | cargo run --bin cli -- k1.k2.k3 -f json.json -f json.json
fn main()
{
    let cli = Cli::parse();

    let has_stdin = !std::io::stdin().is_terminal();

    let mode = cli.mode(has_stdin);

    match mode
    {
        Mode::Help =>
        {
            use clap::CommandFactory;
            Cli::command().print_help().unwrap();
            println!();
            std::process::exit(0);
        }

        Mode::FormatHighlight =>
        {
            eprintln!(
                "[FormatHighlight] files={:?}, stdin={}",
                cli.files, has_stdin
            );
        }

        Mode::Query(expr) =>
        {
            eprintln!(
                "[Query] expr={:?}, files={:?}, stdin={}",
                expr, cli.files, has_stdin
            );
        }

        Mode::Interactive =>
        {
            eprintln!(
                "[Interactive] files={:?}, stdin={}",
                cli.files, has_stdin
            );
        }
    }
}
// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests
{
    use super::*;
    use clap::Parser;

    fn parse(args: &[&str]) -> Cli
    {
        Cli::parse_from(std::iter::once("pq").chain(args.iter().copied()))
    }

    // ---- mode resolution ---------------------------------------------------

    #[test]
    fn bare_no_stdin_is_help()
    {
        let cli = parse(&[]);
        assert_eq!(cli.mode(false), Mode::Help);
    }

    #[test]
    fn bare_with_stdin_is_format_highlight()
    {
        let cli = parse(&[]);
        assert_eq!(cli.mode(true), Mode::FormatHighlight);
    }

    #[test]
    fn dot_no_stdin_is_interactive()
    {
        let cli = parse(&["."]);
        assert_eq!(cli.mode(false), Mode::Interactive);
    }

    #[test]
    fn dot_with_stdin_is_interactive()
    {
        let cli = parse(&["."]);
        assert_eq!(cli.mode(true), Mode::Interactive);
    }

    #[test]
    fn expr_no_stdin_is_query()
    {
        let cli = parse(&[".foo"]);
        assert_eq!(cli.mode(false), Mode::Query(".foo".into()));
    }

    #[test]
    fn expr_with_stdin_is_query()
    {
        let cli = parse(&[".bar"]);
        assert_eq!(cli.mode(true), Mode::Query(".bar".into()));
    }

    #[test]
    fn file_only_no_stdin_is_format_highlight()
    {
        let cli = parse(&["-f", "a.json"]);
        assert_eq!(cli.mode(false), Mode::FormatHighlight);
    }

    #[test]
    fn file_only_with_stdin_is_format_highlight()
    {
        let cli = parse(&["-f", "a.json"]);
        assert_eq!(cli.mode(true), Mode::FormatHighlight);
    }

    #[test]
    fn file_with_expr_is_query()
    {
        let cli = parse(&["-f", "a.json", ".name"]);
        assert_eq!(cli.mode(false), Mode::Query(".name".into()));
    }

    #[test]
    fn file_with_dot_is_interactive()
    {
        let cli = parse(&["-f", "a.json", "."]);
        assert_eq!(cli.mode(false), Mode::Interactive);
    }

    #[test]
    fn multiple_files_no_query_is_format_highlight()
    {
        let cli = parse(&["-f", "a.json", "-f", "b.json"]);
        assert_eq!(cli.mode(false), Mode::FormatHighlight);
    }

    #[test]
    fn stdin_and_file_with_expr_is_query()
    {
        let cli = parse(&["-f", "a.json", ".count"]);
        assert_eq!(cli.mode(true), Mode::Query(".count".into()));
    }

    // ---- field contents ----------------------------------------------------

    #[test]
    fn files_are_collected()
    {
        let cli = parse(&["-f", "a.json", "-f", "b.json"]);
        assert_eq!(cli.files.len(), 2);
    }

    #[test]
    fn no_files_when_omitted()
    {
        let cli = parse(&[".foo"]);
        assert!(cli.files.is_empty());
    }

    #[test]
    fn query_is_none_when_omitted()
    {
        let cli = parse(&["-f", "a.json"]);
        assert!(cli.query.is_none());
    }
}
