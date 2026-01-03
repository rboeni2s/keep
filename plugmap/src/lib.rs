#![allow(unused)]


mod dynbuf;
mod entry;
mod map;
mod resizer;
mod table;


pub use dynbuf::{ConcurrentBuffer, DynBuffer};
pub use map::PlugMap;


#[cfg(test)]
mod tests
{
    use std::thread;

    use super::*;

    #[test]
    fn look_and_feel()
    {
        let map = PlugMap::<usize, &str>::new();

        assert!(
            map.get(&39).is_none(),
            "Empty map did not return None on get(..)"
        );

        map.insert(39, "Briar");

        assert_eq!(
            Some("Briar"),
            map.get(&39).map(|v| *v),
            "get(..) did not return the current value"
        );

        assert_eq!("Briar", *map.insert(39, "Miku").unwrap().read());

        assert_eq!(
            Some("Miku"),
            map.get(&39).map(|v| *v),
            "get(..) did not return the current value"
        );
    }

    // #[test]
    // fn remove()
    // {
    //     let map = PlugMap::<u32, &str>::new();

    //     assert!(map.remove(&39).is_none());
    //     map.insert(39, "Briar");
    //     assert_eq!(Some("Briar"), map.remove(&39).map(|g| *g.read()));
    //     assert!(map.remove(&39).is_none());
    //     assert!(map.insert(39, "Other").is_none());
    //     assert_eq!(Some("Other"), map.remove(&39).map(|g| *g.read()));
    //     assert!(map.remove(&39).is_none());
    // }

    #[test]
    fn many_entries()
    {
        let map = PlugMap::new();

        for i in 0..100
        {
            map.insert(i, i.to_string());
        }

        assert_eq!(Some("39"), map.get(&39).as_ref().map(|g| g.as_str()));
        // assert_eq!(
        //     Some("39"),
        //     map.remove(&39).map(|k| k.read().to_string()).as_deref()
        // );
        // assert!(map.remove(&39).is_none());
        // assert_eq!(None, map.get(&39));
        assert_eq!(Some("31"), map.get(&31).as_ref().map(|g| g.as_str()));
    }

    #[test]
    fn many_threads()
    {
        let map = PlugMap::new();
        let mut threads = vec![];

        for t in 0..10
        {
            let map = map.clone();
            threads.push(thread::spawn(move || {
                for i in 0..10
                {
                    map.insert(i, i.to_string());
                }
            }));
        }

        for t in threads
        {
            t.join();
        }
    }
}
