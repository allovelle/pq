use crossterm::style::Stylize;
use pq::PqResult;
use pq::diagnostics::UsageReport;
use pq::tok::{Act::*, CharMatch::*, State::*, state_transition_table};

// fn main() -> PqResult<()>
// {
//     let mut report = UsageReport::new();

//     UsageReport::emit_state_transition_table(&state_transition_table()[..]);

//     let _json = r#"
//         "init"
//         818
//         -818
//         "\n"
//         true, false
//         null
//         0.22
//         -0.22
//         8.18e+2
//         81800e-2
//         818e+2
//         81800.0e-2
//         []
//         {}
//         [0]
//         [0,1,2,3]
//         {"a":0,"b":1,"c":2}
//         [0, 1, 2, 3]
//         {"a": 0, "b": 1, "c": 2}
//     "#;

//     let json = std::fs::read_to_string("json0.jsonl")?;

//     for line in json.lines().filter(|line| {
//         let trim = line.trim();
//         !trim.is_empty() && !trim.starts_with("//")
//     })
//     {
//         if let Err(err) = tokenize(line, &mut usage_report)
//         {
//             usage_report.error();
//             println!("{}", format!("{err}").red().bold());
//             println!("{}", format!("tokenizing line: {}", line).red().italic());
//         }
//     }

//     usage_report.report();

//     Ok(())
// }

// TODO: Rework tokenize() so that it works off of a stream of characters and
// TODO: lazily produces tokens to an output stream

pub async fn tokenize_stream(stream: &mut impl Iterator<Item = char>)
{
    if let Some(ch) = stream.next()
    {}
}

// file stream |> tokenizer |> parser

use tokio::fs::File;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

#[tokio::main(flavor = "multi_thread", worker_threads = 3)]
async fn main() -> io::Result<()>
{
    let (tx_in, mut rx_in) = mpsc::channel::<String>(8);
    let (tx_out, mut rx_out) = mpsc::channel::<String>(8);

    /* ---------------- INPUT ---------------- */

    tokio::spawn(async move {
        let mut stdin = io::stdin();

        loop
        {
            let mut buf = String::new();
            let n = stdin.read_to_string(&mut buf).await.unwrap();
            if n == 0
            {
                break;
            }
            tx_in.send(buf).await.unwrap();
        }
        // tx_in dropped → downstream shuts down automatically
    });

    /* --------------- PROCESS --------------- */

    tokio::spawn(async move {
        while let Some(input) = rx_in.recv().await
        {
            let output = process(input);
            tx_out.send(output).await.unwrap();
        }
        // tx_out dropped
    });

    /* ---------------- OUTPUT --------------- */

    // ? let mut file = File::create("out.txt").await?;
    let mut stdout = io::stdout();

    while let Some(msg) = rx_out.recv().await
    {
        stdout.write_all(msg.as_bytes()).await?;
        stdout.flush().await?;
        // ? file.write_all(msg.as_bytes()).await?;
    }

    Ok(())
}

fn process(mut s: String) -> String
{
    s.make_ascii_uppercase();
    s
}
