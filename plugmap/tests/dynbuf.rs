use plugmap::ConcurrentBuffer;


#[test]
fn concurrent_buffer_insert()
{
    let buf = ConcurrentBuffer::with_capacity(10);

    assert!(buf.insert(0, 39).is_none());
    assert!(buf.insert(1, 390).is_none());
    assert_eq!(Some(39), buf.insert(0, 2).map(|k| *k.read()));
    assert_eq!(Some(2), buf.insert(0, 10).map(|k| *k.read()));
}


#[test]
fn concurrent_buffer_get()
{
    let buf = ConcurrentBuffer::with_capacity(10);

    assert!(buf.get(0).is_none());
    buf.insert(0, 39);
    assert!(buf.get(1).is_none());
    assert_eq!(Some(39), buf.get(0).map(|g| *g));
}


#[test]
fn concurrent_buffer_put()
{
    let buf = ConcurrentBuffer::with_capacity(5);

    for i in 0..5
    {
        assert_eq!(Ok(i), buf.put(i));
    }

    assert_eq!(Some(3), buf.get(3).map(|g| *g));
    assert!(buf.put(10).is_err());
}
