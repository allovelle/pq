use pq::range::{split_range, split_range_inclusive};

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
