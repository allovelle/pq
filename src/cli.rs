use clap::Parser;
use std::{
    fs::{self, File},
    io::{self, Read},
    path::PathBuf,
    slice, thread,
};

/// Parsed command-line arguments for pique (pq).
///
/// Maps CLI input (file paths, query expression) to concrete runtime modes
/// via the [`mode`](Self::mode) method. See CLI.txt for usage documentation.
#[derive(Parser, Debug)]
#[command(
    about = include_str!("help/short.header.txt").trim_end_matches('\n'),
    after_help = include_str!("help/short.footer.txt"),
    long_about = include_str!("help/long.header.txt"),
    after_long_help = concat!(
        include_str!("help/long.footer.txt"),
        include_str!("help/queries.txt")
    ),
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
            None if has_input => Mode::FormatHighlight,

            // `pq` bare — no files, no stdin, no query:
            // clap will handle help automatically.
            None => Mode::FormatHighlight,

            Some(q) if q == "." => Mode::Interactive,

            Some(expr) => Mode::Query(expr.clone()),
        }
    }

    pub fn total_input_file_lengths(&self)
    -> (usize, Vec<StaticFileDescriptor>)
    {
        let mut descriptors = Vec::with_capacity(self.files.len());
        let mut total_len = 0;
        for file in &self.files
        {
            let file_len = fs::metadata(file).unwrap().len() as usize;
            total_len += file_len;
            descriptors.push(StaticFileDescriptor {
                offset: total_len - file_len,
                length: file_len,
                filename: file.to_string_lossy().into_owned(),
            });
        }
        (total_len, descriptors)
    }

    #[deprecated(note = "Use load_input_files_concurrent for good performance")]
    pub fn load_all_input_files(&self) -> (Vec<u8>, Vec<StaticFileDescriptor>)
    {
        let (total_len, descriptors) = self.total_input_file_lengths();
        let mut buffer = Vec::with_capacity(total_len);
        for file in &self.files
        {
            let mut file_descriptor = File::open(file).unwrap();
            io::copy(&mut file_descriptor, &mut buffer).unwrap();
        }
        (buffer, descriptors)
    }

    /// Load all input files concurrently into a single buffer, returning the
    /// buffer and file descriptors with offsets into the buffer.
    pub fn load_input_files_concurrent(
        &self,
    ) -> (Vec<u8>, Vec<StaticFileDescriptor>)
    {
        // Preallocation prerequisite for buffer length and file offsets
        let (total_len, descriptors) = self.total_input_file_lengths();

        // Preallocate space manually since threads will write directly into it
        // since Vec::with_capacity does not actually set the len of the buffer
        let mut buffer = vec![0u8; total_len];

        thread::scope(|scope| {
            // Buffer base pointer as usize for pointer arithmetic in threads,
            // safe since the buffer is not reallocated or dropped until all
            // threads join due to preallocation
            let buf_base_ptr = buffer.as_mut_ptr() as usize;

            // Not zero copy but very tiny overhead. This is fixable by storing
            // only the end offset of each file and counting backwards
            for desc in descriptors.iter().cloned()
            {
                scope.spawn(move || {
                    // File descriptor shows offset and length so overlap is not
                    // possible as threads write to disjoint regions of the buf
                    let mut file_desc = File::open(&desc.filename).unwrap();
                    let dest = unsafe {
                        slice::from_raw_parts_mut(
                            (buf_base_ptr as *mut u8).add(desc.offset),
                            desc.length,
                        )
                    };
                    file_desc.read_exact(dest).unwrap();
                });
            }
        });

        // Same file descriptors, filled buffer with file contents
        (buffer, descriptors)
    }

    // TODO: We don't care about stdin since it's going to be processed by the
    // TODO: downstream UTF-8 buffer and iter.
    // Pass the stdin descriptor to txt here as it is leaving the user CLI layer

    // ? 1. Make the buffer grow by exact amounts (chunks of stdin) so that we
    // ? 2. don't have wasted space
    // Random access index, seq iteration, iteration from offset, append-only
    // modification, slicing regions, and indices do not invalidate on
    // reallocations since indexing Vec is not based on pointers.
    // ? 3. Slices may have token binding so that there's a sort of 'request->
    // ? 3. response' pattern. In theory, appending to the buffer shouldn't
    // ? 3. invalidate any other systems due to append only nature.

    // TODO: Pass stdio and stdin buffer to the next phase for UTF-8 decoding
    // TODO: and iter

    // TODO: Pass stdio and stdin buffer to the next phase for UTF-8 decoding
    // TODO: and iter

    // TODO: Pass stdio and stdin buffer to the next phase for UTF-8 decoding
    // TODO: and iter
}

/// There will be one less file descriptor than there are files since the last
/// file is stdin.
#[derive(Debug, Clone)]
pub struct StaticFileDescriptor
{
    pub offset: usize,
    pub length: usize,
    pub filename: String,
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

    // ---- total_input_file_lengths -------------------------------------------
    #[test]
    fn total_input_file_lengths()
    {
        let cli = parse(&["-f", "a.json", "-f", "b.json"]);
        let (total_len, descriptors) = cli.total_input_file_lengths();
        assert_eq!(descriptors.len(), 2);
        assert_eq!(total_len, 0); // Assuming empty files for this test
    }

    #[test]
    fn load_all_input_files()
    {
        let cli = parse(&["-f", "a.json", "-f", "b.json"]);
        let (buffer, descriptors) = cli.load_all_input_files();
        assert_eq!(descriptors.len(), 2);
        assert_eq!(buffer.len(), 0); // Assuming empty files for this test
    }
}
