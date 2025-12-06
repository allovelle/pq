use std::fmt;
use std::ops::RangeInclusive;

/// Inclusive char range type
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CharRangeInclusive
{
    pub from: char,
    pub onto: char,
}

impl CharRangeInclusive
{
    /// Create from a standard RangeInclusive<char>
    pub fn from_range(r: RangeInclusive<char>) -> Self
    {
        Self { from: *r.start(), onto: *r.end() }
    }

    /// Convert back to RangeInclusive<char>
    pub fn into_range(self) -> RangeInclusive<char>
    {
        self.from ..= self.onto
    }

    /// Empty sentinel (no range)
    pub fn empty() -> Option<Self>
    {
        None
    }

    /// Check membership
    pub fn contains(&self, ch: char) -> bool
    {
        self.from <= ch && ch <= self.onto
    }

    /// Split the range by removing `at`.
    ///
    /// Returns `(left, right)` where each is `Some(CharRangeInclusive)` if non-empty,
    /// or `None` if that side is empty.
    pub fn split(self, at: char) -> (Option<Self>, Option<Self>)
    {
        // If `at` is outside the range, nothing is removed: return the original as left, right empty.
        if at < self.from || at > self.onto
        {
            return (Some(self), None);
        }

        // Convert to u32 for safe arithmetic
        let from_u = self.from as u32;
        let onto_u = self.onto as u32;
        let at_u = at as u32;

        // If the range is exactly the single char `at`, both sides are empty
        if from_u == onto_u && at_u == from_u
        {
            return (None, None);
        }

        // Left side: from ..= at-1 (if at > from)
        let left = if at_u > from_u
        {
            // safe to unwrap because at_u > from_u implies at_u - 1 >= from_u and is valid scalar
            Some(CharRangeInclusive {
                from: std::char::from_u32(from_u).unwrap(),
                onto: std::char::from_u32(at_u - 1).unwrap(),
            })
        }
        else
        {
            None
        };

        // Right side: at+1 ..= onto (if at < onto)
        let right = if at_u < onto_u
        {
            Some(CharRangeInclusive {
                from: std::char::from_u32(at_u + 1).unwrap(),
                onto: std::char::from_u32(onto_u).unwrap(),
            })
        }
        else
        {
            None
        };

        (left, right)
    }
}

impl fmt::Display for CharRangeInclusive
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        write!(f, "'{}' .. '{}'", self.from, self.onto)
    }
}

fn main()
{
    // Examples
    let r = CharRangeInclusive::from_range('a' ..= 'z');

    // remove 'm' -> left: 'a'..'l', right: 'n'..'z'
    let (l, rgt) = r.split('m');
    println!("original: {}", CharRangeInclusive::from_range('a' ..= 'z'));
    println!("remove 'm' -> left: {:?}, right: {:?}", l, rgt);

    // remove outside -> original returned as left, right None
    let (l2, r2) = r.split('Ω');
    println!("remove 'Ω' -> left: {:?}, right: {:?}", l2, r2);

    // remove left boundary
    let (l3, r3) = r.split('a');
    println!("remove 'a' -> left: {:?}, right: {:?}", l3, r3);

    // remove single-element range
    let single = CharRangeInclusive::from_range('x' ..= 'x');
    let (ls, rs) = single.split('x');
    println!("single 'x' split -> left: {:?}, right: {:?}", ls, rs);
}
