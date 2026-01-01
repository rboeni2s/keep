use keep::*;


struct Cat(&'static str);
impl Cat
{
    fn meow(&self)
    {
        println!("{}: meow!", self.0);
    }
}


impl Drop for Cat
{
    fn drop(&mut self)
    {
        println!("{}: Zzz...", self.0);
    }
}


#[test]
fn roundtrip()
{
    let keep = Keep::new(39);
    let guard = keep.read();

    keep.write(42);

    assert_eq!(42, *keep.read());
    drop(keep);
    assert_eq!(39, *guard);
}


#[test]
fn drops()
{
    let keep = Keep::new(Cat("Fleur"));
    let guard = keep.read();
    guard.meow();
    drop(guard);
    drop(keep.clone());
    println!("Dropping fleur");
    drop(keep);

    let keep2 = Keep::new(Cat("Yuumi"));
    keep2.write(Cat("Evil Yuumi"));
    let guard2 = keep2.read();
    drop(keep2);
    guard2.meow();
    println!("Dropping Evil Yuumi");
    drop(guard2);
}
