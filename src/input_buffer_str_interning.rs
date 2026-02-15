//! An append-only UTF-8 text buffer that uses a single large heap allocation.
//! Allows handing out string slice references that remain valid even as new
//! data is appended, up to the buffer's capacity.

use std::alloc::{Layout, alloc, dealloc};
use std::cell::RefCell;
use std::ptr::NonNull;

/// An append-only UTF-8 text buffer that uses a single large heap allocation.
/// Allows handing out string slice references that remain valid even as new
/// data is appended, up to the buffer's capacity.
///
/// This works by allocating a single large buffer upfront and never reallocating,
/// ensuring that all pointers remain valid for the lifetime of the buffer.
pub struct Utf8Buffer
{
    // The raw heap allocation
    buffer: NonNull<u8>,
    // Total capacity of the buffer
    capacity: usize,
    // Current length of valid data
    len: RefCell<usize>,
}

impl Utf8Buffer
{
    /// Creates a new UTF-8 buffer with the specified capacity.
    ///
    /// # Panics
    /// Panics if capacity is 0 or if allocation fails.
    pub fn new(capacity: usize) -> Self
    {
        assert!(capacity > 0, "Capacity must be greater than 0");

        let layout = Layout::array::<u8>(capacity).expect("Invalid layout");
        let buffer = unsafe {
            let ptr = alloc(layout);
            if ptr.is_null()
            {
                panic!("Allocation failed");
            }
            NonNull::new_unchecked(ptr)
        };

        Self { buffer, capacity, len: RefCell::new(0) }
    }

    /// Appends UTF-8 bytes to the buffer and returns a reference to them.
    ///
    /// # Panics
    /// Panics if:
    /// - The bytes are not valid UTF-8
    /// - There is insufficient capacity remaining
    ///
    /// # Safety
    /// The returned reference has a 'static lifetime, which is technically incorrect.
    /// However, it's safe as long as:
    /// 1. The buffer outlives all uses of returned references
    /// 2. The buffer is never dropped while references exist
    pub fn append(&self, bytes: &[u8]) -> &'static str
    {
        std::str::from_utf8(bytes).expect("Invalid UTF-8");

        if bytes.is_empty()
        {
            return "";
        }

        let mut len = self.len.borrow_mut();
        let new_len = *len + bytes.len();

        if new_len > self.capacity
        {
            panic!(
                "Insufficient capacity: need {} bytes, but only {} available",
                bytes.len(),
                self.capacity - *len
            );
        }

        // SAFETY: We've verified there's enough space and we have exclusive
        // access via RefCell
        unsafe {
            let dst = self.buffer.as_ptr().add(*len);
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
        }

        let start_offset = *len;
        *len = new_len;

        // SAFETY:
        // 1. The buffer never moves (no reallocation)
        // 2. We validated UTF-8
        // 3. The lifetime is extended to 'static, which is a lie but safe
        //    if the buffer outlives all references
        unsafe {
            let slice_ptr = self.buffer.as_ptr().add(start_offset);
            let slice = std::slice::from_raw_parts(slice_ptr, bytes.len());
            std::str::from_utf8_unchecked(slice)
        }
    }

    /// Appends a string to the buffer and returns a reference to it.
    pub fn append_str(&self, s: &str) -> &'static str
    {
        self.append(s.as_bytes())
    }

    /// Returns a reference to all data currently in the buffer as a string slice.
    pub fn as_str(&self) -> &str
    {
        let len = *self.len.borrow();

        if len == 0
        {
            return "";
        }

        // SAFETY:
        // - We've validated UTF-8 on all appends
        // - len is within bounds of our allocation
        unsafe {
            let slice = std::slice::from_raw_parts(self.buffer.as_ptr(), len);
            std::str::from_utf8_unchecked(slice)
        }
    }

    /// Returns the number of bytes currently stored in the buffer.
    pub fn len(&self) -> usize
    {
        *self.len.borrow()
    }

    /// Returns true if the buffer contains no data.
    pub fn is_empty(&self) -> bool
    {
        self.len() == 0
    }

    /// Returns the total capacity of the buffer in bytes.
    pub fn capacity(&self) -> usize
    {
        self.capacity
    }

    /// Returns the number of bytes available for future appends.
    pub fn remaining_capacity(&self) -> usize
    {
        self.capacity - self.len()
    }

    /// Clears all data from the buffer, but keeps the allocation.
    /// Previously handed out references become invalid after this call.
    pub fn clear(&self)
    {
        *self.len.borrow_mut() = 0;
    }
}

impl Drop for Utf8Buffer
{
    fn drop(&mut self)
    {
        let layout =
            Layout::array::<u8>(self.capacity).expect("Invalid layout");
        unsafe {
            dealloc(self.buffer.as_ptr(), layout);
        }
    }
}

// SAFETY: The buffer can be sent between threads because:
// - The raw pointer is just data
// - RefCell provides interior mutability protection
// - No thread-local state
unsafe impl Send for Utf8Buffer {}

// Note: Not Sync because RefCell is not Sync
// If you need Sync, replace RefCell with Mutex or AtomicUsize

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_basic_append()
    {
        let buffer = Utf8Buffer::new(1024);
        buffer.append_str("Hello");
        buffer.append_str(" ");
        buffer.append_str("World");

        assert_eq!(buffer.as_str(), "Hello World");
        assert_eq!(buffer.len(), 11);
    }

    #[test]
    fn test_stable_references()
    {
        let buffer = Utf8Buffer::new(1024);
        let ref1 = buffer.append_str("First");
        let ref2 = buffer.append_str("Second");
        let ref3 = buffer.append_str("Third");

        // All references remain valid even after more appends
        buffer.append_str("Fourth");

        assert_eq!(ref1, "First");
        assert_eq!(ref2, "Second");
        assert_eq!(ref3, "Third");

        // Full buffer should contain all data
        assert_eq!(buffer.as_str(), "FirstSecondThirdFourth");
    }

    #[test]
    fn test_empty_append()
    {
        let buffer = Utf8Buffer::new(1024);
        let empty_ref = buffer.append_str("");
        assert_eq!(empty_ref, "");
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    #[should_panic(expected = "Invalid UTF-8")]
    fn test_invalid_utf8()
    {
        let buffer = Utf8Buffer::new(1024);
        buffer.append(&[0xFF, 0xFE, 0xFD]);
    }

    #[test]
    #[should_panic(expected = "Insufficient capacity")]
    fn test_capacity_exceeded()
    {
        let buffer = Utf8Buffer::new(10);
        buffer.append_str("This is way too long for the buffer");
    }

    #[test]
    fn test_capacity_tracking()
    {
        let buffer = Utf8Buffer::new(100);
        assert_eq!(buffer.capacity(), 100);
        assert_eq!(buffer.remaining_capacity(), 100);

        buffer.append_str("Hello");
        assert_eq!(buffer.remaining_capacity(), 95);
        assert_eq!(buffer.len(), 5);
    }

    #[test]
    fn test_clear()
    {
        let buffer = Utf8Buffer::new(1024);
        buffer.append_str("Some data");
        assert_eq!(buffer.len(), 9);

        buffer.clear();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        assert_eq!(buffer.capacity(), 1024);
    }

    #[test]
    fn test_as_str_changes()
    {
        let buffer = Utf8Buffer::new(1024);
        assert_eq!(buffer.as_str(), "");

        buffer.append_str("Hello");
        assert_eq!(buffer.as_str(), "Hello");

        buffer.append_str(" World");
        assert_eq!(buffer.as_str(), "Hello World");
    }
}
