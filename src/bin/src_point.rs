#![allow(dead_code)]

use std::fmt;

fn main()
{
    let sent = Sentinel::new();
}

/// Number of bits required to represent `n`.
/// Returns 0 for n == 0, otherwise returns 1..=usize::BITS.
fn bits_required(n: usize) -> u32
{
    if n == 0
    {
        0
    }
    else
    {
        // ilog2 panics on 0, so we guarded above.
        n.ilog2() + 1
    }
}

/// Errors that can occur when packing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackError
{
    /// The `line` value does not fit in the current right-side bit budget.
    LineOverflow
    {
        line: usize, allowed_bits: u32
    },

    /// The `ch` (char/column) value does not fit in the left-side bit budget.
    CharOverflow
    {
        ch: usize, allowed_bits: u32
    },
}

impl fmt::Display for PackError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        match self
        {
            PackError::LineOverflow { line, allowed_bits } =>
            {
                write!(f, "line {} does not fit in {} bits", line, allowed_bits)
            }
            PackError::CharOverflow { ch, allowed_bits } =>
            {
                write!(f, "char {} does not fit in {} bits", ch, allowed_bits)
            }
        }
    }
}

/// Sentinel tracks the increment-only counters and exposes the current splitter.
/// By default the splitter is derived from `max_line` (the external increment-only counter).
#[derive(Debug, Default, Clone, Copy)]
pub struct Sentinel
{
    /// The external increment-only line counter (max observed or current line).
    max_line: usize,

    /// Optional increment-only char counter (not used for splitting by default,
    /// but available if you want to split by char instead).
    max_char: usize,
}

impl Sentinel
{
    /// Create a new sentinel with both counters starting at 0.
    pub fn new() -> Self
    {
        Self { max_line: 0, max_char: 0 }
    }

    /// Create a sentinel with an initial line value.
    pub fn with_line(line: usize) -> Self
    {
        Self { max_line: line, max_char: 0 }
    }

    /// Increment the line counter (returns the new line).
    /// This is the operation you call while parsing when you advance to the next logical line.
    pub fn inc_line(&mut self) -> usize
    {
        self.max_line = self.max_line.wrapping_add(1);
        self.max_line
    }

    /// Set the line counter explicitly (useful for initialization).
    pub fn set_line(&mut self, line: usize)
    {
        self.max_line = line;
    }

    /// Read-only access to the current max_line.
    pub fn max_line(&self) -> usize
    {
        self.max_line
    }

    /// Increment the char counter (optional; not used for splitting by default).
    pub fn inc_char(&mut self) -> usize
    {
        self.max_char = self.max_char.wrapping_add(1);
        self.max_char
    }

    /// Set the char counter explicitly.
    pub fn set_char(&mut self, ch: usize)
    {
        self.max_char = ch;
    }

    /// Read-only access to the current max_char.
    pub fn max_char(&self) -> usize
    {
        self.max_char
    }

    /// Number of low bits reserved for the line counter.
    /// This is computed from `max_line` (bits_required(max_line)).
    pub fn right_bits_for_line(&self) -> u32
    {
        bits_required(self.max_line)
    }

    /// Number of low bits reserved for the char counter (if you wanted to split by char).
    pub fn right_bits_for_char(&self) -> u32
    {
        bits_required(self.max_char)
    }

    /// Number of bits available for the left-side value given `right_bits`.
    fn left_bits_from_right_bits(right_bits: u32) -> u32
    {
        usize::BITS - right_bits
    }

    /// Convenience: number of left bits available when splitting by line.
    pub fn left_bits_for_line(&self) -> u32
    {
        Self::left_bits_from_right_bits(self.right_bits_for_line())
    }

    /// Build the mask for the low `right_bits` bits.
    /// Handles the special case `right_bits == usize::BITS`.
    fn low_mask(right_bits: u32) -> usize
    {
        if right_bits == 0
        {
            0usize
        }
        else if right_bits >= usize::BITS
        {
            usize::MAX
        }
        else
        {
            // safe: right_bits < usize::BITS
            (1usize << right_bits) - 1
        }
    }
}

/// A packed source point: low bits = line, high bits = char (or column).
/// Use `pack`/`unpack` with a `&Sentinel` so the same packed value can be interpreted
/// correctly as the sentinel's splitter evolves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePoint
{
    packed: usize,
}

impl SourcePoint
{
    // ! Invariant: SourcePoint::new() will not be called with a value that has
    // ! a bit-width larger than the provided Sentinel. As such, the Sentinel
    // ! should probably call a crate private constructor to make it impossible
    // ! for sentinel-less SrcPoints to be instantiated.

