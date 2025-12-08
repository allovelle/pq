pub use {const_sequential_iterator::*, replayable_iterator::*};

mod replayable_iterator
{
    /// A wrapper iterator that can replay the last yielded item.
    pub struct Replay<I>
    where
        I: Iterator,
        I::Item: Clone,
    {
        iter: I,
        last: Option<I::Item>,
        replay: bool,
    }

    impl<I> Replay<I>
    where
        I: Iterator,
        I::Item: Clone,
    {
        pub fn new(iter: I) -> Self
        {
            Replay { iter, last: None, replay: false }
        }

        /// Request that the next call to `.next()` repeats the last yielded item.
        pub fn replay(&mut self)
        {
            if !self.replay
            {
                self.replay = true;
            }
        }

        /// Optional convenience: view the previous value.
        pub fn peek_prev(&self) -> Option<&I::Item>
        {
            self.last.as_ref()
        }
    }

    impl<I> Iterator for Replay<I>
    where
        I: Iterator,
        I::Item: Clone,
    {
        type Item = I::Item;

        fn next(&mut self) -> Option<Self::Item>
        {
            if self.replay
            {
                self.replay = false;
                return self.last.clone();
            }

            let nxt = self.iter.next();
            if let Some(ref v) = nxt
            {
                self.last = Some(v.clone());
            }
            nxt
        }
    }

    /// Extension trait to attach `.replayable()` to all iterators.
    pub trait Replayable: Iterator + Sized
    where
        Self::Item: Clone,
    {
        fn replayable(self) -> Replay<Self>
        {
            Replay::new(self)
        }
    }

    impl<T> Replayable for T
    where
        T: Iterator,
        T::Item: Clone,
    {
    }
}

// TODO: Fix the const loops using manual indices
mod const_sequential_iterator
{
    pub struct SeqIter<'col, T>
    {
        at: usize,
        of: &'col [T],
    }

    impl<'col, T> SeqIter<'col, T>
    {
        pub const fn next(&mut self) -> Option<&'col T>
        {
            if self.at < self.of.len()
            {
                // let item: T = self.of[self.at];
                // self.at += 1;
                // return Some(&item);
                return None;
            }
            None
        }
    }
}
