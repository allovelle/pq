// TODO: Determine if in pipe
// TODO: Determine isatty (why different from pipe)?
// TODO: Determine if output is pipe (don't write colors to file)
// TODO: printf > pq > json.json{l}
// TODO: printf > pq | wc -l
// TODO: printf > pq
// TODO: pq somefile.json

// TODO: Live queries allow TTY input to type the query

/// | Input side        | Output side     | Notes                                                 |
/// |------------------:|:----------------|------------------------------------------------------:|
/// | stdin = TTY       | stdout = TTY    | interactive mode, colors ok                           |
/// | stdin = TTY       | stdout = pipe   | interactive input, output consumed by another process |
/// | stdin = TTY       | stdout = file   | interactive input, output redirected to file          |
/// | stdin = pipe      | stdout = TTY    | pipeline input, pretty/color output                   |
/// | stdin = pipe      | stdout = pipe   | pipeline both ways, no colors                         |
/// | stdin = pipe      | stdout = file   | pipeline input, file output                           |
/// | stdin = file      | stdout = TTY    | file input, pretty/color output                       |
/// | stdin = file      | stdout = pipe   | file input, output consumed                           |
/// | stdin = file      | stdout = file   | file input, file output                               |
/// | no stdin/file     | stdout = TTY    | show usage/help or REPL                               |
/// | multiple files    | any stdout      | sequential or merged processing                       |
/// | stderr = TTY/pipe | -               | decide on colored error messages                      |
fn main() {}

enum JsonInputType
{
    /// Typed from TTY with enter to submit, or piped in from another process
    Stdin,

    /// Path with `.json` extension, validate pure JSON, no JSONC, etc.
    Json(String),

    /// [JSON With comments](https://jsonc.org). Mode line: breaks JSONL compat.
    /// Path with `.jsonc` extension, validate to be pure JSONC, no JSONL, etc.
    Jsonc(String),

    /// [JSON Lines](https://jsonlines.org). Does not overlap with JSONC.
    /// Path with `.jsonl` extension, validate to be pure JSONL, no JSONC, etc.
    Jsonl(String),

    /// [JSON for humans](https://json5.org). Overlaps with JSONC (commas, etc.)
    /// Path with `.json5` extension, validate to be pure JSON5, no JSONL.
    Json5(String),
}

/// ----------------------------------------------------------------------------
enum InputSource
{
    StdinPipe,
    StdinTTY,
    StdinFile,
    NoInput,
    MultipleFiles(Vec<String>),
    StdinAndFile(String), // ambiguous case
}

enum OutputDestination
{
    StdoutPipe,
    StdoutTTY,
    StdoutFile(String),
}

enum StderrDestination
{
    StderrTTY,
    StderrPipe,
    StderrFile(String),
}
enum InOutErrContext
{
    StdinPipeStdoutTTY,
    StdinPipeStdoutPipe,
    StdinPipeStdoutFile(String),
    StdinTTYStdoutTTY,
    StdinTTYStdoutPipe,
    StdinTTYStdoutFile(String),
    StdinFileStdoutTTY(String),
    StdinFileStdoutPipe(String),
    StdinFileStdoutFile(String, String),
    NoInputStdoutTTY,
    MultipleFilesAnyOutput(Vec<String>, OutputDestination),
    StderrContext(StderrDestination),
}

/*
• Input side
    ◦ stdin is a pipe (printf > pq …)
    ◦ stdin is a TTY (pq waiting for interactive input)
    ◦ stdin is a file (pq somefile.json)
• Output side
    ◦ stdout is a pipe (pq | wc -l)
    ◦ stdout is a TTY (pq printing to terminal, maybe with colors)
    ◦ stdout is a file (pq > json.json)

You’ve got the core cases already (pipe vs tty vs file). The missing ones are:
• no input (usage/help/REPL),
• multiple inputs,
• stdin + file combined,
• stderr context,
• Windows text vs binary mode,
• empty input.

• No input at all
    ◦ User invoked pq with no stdin and no file arguments.
    ◦ Decide: show usage/help, or wait for interactive input.
• Multiple input sources
    ◦ pq file.json another.json — reading multiple files sequentially.
    ◦ Different from single file or stdin.
• Stdin + args combined
    ◦ printf … | pq file.json — ambiguous: do you merge stdin and file input, or error?
• Stderr context
    ◦ Detect if stderr is a TTY (for colored error messages).
    ◦ Often overlooked, but important if you want consistent UX.
• Binary vs text mode
    ◦ On Windows, you may need to distinguish between text mode (CRLF translation) and raw binary mode for stdin/stdout.
• isatty vs pipe nuance
    ◦ isatty tells you if the fd is a terminal.
    ◦ “pipe” means the fd is not a terminal and is connected to another process.
    ◦ You might also encounter redirection to a regular file (not a pipe, not a tty).
• Empty input
    ◦ Stdin is open but produces zero bytes.
    ◦ Decide: treat as empty JSON array/object, or error.
*/