    /// Pack `line` (low bits) and `ch` (high bits) using the sentinel's current splitter.
    ///
    /// Returns `Err(PackError)` if either value does not fit in the allocated bits.
    pub fn pack_by_line(
        line: usize,
        ch: usize,
        s: &Sentinel,
    ) -> Result<Self, PackError>
    {
        let right_bits = s.right_bits_for_line();
        let left_bits = Sentinel::left_bits_from_right_bits(right_bits);

        // Check fits for line
        if right_bits < usize::BITS
        {
            let max_line =
                if right_bits == 0 { 0 } else { (1usize << right_bits) - 1 };
            if line > max_line
            {
                return Err(PackError::LineOverflow {
                    line,
                    allowed_bits: right_bits,
                });
            }
        }
        else
        {
            // right_bits == usize::BITS -> line can be any usize (fits)
        }

        // Check fits for ch (left side)
        if left_bits == 0
        {
            if ch != 0
            {
                return Err(PackError::CharOverflow {
                    ch,
                    allowed_bits: left_bits,
                });
            }
        }
        else if left_bits < usize::BITS
        {
            let max_ch = (1usize << left_bits) - 1;
            if ch > max_ch
            {
                return Err(PackError::CharOverflow {
                    ch,
                    allowed_bits: left_bits,
                });
            }
        } // else left_bits == usize::BITS -> ch fits

        // Compose packed: (ch << right_bits) | line
        let packed = if right_bits == 0
        {
            ch
        }
        else
        {
            (ch << right_bits) | (line & Sentinel::low_mask(right_bits))
        };

        Ok(SourcePoint { packed })
    }

    /// Unpack the line (low bits) using the sentinel's current splitter.
    pub fn line(&self, s: &Sentinel) -> usize
    {
        let rb = s.right_bits_for_line();
        if rb == 0
        {
            0
        }
        else if rb >= usize::BITS
        {
            self.packed
        }
        else
        {
            self.packed & Sentinel::low_mask(rb)
        }
    }

    /// Unpack the char (high bits) using the sentinel's current splitter.
    pub fn ch(&self, s: &Sentinel) -> usize
    {
        let rb = s.right_bits_for_line();
        if rb >= usize::BITS { 0 } else { self.packed >> rb }
    }

    /// Access the raw packed usize (for storage).
    pub fn raw(&self) -> usize
    {
        self.packed
    }

    /// Reconstruct a SourcePoint from a raw packed usize.
    pub fn from_raw(raw: usize) -> Self
    {
        SourcePoint { packed: raw }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn bits_required_examples()
    {
        assert_eq!(bits_required(0), 0);
        assert_eq!(bits_required(1), 1);
        assert_eq!(bits_required(2), 2);
        assert_eq!(bits_required(3), 2);
        assert_eq!(bits_required(255), 8);
        assert_eq!(bits_required(256), 9);
    }

    #[test]
    fn pack_unpack_basic()
    {
        let mut s = Sentinel::new();
        // start with line 0 -> right_bits = 0 -> all bits for char
        assert_eq!(s.right_bits_for_line(), 0);
        let sp = SourcePoint::pack_by_line(0, 0x1234_5678, &s).unwrap();
        assert_eq!(sp.line(&s), 0);
        assert_eq!(sp.ch(&s), 0x1234_5678);

        // increment line to 1 -> right_bits = 1
        s.inc_line(); // max_line = 1
        assert_eq!(s.right_bits_for_line(), 1);

        // pack a small line and small char
        let sp2 = SourcePoint::pack_by_line(1, 3, &s).unwrap();
        assert_eq!(sp2.line(&s), 1);
        assert_eq!(sp2.ch(&s), 3);

        // if line grows, right_bits increases and left capacity shrinks
        s.set_line(1023); // needs 10 bits
        assert_eq!(s.right_bits_for_line(), 10);
        let left_bits = s.left_bits_for_line();
        assert!(left_bits <= usize::BITS);

        // char must fit in left_bits
        let max_ch = if left_bits == 0
        {
            0
        }
        else if left_bits >= usize::BITS
        {
            usize::MAX
        }
        else
        {
            (1usize << left_bits) - 1
        };

        // packing a char that fits should succeed
        let sp3 = SourcePoint::pack_by_line(512, max_ch, &s).unwrap();
        assert_eq!(sp3.line(&s), 512);
        assert_eq!(sp3.ch(&s), max_ch);

        // packing a char that doesn't fit should error
        if left_bits > 0 && left_bits < usize::BITS
        {
            let too_big = max_ch.wrapping_add(1);
            assert!(SourcePoint::pack_by_line(1, too_big, &s).is_err());
        }
    }
}
