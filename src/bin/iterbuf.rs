// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary
// TODO: This is only for STDIN --> FILES use mmap so no memcp is necessary

// TODO: THIS NEEDS TO IMPLEMENT A PARTIAL UTF-8 CODEPOINT BUFFER SO THAT ANY
// TODO: AMOUNT OF BYTES CAN BE APPENDED WITHOUT INVALIDATING REFERENCES.
// TODO: CAN USE THE UTF-8 ITER TYPE FROM THE TINY BRANCH.

// ! This buffers over <T> but it needs to buffer over u8, index by usize over
// ! <T>, iterate over <T>

/// Allows concurrent appending, iteration, and random access over a buffer by
/// taking advantage of the fact that all indices point to previous ranges due
/// to the underlying append-only buffer.
/// 1. Allows iteration
/// 2. Append-only modification
/// 3. Random access indexing
struct BufIter<T>
{
    buffer: Vec<T>,
    counter: usize,
}

impl<T: Copy> BufIter<T>
{
    fn new(buffer: Vec<T>) -> Self { BufIter { buffer, counter: 0 } }

    fn push(&mut self, item: T) { self.buffer.push(item); }

    fn iter(mut self) -> Self
    {
        self.counter = 0;
        self
    }

    fn iter_from(mut self, index: usize) -> Self
    {
        self.counter = index;
        self
    }

    fn next(&mut self) -> Option<T>
    {
        let item = self.buffer.get(self.counter).copied();
        self.counter = self.counter.saturating_add(1);
        item
    }

    fn get(&self, index: usize) -> Option<T> { self.buffer.get(index).copied() }
}

fn main()
{
    let mut iter = BufIter::new(vec![818, 616, 118, 8]).iter();
    let mut range = 0 .. 10;

    while let Some(i) = iter.next()
    {
        // Concurrent modification
        if let Some(n) = range.next()
        {
            iter.push(i + n);
            println!("{:?} push", i + n);
        }
        else
        {
            println!("{i:?} iter");
        }
    }

    println!("{:?} rand", iter.get(0));
}
