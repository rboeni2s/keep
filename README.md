# Keep - a service creation toolkit

Keep provides various tools and utilities to build modular services split across 3 crates:
* `keep` provides a concurrent memory reclamaition system
* `plugmap` provides a concurrent hashmap
* `plug` provides a service management and initialization solution *(please see the example below)*.

## Short Example
This Example constructs a layered service from two layers: `FakeDB` and `FakeUserService`.
It demonstrates the creation of services and automatic service instanciation (through `build_reg!(..)`).
This is a simple example and does not cover service event dispatching using `LayerDispatch`.

The example below is the same example as the one in `plug/examples/simple.rs`.
you can run the example like this:
```bash
cargo run --example simple
```

```rust
use keep::{Guard, Keep};
use plug::prelude::*;
use plugmap::PlugMap;
use std::{thread, time::Duration};


/// lets use just a usize as user id.
/// I'll typedef it as `UserID` to make the code
/// using it easier to read.
type UserID = usize;


/// Example User data. Each user just has a name :)
struct User
{
    name: String,
}


/// A "fake database" service.
///
/// In order to not complicate this example with a real database connection
/// this service will use a hashmap to emulate a database.
#[service]
struct FakeDB
{
    #[default]
    map: PlugMap<UserID, User>,
}


impl FakeDB
{
    /// Adds a new user into the fake database
    fn insert_user(&self, id: UserID, user: User)
    {
        self.map.insert(id, user);
    }

    /// Tries to fetch a user from the fake database
    fn fetch_user(&self, id: UserID) -> Option<Guard<User>>
    {
        self.map.get(&id)
    }
}


/// This is a fake user service.
///
/// This service is able to register new users and search for existing ones.
#[service]
struct FakeUserService
{
    #[layer]
    db: FakeDB,

    #[value = Keep::new(0)]
    next_id: Keep<UserID>,
}


impl FakeUserService
{
    /// Registers a new user
    fn register_user(&self, name: &str)
    {
        // Get the next user id and increase it by one.
        //
        // Btw: this is not thread-safe but will be enough for this example...
        let id = *self.next_id.read();
        self.next_id.write(id + 1);

        // Create the new user...
        let user = User {
            name: name.to_string(),
        };

        // ...and store it!
        self.db.insert_user(id, user);
    }

    /// Tries to get the username of the user with the userid`id`
    fn get_username(&self, id: UserID) -> Option<String>
    {
        self.db.fetch_user(id).map(|f| f.name.clone())
    }
}


fn main()
{
    // This is the registry that will manage our two services.
    let registry = build_reg!(FakeUserService, FakeDB);

    // Lets construct a hypothetical scenario where one thread (worker_a)
    // waits for a user named "Test" and another thread (worker_b), that sleeps for
    // a few seconds and then creates a few users. I'll have this be two different threads,
    // just to demonstrate ability of plug's layered service system to work concurrently.

    // spawn worker_a
    let reg = registry.clone();
    let worker_a = thread::spawn(move || worker_a_task(reg));

    // spawn worker_b
    let reg = registry.clone();
    let worker_b = thread::spawn(move || worker_b_task(reg));

    // Wait for both threads to finish
    let _ = worker_a.join();
    let _ = worker_b.join();

    println!("Both workers are finished!!!");
}


/// This is the task that worker a will perform
fn worker_a_task(reg: Registry)
{
    println!("A: Waiting for user 'Test' to be registered");

    // Get a handle the the instance of the fake user service.
    let user_service = reg.get_unchecked::<FakeUserService>();

    loop
    {
        // Check if a user with an id of 2 exists and is named "Test"
        if let Some(username) = user_service.get_username(2)
        {
            if username == "Test"
            {
                println!("A: The user 'Test' was registered with id 2");
                break;
            }
        }

        // Sleep to avoid wasting cpu...
        thread::sleep(Duration::from_millis(200));
    }
}


/// This is the task that worker b will perform
fn worker_b_task(reg: Registry)
{
    // Sleep 4 seconds
    for i in (1..=4).rev()
    {
        println!("B: Zzz... {i}");
        thread::sleep(Duration::from_secs(1));
    }

    println!("B: Registering 3 users NotTest, tseT and Test right now!!");

    // Get a handle the the instance of the fake user service.
    let user_service = reg.get_unchecked::<FakeUserService>();

    user_service.register_user("NotTest");
    user_service.register_user("tseT");
    user_service.register_user("Test");
}
```
