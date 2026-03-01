use crate::parser::{Row, RowType};
use crossterm::style::Stylize;
use std::io::{self, IsTerminal};

/// Helper to convert bool to usize for indexing
#[inline]
fn udx(b: bool) -> usize
{
    b as usize
}

/// Theme configuration for JSON syntax highlighting
#[derive(Clone)]
pub struct Theme
{
    pub style_key: fn(&str) -> String,
    pub style_quote_key: fn(&str) -> String,
    pub style_open: fn(&str) -> String,
    pub style_end: fn(&str) -> String,
    pub style_quote_val: fn(&str) -> String,
    pub style_txt: fn(&str) -> String,
    pub style_nil: fn(&str) -> String,
    pub style_num: fn(&str) -> String,
    pub style_bit: fn(&str) -> String,
    pub style_punctuation: fn(&str) -> String,
}

impl Theme
{
    /// Colored theme using crossterm
    pub fn colored() -> Self
    {
        Self {
            style_key: |s| s.green().to_string(),
            style_quote_key: |s| s.dark_green().to_string(),
            style_open: |s| s.red().to_string(),
            style_end: |s| s.red().to_string(),
            style_quote_val: |s| s.dark_red().to_string(),
            style_txt: |s| s.red().to_string(),
            style_nil: |s| s.red().to_string(),
            style_num: |s| s.cyan().to_string(),
            style_bit: |s| s.yellow().to_string(),
            style_punctuation: |s| s.white().to_string(),
        }
    }

    /// Plain theme (no colors)
    pub fn plain() -> Self
    {
        Self {
            style_key: |s| s.to_string(),
            style_quote_key: |s| s.to_string(),
            style_open: |s| s.to_string(),
            style_end: |s| s.to_string(),
            style_quote_val: |s| s.to_string(),
            style_txt: |s| s.to_string(),
            style_nil: |s| s.to_string(),
            style_num: |s| s.to_string(),
            style_bit: |s| s.to_string(),
            style_punctuation: |s| s.to_string(),
        }
    }
}

/// Format configuration
pub struct FormatConfig
{
    pub indent_size: usize,
    pub max_line_length: usize,
    pub theme: Theme,
}

impl Default for FormatConfig
{
    fn default() -> Self
    {
        let use_colors = io::stdout().is_terminal();
        Self {
            indent_size: 4,
            max_line_length: 80,
            theme: if use_colors { Theme::colored() } else { Theme::plain() },
        }
    }
}

impl FormatConfig
{
    pub fn new() -> Self
    {
        Self::default()
    }

    pub fn with_indent(mut self, size: usize) -> Self
    {
        self.indent_size = size;
        self
    }

    pub fn with_max_line_length(mut self, length: usize) -> Self
    {
        self.max_line_length = length;
        self
    }

    pub fn with_colors(mut self, use_colors: bool) -> Self
    {
        self.theme = if use_colors { Theme::colored() } else { Theme::plain() };
        self
    }

    pub fn with_theme(mut self, theme: Theme) -> Self
    {
        self.theme = theme;
        self
    }
}

/// Format the entire table into lines of JSON
pub fn format_table(table: &[Row], config: &FormatConfig) -> Vec<String>
{
    if table.is_empty()
    {
        return vec![];
    }

    let mut lines = Vec::new();
    let mut accumulate_indent = 0;

    for row in table
    {
        let parent = table.get(row.par as usize).unwrap_or(row);
        let next = table.get(row.id as usize + 1).unwrap_or(row);

        // Determine row properties
        let is_parent = row.id == next.par;
        let empty =
            matches!(row.ty, RowType::Obj | RowType::Arr) && row.id != next.par;
        let last = next.id == row.id || next.par < row.par;

        // Format the main row line
        let indent = " ".repeat(accumulate_indent * config.indent_size);

        // Build key part
        let key = if parent.ty != RowType::Arr && row.id != 0
        {
            format!(
                "{}{}{}",
                (config.theme.style_quote_key)("\""),
                (config.theme.style_key)(&row.key),
                (config.theme.style_quote_key)("\"")
            )
        }
        else
        {
            String::new()
        };

        let colon = if row.id != 0 && !key.is_empty()
        {
            (config.theme.style_punctuation)(": ")
        }
        else
        {
            String::new()
        };

        // Build value part
        let val = match row.ty
        {
            RowType::Arr if empty =>
            {
                format!(
                    "{}{}",
                    (config.theme.style_open)("["),
                    (config.theme.style_end)("]")
                )
            }
            RowType::Arr => (config.theme.style_open)("["),
            RowType::Obj if empty =>
            {
                format!(
                    "{}{}",
                    (config.theme.style_open)("{"),
                    (config.theme.style_end)("}")
                )
            }
            RowType::Obj => (config.theme.style_open)("{"),
            RowType::Nil => (config.theme.style_nil)(&row.val),
            RowType::Bit => (config.theme.style_bit)(&row.val),
            RowType::Str =>
            {
                format!(
                    "{}{}{}",
                    (config.theme.style_quote_val)("\""),
                    (config.theme.style_txt)(&row.val),
                    (config.theme.style_quote_val)("\"")
                )
            }
            RowType::Num => (config.theme.style_num)(&row.val),
        };

        let comma = if (empty || !is_parent) && !last
        {
            (config.theme.style_punctuation)(",")
        }
        else
        {
            String::new()
        };

        let line = format!("{}{}{}{}{}", indent, key, colon, val, comma);
        lines.push(line);

        // Update indent for next iteration
        if is_parent && !empty
        {
            accumulate_indent += 1;
        }
        else if last
        {
            accumulate_indent = accumulate_indent.saturating_sub(1);
        }

        // Emit closing brackets by walking up the tree
        let mut node = row;
        let mut increment_dedent = accumulate_indent;

        while node.par > next.par || next.id == node.id
        {
            let parent_node = table.get(node.par as usize).unwrap_or(node);

            // Determine if we need a closing bracket
            let brace = if node.par > next.par || node.id == next.id
            {
                if parent_node.ty == RowType::Obj
                {
                    (config.theme.style_end)("}")
                }
                else
                {
                    (config.theme.style_end)("]")
                }
            }
            else
            {
                String::new()
            };

            // Next node is sibling of current node's parent
            let comma = if next.par == parent_node.par
            {
                (config.theme.style_punctuation)(",")
            }
            else
            {
                String::new()
            };

            if !brace.is_empty()
            {
                let indent = " ".repeat(increment_dedent * config.indent_size);
                let line = format!("{}{}{}", indent, brace, comma);
                lines.push(line);
            }

            node = parent_node;
            increment_dedent = increment_dedent.saturating_sub(1);
        }

        // Adjust accumulate_indent if needed
        if accumulate_indent > increment_dedent + 1
        {
            accumulate_indent = accumulate_indent.saturating_sub(1);
        }

        // Handle the final root closing bracket
        if let Some(root) = table.first()
        {
            let last_before_root = next.id == row.id;
            if last_before_root
            {
                let indent = " ".repeat(increment_dedent * config.indent_size);
                let end = if root.ty == RowType::Obj
                {
                    (config.theme.style_end)("}")
                }
                else
                {
                    (config.theme.style_end)("]")
                };
                let line = format!("{}{}", indent, end);
                lines.push(line);
            }
        }
    }

    lines
}

