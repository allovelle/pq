use pq::ret_if;

#[test]
fn test_entire_pipeline()
{
    assert_ne!('a', 'b');

    let fn0 = || {
        ret_if!(false, 123);
        818
    };

    assert_eq!(fn0(), 818);
}

#[test]
fn test_entire_pipeline2()
{
    assert_ne!('a', 'b');

    let fn0 = || {
        ret_if!(false, 123);
        818
    };

    assert_eq!(fn0(), 818);
}
