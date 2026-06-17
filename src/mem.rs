//! Virtual memory arena for efficient appending and random access of bytes.
//! It behaves like a manually managed virtual heap. It has:
//! True append-only buffer
//! No reallocs
//! 4KB page commitment
//! Exactly what you wanted
//! OS paging support
//! Inactive pages can be swapped out automatically
//! Random access by index
//! Pure pointer arithmetic
//! No Vec-style over-allocation
//! No exponential growth

use std::ops::Deref;
use std::ptr::NonNull;

const PAGE: usize = 4096;

pub struct VirtualArena
{
    ptr: NonNull<u8>,
    capacity: usize,
    len: usize,
}

impl VirtualArena
{
    /// Reserve virtual address space (no physical memory yet)
    pub fn new(capacity: usize) -> Self
    {
        assert!(capacity > 0);

        unsafe {
            #[cfg(target_os = "windows")]
            let ptr = {
                use windows_sys::Win32::System::Memory::{
                    MEM_RESERVE, PAGE_READWRITE, VirtualAlloc,
                };

                let p = VirtualAlloc(
                    std::ptr::null_mut(),
                    capacity,
                    MEM_RESERVE,
                    PAGE_READWRITE,
                );

                if p.is_null()
                {
                    panic!("VirtualAlloc reserve failed");
                }

                p as *mut u8
            };

            #[cfg(unix)]
            let ptr = {
                use libc::{MAP_ANON, MAP_PRIVATE, PROT_NONE, mmap};

                let p = mmap(
                    std::ptr::null_mut(),
                    capacity,
                    PROT_NONE,
                    MAP_PRIVATE | MAP_ANON,
                    -1,
                    0,
                );

                if p == libc::MAP_FAILED
                {
                    panic!("mmap reserve failed");
                }

                p as *mut u8
            };

            Self { ptr: NonNull::new(ptr).expect("null ptr"), capacity, len: 0 }
        }
    }

    /// Append bytes, committing pages as needed
    pub fn append(&mut self, bytes: &[u8])
    {
        let new_len = self.len + bytes.len();
        assert!(new_len <= self.capacity, "VirtualArena overflow");

        self.commit_if_needed(new_len);

        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                self.ptr.as_ptr().add(self.len),
                bytes.len(),
            );
        }

        self.len = new_len;
    }

    /// Single byte random access
    pub fn get(&self, i: usize) -> u8
    {
        assert!(i < self.len);
        unsafe { *self.ptr.as_ptr().add(i) }
    }

    pub fn len(&self) -> usize { self.len }

    /// Full slice view of committed data
    pub fn as_slice(&self) -> &[u8]
    {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    /// Commit additional pages if needed
    fn commit_if_needed(&mut self, new_len: usize)
    {
        let old_pages = (self.len + PAGE - 1) / PAGE;
        let new_pages = (new_len + PAGE - 1) / PAGE;

        if new_pages <= old_pages
        {
            return;
        }

        let start = old_pages * PAGE;
        let size = (new_pages - old_pages) * PAGE;

        unsafe {
            #[cfg(target_os = "windows")]
            {
                use windows_sys::Win32::System::Memory::{
                    MEM_COMMIT, PAGE_READWRITE, VirtualAlloc,
                };

                let p = VirtualAlloc(
                    self.ptr.as_ptr().add(start) as _,
                    size,
                    MEM_COMMIT,
                    PAGE_READWRITE,
                );

                if p.is_null()
                {
                    panic!("VirtualAlloc commit failed");
                }
            }

            #[cfg(unix)]
            {
                use libc::{PROT_READ, PROT_WRITE, mprotect};

                let res = mprotect(
                    self.ptr.as_ptr().add(start) as _,
                    size,
                    PROT_READ | PROT_WRITE,
                );

                if res != 0
                {
                    panic!("mprotect commit failed");
                }
            }
        }
    }
}

impl Drop for VirtualArena
{
    fn drop(&mut self)
    {
        unsafe {
            #[cfg(target_os = "windows")]
            {
                use windows_sys::Win32::System::Memory::MEM_RELEASE;
                use windows_sys::Win32::System::Memory::VirtualFree;

                VirtualFree(self.ptr.as_ptr() as _, 0, MEM_RELEASE);
            }

            #[cfg(unix)]
            {
                libc::munmap(self.ptr.as_ptr() as _, self.capacity);
            }
        }
    }
}