/// Check if a row and its children can fit on one line (within max_length)
fn can_inline(table: &[Row], row_idx: usize, max_length: usize) -> bool
{
    let row = &table[row_idx];

    // Only consider inlining objects and arrays
    if !matches!(row.ty, RowType::Obj | RowType::Arr)
    {
        return false;
    }

    // Calculate the projected length if inlined
    let projected = calculate_inline_length(table, row_idx);
    projected <= max_length
}

/// Calculate the length if this row and its children were inlined
fn calculate_inline_length(table: &[Row], row_idx: usize) -> usize
{
    let row = &table[row_idx];
    let mut length = 0;

    // Opening bracket
    length += 1;

    // Get all direct children
    let children: Vec<_> =
        table.iter().enumerate().filter(|(_, r)| r.par == row.id).collect();

    for (idx, (child_idx, child)) in children.iter().enumerate()
    {
        // Key (if object)
        if row.ty == RowType::Obj
        {
            length += child.key.len() + 4; // "key":
        }

        // Value
        match child.ty
        {
            RowType::Str => length += child.val.len() + 2, // "value"
            RowType::Num | RowType::Bit | RowType::Nil =>
            {
                length += child.val.len()
            }
            RowType::Obj | RowType::Arr =>
            {
                // Recursively check nested structures
                length += calculate_inline_length(table, *child_idx);
            }
        }

        // Comma and space between elements
        if idx < children.len() - 1
        {
            length += 2; // ", "
        }
    }

    // Closing bracket
    length += 1;

    length
}

/// Format table with inlining for small structures (respects max_line_length)
pub fn format_table_compact(table: &[Row], config: &FormatConfig)
-> Vec<String>
{
    // TODO: Implement smart inlining based on max_line_length
    // For now, delegate to standard formatter
    format_table(table, config)
}

/// Print formatted JSON to stdout
pub fn print_formatted(table: &[Row], config: &FormatConfig)
{
    for line in format_table(table, config)
    {
        println!("{}", line);
    }
}

/// Format and return as a single string
pub fn format_to_string(table: &[Row], config: &FormatConfig) -> String
{
    format_table(table, config).join("\n")
}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::parser::*;

    #[test]
    fn test_format_simple_array()
    {
        let json = r#"[1, 2, 3]"#;
        let table = parse_from_str(json).unwrap();
        let config = FormatConfig::new().with_colors(false);
        let lines = format_table(&table, &config);

        // Should produce formatted output
        assert!(!lines.is_empty());
        println!("Formatted array:");
        for line in &lines
        {
            println!("{}", line);
        }
    }

    #[test]
    fn test_format_nested_object()
    {
        let json = r#"{"name": "test", "nested": {"key": "value"}}"#;
        let table = parse_from_str(json).unwrap();
        let config = FormatConfig::new().with_colors(false);
        let lines = format_table(&table, &config);

        assert!(!lines.is_empty());
        println!("Formatted object:");
        for line in &lines
        {
            println!("{}", line);
        }
    }

    #[test]
    fn test_format_complex()
    {
        let json = r#"[1, 2, [3, 4], 5, 6]"#;
        let table = parse_from_str(json).unwrap();
        let config = FormatConfig::new().with_colors(false);
        let lines = format_table(&table, &config);

        assert!(!lines.is_empty());
        println!("Formatted complex:");
        for line in &lines
        {
            println!("{}", line);
        }
    }

    #[test]
    fn test_format_with_colors()
    {
        let json = r#"{"key": "value", "num": 42}"#;
        let table = parse_from_str(json).unwrap();
        let config = FormatConfig::new().with_colors(true);
        let lines = format_table(&table, &config);

        println!("Formatted with colors:");
        for line in &lines
        {
            println!("{}", line);
        }
    }
}
