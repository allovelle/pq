use std::char;
use std::marker::Sized;

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

impl Inc for char
{
    fn inc(self) -> Self
    {
        // panics if next char doesn't exist
        char::from_u32(self as u32 + 1).expect("invalid char increment")
    }

    fn inc_strict(self) -> Self
    {
        // same as inc(), but explicit about panicking
        char::from_u32(self as u32 + 1).unwrap()
    }

    fn inc_checked(self) -> Option<Self>
    {
        char::from_u32(self as u32 + 1)
    }

    fn inc_wrapping(self) -> Self
    {
        // wrap around at u32::MAX back to '\u{0}'
        char::from_u32((self as u32).wrapping_add(1) % (char::MAX as u32 + 1))
            .unwrap_or('\u{0}')
    }

    fn inc_carrying(self, carry: bool) -> (Self, bool)
    {
        let val = self as u32;
        let (next, overflow) =
            val.overflowing_add(1 + if carry { 1 } else { 0 });
        match char::from_u32(next)
        {
            Some(c) => (c, overflow),
            None => ('\u{0}', true),
        }
    }

    unsafe fn inc_unchecked(self) -> Self
    {
        // unsafe: assumes next is valid Unicode scalar
        unsafe { char::from_u32_unchecked(self as u32 + 1) }
    }

    fn inc_saturating(self) -> Self
    {
        match self
        {
            char::MAX => self,
            _ => char::from_u32(self as u32 + 1).unwrap(),
        }
    }

    fn inc_overflowing(self) -> (Self, bool)
    {
        let (next, overflow) = (self as u32).overflowing_add(1);
        match char::from_u32(next)
        {
            Some(c) => (c, overflow),
            None => ('\u{0}', true),
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::Inc;

    #[test]
    fn test_char_inc_basic()
    {
        assert_eq!('a'.inc(), 'b');
        assert_eq!('a'.inc_checked(), Some('b'));
        assert_eq!('a'.inc_saturating(), 'b');
    }

    #[test]
    fn test_char_inc_edge()
    {
        // max char
        assert_eq!(char::MAX.inc_saturating(), char::MAX);
        assert_eq!(char::MAX.inc_checked(), None);

        // wrapping goes back to '\u{0}'
        assert_eq!(char::MAX.inc_wrapping(), '\u{0}');
    }

    #[test]
    fn test_char_inc_overflowing()
    {
        let (c, overflow) = 'a'.inc_overflowing();
        assert_eq!(c, 'b');
        assert!(!overflow);

        let (c, overflow) = char::MAX.inc_overflowing();
        assert_eq!(c, '\u{0}');
        assert!(overflow);
    }

    // * Usize Increment Tests

    #[test]
    fn test_usize_inc_basic()
    {
        assert_eq!(5usize.inc(), 6);
        assert_eq!(5usize.inc_checked(), Some(6));
        assert_eq!(5usize.inc_saturating(), 6);
    }

    #[test]
    fn test_usize_inc_edge()
    {
        let max = usize::MAX;

        // checked returns None at max
        assert_eq!(max.inc_checked(), None);

        // saturating stays at max
        assert_eq!(max.inc_saturating(), max);

        // wrapping goes back to 0
        assert_eq!(max.inc_wrapping(), 0);
    }

    #[test]
    fn test_usize_inc_overflowing()
    {
        let (val, overflow) = 5usize.inc_overflowing();
        assert_eq!(val, 6);
        assert!(!overflow);

        let (val, overflow) = usize::MAX.inc_overflowing();
        assert_eq!(val, 0);
        assert!(overflow);
    }

    #[test]
    fn test_usize_inc_carrying()
    {
        let (val, carry) = 5usize.inc_carrying(false);
        assert_eq!(val, 6);
        assert!(!carry);

        let (val, carry) = usize::MAX.inc_carrying(false);
        assert_eq!(val, 0);
        assert!(carry);
    }

    #[test]
    fn test_usize_inc_unchecked()
    {
        unsafe {
            assert_eq!(5usize.inc_unchecked(), 6);
        }
    }
}
