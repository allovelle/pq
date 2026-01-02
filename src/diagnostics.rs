use crate::iter::Replayable;
use crate::ret_if;
use crate::tokv1::{Act::*, CharMatch::*, State::*};
use crate::tokv1::{Act::*, CharMatch::*, State::*, *};
use crate::txt::ToDebug;
use crossterm::style::Stylize;
use std::collections::{HashMap, HashSet};
use std::ops::RangeInclusive;
use std::{fmt, hash};
use strum::*;
use thiserror::Error;

use crate::tokv1;

#[derive(Default, Clone)]
pub struct UsageReport
{
    used_transitions: HashSet<Row>,
    expect_transitions: HashSet<Row>,
    errors: usize,
    documents_examined: usize,
    w_state: usize,
    w_tok_act: usize,
    w_ch: usize,
    w_buf: usize,
}

impl UsageReport
{
    pub fn new() -> Self
    {
        let used_transitions = HashSet::with_capacity(max_state_transitions());
        let expect_transitions = HashSet::from_iter(state_transition_table());
        let mut this =
            Self { used_transitions, expect_transitions, ..Default::default() };
        this.w_state = longest_variant_name::<State>();
        this.w_tok_act = longest_variant_name::<Act>() + 2;
        this.w_ch = format!("{:?}", '\u{10FFFF}').len();
        this.w_buf = 8;
        this
    }

    pub fn log_new_document(&self)
    {
        let Self { w_state, w_tok_act, w_ch, w_buf, .. } = self;

        let header = format!(
            "{:<w_state$} {:<w_ch$} {:<w_state$} {:<w_tok_act$} {:<16} {:<16} {:<w_buf$} {:<w_buf$}",
            "State",
            "Char",
            "Next",
            "Act",
            "Accept",
            "Except",
            "PreBuf",
            "EndBuf"
        );
        println!("\n\n\n{}", header.cyan().underlined());
    }

    pub fn log_state_transition(
        &mut self,
        curr: State,
        ch: char,
        next: Row,
        buf: String,
    )
    {
        let row = next;
        self.used_transitions.insert(row);
        ret_if!(row.from == row.onto && row.act == IGN, ());
        let Self { w_state, w_tok_act, w_ch, w_buf, .. } = self;

        println!(
            "{from:<w_state$} {char:<w_ch$} {next:<w_state$} {act:<w_tok_act$} {acc:<16} {exc:<16} {prebuf:<w_buf$} {postbuf:w_buf$}",
            from = format!("{:?}", curr),
            char = format!("{:?}", ch),
            next = format!("{:?}", row.onto),
            act = format!("{:?}", row.act),
            acc = format!("{:?}", row.accept),
            exc = format!("{:?}", row.except),
            prebuf = format!("{:?}", buf),
            postbuf = format!("{:?}   ", match row.act
            {
                FIN => ch.to_string(),
                TOK | ATK => String::new(),
                ACC => format!("{buf}{ch}"),
                IGN => buf.clone(),
                AGN => String::new(),
            })
        );

        if row.onto == State::end_state()
        {
            println!("Hit explicit {} state", "END".underlined());
        }
    }

    pub fn log_end_document(&self, toks: &Vec<Tok>)
    {
        println!("\n{}\n", format!("Tokens: {toks:?}").green());
    }

    pub fn new_document(&mut self)
    {
        self.documents_examined += 1;
    }

    pub fn log_row(&mut self, row: Row)
    {
        self.used_transitions.insert(row);
    }

    pub fn error(&mut self)
    {
        self.errors += 1;
    }

    pub fn final_report(&self)
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

    pub fn emit_state_transition_table(table: &[Row])
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
}
