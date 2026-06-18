use std::fmt;

impl<T: fmt::Debug> ToDebug for T {}

pub trait ToDebug: fmt::Debug
{
    /// Equivalent to `format!("{:?}", thing);`
    fn to_debug(&self) -> String { format!("{self:?}") }

    /// Equivalent to `format!("{:#?}", thing);`
    fn to_long_debug(&self) -> String { format!("{self:#?}") }

    /// A debug view of a debug view (includes the outer quotes)
    fn to_debug_literal(&self) -> String
    {
        format!("{:?}", format!("{}", self.to_debug()))
    }

    /// Standard format does not allow for width & alignment formatting.
    fn to_debug_left(&self, space: usize) -> String
    {
        format!("{:<space$}", format!("{self:?}"))
    }

    /// Standard format does not allow for width & alignment formatting.
    fn to_debug_right(&self, space: usize) -> String
    {
        format!("{:>space$}", format!("{self:?}"))
    }

    /// Standard format does not allow for width & alignment formatting.
    fn to_debug_center(&self, space: usize) -> String
    {
        format!("{:^space$}", format!("{self:?}"))
    }
}

// TODO: This is a pretty formatter for 1-4 byte codepoints to use the U+... fmt
/// **Format ASCII & multi-byte codepoints as either their escape-code format
/// `\u{AB12}` or their Unicode codepoint `U+AB12`.**
pub trait CodepointView
{
    fn fmt_escape(self) -> String;
    fn fmt_unicode(self) -> String;
}

impl CodepointView for char
{
    fn fmt_escape(self) -> String { format!("\\u{:04X}", self as u32) }

    fn fmt_unicode(self) -> String { format!("U+{:04X}", self as u32) }
}
