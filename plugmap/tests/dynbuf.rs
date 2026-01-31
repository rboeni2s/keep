use keep::Guard;
use plugmap::{ConcurrentBuffer, DynBuffer};
use std::{sync::atomic::AtomicUsize, thread};


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
        assert_eq!(Some(i), buf.put(i).ok());
    }

    assert_eq!(Some(3), buf.get(3).map(|g| *g));
    assert!(buf.put(10).is_err());
}


#[test]
fn dynbuf_st()
{
    let buffer = DynBuffer::new();
    let og = (0..256).collect::<Vec<_>>();

    for i in &og
    {
        buffer.push(*i);
    }

    let mut numbers = Vec::new();

    while let Some(num) = buffer.pop()
    {
        numbers.push(*num.read());
    }

    numbers.sort();
    assert_eq!(og, numbers);
}


#[ignore = "disable this test until the dynbuf deadlocks are fixed..."]
#[test]
fn dynbuf_mt()
{
    let buffer = Guard::new(DynBuffer::new());
    let original = (0..32).collect::<Vec<_>>();
    let finished = Guard::new(AtomicUsize::new(0));

    let buf = buffer.clone();
    let og = original.clone();
    let f = finished.clone();
    let a = thread::spawn(move || {
        for i in &og[..15]
        {
            buf.push(*i);
        }

        f.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    });

    let buf = buffer.clone();
    let og = original.clone();
    let f = finished.clone();
    let c = thread::spawn(move || {
        for i in &og[15..]
        {
            buf.push(*i);
        }

        f.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    });

    let buf = buffer.clone();
    let og = original.clone();
    let b = thread::spawn(move || {
        let mut numbers = Vec::new();
        while numbers.len() < 32
        {
            if let Some(num) = buf.pop()
            {
                numbers.push(*num.read());
            }
            else
            {
                if finished.load(std::sync::atomic::Ordering::Relaxed) <= 2
                {
                    break;
                }
            }
        }

        numbers.sort();
        assert_eq!(og, numbers);
    });

    let _ = a.join();
    let _ = b.join();
    let _ = c.join();
}
