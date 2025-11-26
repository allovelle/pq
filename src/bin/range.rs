use std::{
    marker::Sized,
    ops::{Add, Range, Sub},
};

fn split_range<Udx>(range: Range<Udx>, by: Udx) -> (Range<Udx>, Range<Udx>)
where
    Udx: Copy + PartialOrd + Sub<Output = Udx> + Inc,
{
    let left = range.start .. by;
    let right = by.inc() .. range.end;

    // TODO: Use RangeBound because it makes the .. vs ..= explicit
    if range.contains(&by)
    {
        // let left = range.start .. range.end - by;
        // let right = range.end - by .. range.end;
        let left = range.start .. by;
        let right = (by .. range.end);
        (left, right)
    }
    else
    {
        // Construct a zero value without further-restricting the generic type
        #[allow(clippy::eq_op)]
        let x: Udx = range.start - range.start;
        (range, x .. x)
    }
}

fn main()
{
    let (left, right) = split_range(0 .. 10, 11);
    println!("{:?} {:?}", left, right);

    let (left, right) = split_range(0 .. 10, 3);
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
