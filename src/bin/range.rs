use std::marker::Sized;
use std::ops::{Range, RangeInclusive};

/// Removes the element from the range by creating 2 ranges that exclude that
/// element. If the elemnt is not in the range to begin with, the ranges overlap
/// with each other by having the first range end on the second range start.
/// Returns `None` if the element isn't in the range to begin with.
fn split_range<I>(range: Range<I>, by: I) -> Option<(Range<I>, Range<I>)>
where
    I: Copy + PartialOrd + Inc,
{
    match range.contains(&by)
    {
        true => Some((range.start .. by, by.inc() .. range.end)),
        false => None,
    }
}

fn split_range_inclusive<I>(
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

fn main()
{
    let (left, right) = split_range(0 .. 10, 8).unwrap();
    println!("{:?} {:?}", left, right);

    let (left, right) = split_range(0 .. 10, 3).unwrap();
    println!("{:?} {:?}", left, right);

    let (left, right) = split_range_inclusive(0 ..= 10, 8).unwrap();
    println!("{:?} {:?}", left, right);

    let (left, right) = split_range_inclusive(0 ..= 10, 3).unwrap();
    println!("{:?} {:?}", left, right);
}

pub trait Inc
where
    Self: Sized,
{
    fn inc(self) -> Self;
    fn inc_strict(self) -> Self;
    fn inc_checked(self) -> Option<Self>;
    fn inc_wrapping(self) -> Self;
    fn inc_carrying(self, carry: bool) -> (Self, bool);
    /// # Safety
    /// See stdlib number types' unchecked_add() implementation.
    unsafe fn inc_unchecked(self) -> Self;
    fn inc_saturating(self) -> Self;
    fn inc_overflowing(self) -> (Self, bool);
}

impl Inc for usize
{
    fn inc(self) -> Self
    {
        self + 1
    }

    fn inc_strict(self) -> Self
    {
        self.strict_add(1)
    }

    fn inc_checked(self) -> Option<Self>
    {
        self.checked_add(1)
    }

    fn inc_wrapping(self) -> Self
    {
        self.wrapping_add(1)
    }

    fn inc_carrying(self, carry: bool) -> (Self, bool)
    {
        self.carrying_add(1, carry)
    }

    unsafe fn inc_unchecked(self) -> Self
    {
        unsafe { self.unchecked_add(1) }
    }

    fn inc_saturating(self) -> Self
    {
        self.saturating_add(1)
    }

    fn inc_overflowing(self) -> (Self, bool)
    {
        self.overflowing_add(1)
    }
}
