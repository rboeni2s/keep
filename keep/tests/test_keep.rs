mod helper;


use helper::DropChecker;
use keep2::prelude::*;


#[test]
fn do_not_drop_current_value()
{
    let drop_checker = DropChecker::new();
    let keep = Keep::new(drop_checker.drops());
    let guard = keep.read();

    assert_eq!((), guard.can_access());
    assert_eq!(1, drop_checker.check());
    drop(guard);
    assert_eq!(1, drop_checker.check());
    drop(keep);
    assert_eq!(0, drop_checker.check());
}


#[test]
fn guard_survives_keep()
{
    let drop_checker = DropChecker::new();
    let keep = Keep::new(drop_checker.drops());
    let guard = keep.read();

    assert_eq!((), guard.can_access());
    assert_eq!(1, drop_checker.check());
    drop(keep);
    assert_eq!(1, drop_checker.check());
    assert_eq!((), guard.can_access());
    drop(guard);
    assert_eq!(0, drop_checker.check());
}


#[test]
fn unguarded_write_drops()
{
    let drop_checker = DropChecker::new();
    let dummy = DropChecker::new();
    let keep = Keep::new(drop_checker.drops());

    assert_eq!(1, drop_checker.check());
    keep.write(dummy.drops());
    assert_eq!(0, drop_checker.check());
}


#[test]
fn guarded_write_does_not_drop()
{
    let drop_checker = DropChecker::new();
    let dummy = DropChecker::new();
    let keep = Keep::new(drop_checker.drops());

    let guard = keep.read();

    assert_eq!(1, drop_checker.check());
    keep.write(dummy.drops());
    assert_eq!(1, drop_checker.check());
    assert_eq!((), guard.can_access());
    drop(guard);
    assert_eq!(0, drop_checker.check());
}
