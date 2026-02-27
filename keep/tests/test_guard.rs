mod helper;


use std::thread;

use crate::helper::Drops;
use helper::DropChecker;
use keep::{guard::Guard, heap::Heap};


#[test]
fn guard_drops()
{
    let drop_checker = DropChecker::new();
    let guard = Guard::new(drop_checker.drops());

    assert_eq!((), guard.can_access());
    assert_eq!(1, drop_checker.check());

    drop(guard);

    assert_eq!(0, drop_checker.check());
}


#[test]
fn guard_cloning()
{
    let drop_checker = DropChecker::new();
    let guard_a = Guard::new(drop_checker.drops());
    let guard_b = guard_a.clone();

    assert_eq!(1, drop_checker.check());
    drop(guard_a);
    assert_eq!(1, drop_checker.check());
    drop(guard_b);
    assert_eq!(0, drop_checker.check());
}


#[test]
fn guard_t_from_box_t()
{
    let drop_checker = DropChecker::new();
    let drops = Box::new(drop_checker.drops());
    let guard = Guard::<Drops>::new(drops);

    assert_eq!((), guard.can_access());
    assert_eq!(1, drop_checker.check());
    drop(guard);
    assert_eq!(0, drop_checker.check());
}


#[test]
fn guard_t_from_heap_t()
{
    let drop_checker = DropChecker::new();
    let drops = Heap::new(drop_checker.drops());
    let guard = Guard::<Drops>::new(drops);

    assert_eq!((), guard.can_access());
    assert_eq!(1, drop_checker.check());
    drop(guard);
    assert_eq!(0, drop_checker.check());
}


#[test]
fn mt_guard()
{
    let drop_checker = DropChecker::new();

    let guard_a = Guard::new(drop_checker.drops());
    let guard_b = guard_a.clone();

    let _bg_thread_scope = thread::scope(|scope| {
        scope.spawn(|| {
            assert_eq!((), guard_b.can_access());
            drop(guard_b);
        });

        scope.spawn(|| {
            assert_eq!((), guard_a.can_access());
            drop(guard_a);
        });
    });

    assert_eq!(0, drop_checker.check());
}
