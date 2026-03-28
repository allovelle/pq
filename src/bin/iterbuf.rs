// ! This buffers over <T> but it needs to buffer over u8, index by usize over
// ! <T>, iterate over <T>
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

    while let Some(i) = iter.next()
    {
        println!("{i:?}");
    }

    println!("{:?}", iter.get(0));
}
