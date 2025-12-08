use crossterm::style::Stylize;
use pq::iter::Replayable;
use pq::ret_if;
use std::collections::{HashMap, HashSet};
use std::ops::RangeInclusive;
use std::{fmt, hash};
use strum::*;
use thiserror::Error;
use {Act::*, CharMatch::*, State::*};

pub struct UsageReport
{
    used_transitions: HashSet<Row>,
    expect_transitions: HashSet<Row>,
    errors: usize,
    documents_examined: usize,
}

impl UsageReport
{
    fn new() -> Self
    {
        let used_transitions = HashSet::with_capacity(max_state_transitions());
        let expect_transitions = HashSet::from_iter(state_transition_table());
        let (documents_examined, errors) = (0, 0);
        Self {
            used_transitions,
            expect_transitions,
            documents_examined,
            errors,
        }
    }

    fn new_document(&mut self)
    {
        self.documents_examined += 1;
    }

    fn log_row(&mut self, row: Row)
    {
        self.used_transitions.insert(row);
    }

    fn error(&mut self)
    {
        self.errors += 1;
    }

    fn report(&self)
    {
        let dbg_msg = format!(
            "Hit {} out of {} state transitions, missed:",
            self.used_transitions.len(),
            self.expect_transitions.len(),
        );

        let style = if self.used_transitions.len()
            < self.expect_transitions.len()
        {
            <String as Stylize>::yellow
        }
        else
        {
            <String as Stylize>::reset
        };

        println!("{}", style(dbg_msg));

        if self.used_transitions.len() < self.expect_transitions.len()
        {
            for unused in self
                .expect_transitions
                .difference(&self.used_transitions)
                .take(4)
            {
                println!("{}", style(unused.to_debug()))
            }
            println!("{}", style("...".to_string()));
            println!("{}", style("...".to_string()));
        }

        println!("Hit {} errors", self.errors.to_string().red());
    }
}

fn emit_table(table: &[Row])
{
    let state = longest_variant_name::<State>();
    let tok_act = longest_variant_name::<Act>();
    let accept = {
        table
            .iter()
            .map(|s| {
                // Within('\u{10FFFF}', '\u{10FFFF}');
                let len_acc = format!("{:?}", s.accept).len();
                let len_exc = format!("{:?}", s.except).len();
                len_acc.max(len_exc)
            })
            .max()
            .unwrap_or_default()
    };
    let header = format!(
        "| {:<state$} | {:^accept$} | {:^accept$} | {:<state$} | {:<tok_act$} |",
        "From", "Accept", "Except", "Onto", "Action",
    );

    println!("{}", header.blue().underlined());

    for row in table
    {
        println!(
            "| {fro:<state$} | {acc:^accept$} | {exc:^accept$} | {to:<state$} | {act:<tok_act$} |",
            fro = format!("{:?}", row.from),
            acc = format!("{:?}", row.accept),
            exc = format!("{:?}", row.except),
            to = format!("{:?}", row.onto),
            act = format!("{:?}", row.act),
        );
    }
    println!();
}

fn main() -> PqResult<()>
{
    emit_table(&state_transition_table()[..]);

    let _json = r#"
        "init"
        818
        -818
        "\n"
        true, false
        null
        0.22
        -0.22
        8.18e+2
        81800e-2
        818e+2
        81800.0e-2
        []
        {}
        [0]
        [0,1,2,3]
        {"a":0,"b":1,"c":2}
        [0, 1, 2, 3]
        {"a": 0, "b": 1, "c": 2}
    "#;

    let json = std::fs::read_to_string("json0.jsonl")?;

    let mut usage_report = UsageReport::new();

    for line in json.lines().filter(|line| {
        let trim = line.trim();
        !trim.is_empty() && !trim.starts_with("//")
    })
    {
        if let Err(err) = tokenize(line, &mut usage_report)
        {
            usage_report.error();
            println!("{}", format!("{err}").red().bold());
            println!("{}", format!("tokenizing line: {}", line).red().italic());
        }
    }

    usage_report.report();

    Ok(())
}
