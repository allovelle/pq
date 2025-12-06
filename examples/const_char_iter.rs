use pq::txt::*;

fn main()
{
    let chars = "Hello World!";
    let mut iter = utf8_iter_chars_const(chars);
    while let Some(ch) = iter.next()
    {
        print!("`{ch}`");
    }
    println!();
}
