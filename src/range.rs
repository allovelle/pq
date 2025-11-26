use crate::inc::Inc;
use std::ops::{Range, RangeInclusive};

/// Removes the element from the range by creating 2 ranges that exclude that
/// element. If the elemnt is not in the range to begin with, the original range
/// is returned unmodified. Returns `None` if the element isn't in the range.
pub fn split_range<I>(range: Range<I>, by: I) -> Option<(Range<I>, Range<I>)>
where
    I: Copy + PartialOrd + Inc,
{
    match range.contains(&by)
    {
        true => Some((range.start .. by, by.inc() .. range.end)),
        false => None,
    }
}

/// Removes the element from the range by creating 2 ranges that exclude that
/// element. If the elemnt is not in the range to begin with, the original range
/// is returned unmodified. Returns `None` if the element isn't in the range.
pub fn split_range_inclusive<I>(
    range: RangeInclusive<I>,
    by: I,
) -> Option<(Range<I>, Range<I>)>
where
    I: Copy + PartialOrd + Inc,
{
    match range.contains(&by)
    {
        true => Some((*range.start() .. by, by.inc() .. range.end().inc())),
        false => None,
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    fn collect_range(r: std::ops::Range<char>) -> String
    {
        r.collect::<String>()
    }

    #[test]
    fn test_remove_middle_char()
    {
        let r = 'a' .. 'd'; // 'a','b','c'
        let parts = split_range(r, 'b').unwrap();
        let s = collect_range(parts.0) + &collect_range(parts.1);
        assert_eq!(s, "ac"); // 'abc' - 'b' = 'ac'
    }

    #[test]
    fn test_remove_first_char()
    {
        let r = 'a' .. 'd'; // 'a','b','c'
        let parts = split_range(r, 'a').unwrap();
        let s = collect_range(parts.0) + &collect_range(parts.1);
        assert_eq!(s, "bc"); // 'abc' - 'a' = 'bc'
    }

    #[test]
    fn test_remove_last_char_exclusive()
    {
        let r = 'a' .. 'd'; // 'a','b','c'
        assert!(split_range(r.clone(), 'c').is_some()); // 'c' is inside
        assert!(split_range(r, 'd').is_none()); // 'd' is excluded
    }

    #[test]
    fn test_remove_last_char_inclusive()
    {
        let r = 'a' ..= 'c'; // 'a','b','c'
        let parts = split_range_inclusive(r, 'c').unwrap();
        let s = collect_range(parts.0) + &collect_range(parts.1);
        assert_eq!(s, "ab"); // 'abc' - 'c' = 'ab'
    }

    #[test]
    fn test_unicode_codepoint()
    {
        let alpha = '\u{03B1}'; // Greek small letter alpha
        let beta = '\u{03B2}'; // beta
        let gamma = '\u{03B3}'; // gamma

        let r = alpha ..= gamma; // α, β, γ
        let parts = split_range_inclusive(r, beta).unwrap();
        let s = collect_range(parts.0) + &collect_range(parts.1);
        assert_eq!(s, format!("{}{}", alpha, gamma)); // αβγ - β = αγ
    }

    #[test]
    fn test_not_in_range()
    {
        let r = 'a' .. 'c'; // 'a','b'
        assert!(split_range(r, 'z').is_none());
        assert!(split_range_inclusive('a' ..= 'c', 'z').is_none());
    }
}
