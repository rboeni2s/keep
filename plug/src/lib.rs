pub mod dispatch;
pub mod layer_context;
pub mod registry;
pub mod resolver;


#[cfg(feature = "macro")]
pub use proc_layer;

pub mod prelude
{
    pub use crate::dispatch::{LayerDispatch, NoDispatch, SimpleDispatch};
    pub use crate::layer_context::{LayerConstruct, StaticContext};
    pub use crate::registry::{Layer, Registry};
    pub use crate::resolver::Resolver;
    pub use crate::static_context;

    #[cfg(feature = "macro")]
    pub use proc_layer::{build_reg, service};
}


#[cfg(test)]
mod tests
{
    use super::prelude::*;
    use std::{fmt::Display, thread};

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

    impl SimpleDispatch<String> for Cat
    {
        fn simple_dispatch(&self, event: &String)
        {
            println!("{}: mission {}!!", self.0, event);
        }
    }

    #[test]
    fn look_and_feel()
    {
        print!("\n");
        let reg = Registry::new();
        reg.insert(Cat("Fleur"));
        reg.get_unchecked::<Cat>().meow();
        reg.dispatch(&"Sleep".to_string());
    }

    #[test]
    fn multiple_threads()
    {
        print!("\n");

        let reg = Registry::<String>::new();
        let reg_clone = reg.clone();

        let t2 = thread::spawn(move || {
            loop
            {
                if let Some(layer) = reg.get::<Cat>()
                {
                    layer.meow();
                    break;
                }
            }
        });

        let t1 = thread::spawn(move || {
            reg_clone.insert(Cat("Dimensional Fleur"));
            reg_clone.dispatch(&"Wormhole".into());
        });

        t1.join().unwrap();
        t2.join().unwrap();
    }

    #[test]
    fn resolver()
    {
        static A_CONTEXT: StaticContext = static_context!(A, []);
        struct A(&'static str);
        impl LayerConstruct for A
        {
            fn construct(_registry: &Registry) -> Self
            {
                Self("Test")
            }
        }

        static B_CONTEXT: StaticContext = static_context!(B, [A]);
        struct B(Layer<A>);
        impl LayerConstruct for B
        {
            fn construct(registry: &Registry) -> Self
            {
                Self(registry.get_unchecked())
            }
        }

        impl B
        {
            fn data(&self) -> &str
            {
                self.0.0
            }
        }

        let reg = Resolver::new()
            .add_ctx(&B_CONTEXT)
            .add_ctx(&A_CONTEXT)
            .build_reg()
            .unwrap();

        assert_eq!("Test", reg.get_unchecked::<B>().data());
    }

    #[test]
    fn no_dispatch_reg()
    {
        let reg = Registry::new();
        reg.insert(String::from("Test"));
        // reg.dispatch(..); // this can never work, because a value of NoDispatch cannot be constructed
        assert_eq!("Test", &**reg.get_unchecked::<String>())
    }

    #[test]
    fn dispatching_resolver()
    {
        static A_CTX: StaticContext<&str> = static_context!(A);
        struct A(&'static str);

        impl<T> SimpleDispatch<T> for A
        where
            T: Display,
        {
            fn simple_dispatch(&self, event: &T)
            {
                println!("{}: {}", self.0, event);
            }
        }

        impl<T: Display> LayerConstruct<T> for A
        {
            fn construct(_registry: &Registry<T, (), ()>) -> Self
            {
                Self("Gwen")
            }
        }

        let reg = Resolver::new().add_ctx(&A_CTX).build_reg().unwrap();

        print!("\n");
        reg.dispatch(&"Scissors out !!!");
    }

    #[cfg(feature = "macro")]
    #[test]
    fn proc_layer()
    {
        enum Action
        {
            Fight,
        }

        #[proc_layer::service]
        struct Chogath<Action>
        {
            #[default]
            health: usize,
        }

        impl Chogath
        {
            fn attack(&self, dmg: usize)
            {
                println!("Chogath: uh... -{dmg}");
                println!("Chogath: now i am at {} health...", self.health);
            }
        }

        impl SimpleDispatch<Action> for Chogath {}

        #[proc_layer::service]
        struct Gwen<Action>
        {
            #[layer]
            chogath: Chogath,

            #[value = "Snip Snip!!"]
            voice_line: &'static str,

            #[value = 39]
            dmg: usize,
        }

        impl SimpleDispatch<Action> for Gwen
        {
            fn simple_dispatch(&self, event: &Action)
            {
                match event
                {
                    Action::Fight =>
                    {
                        println!("Gwen: {}", self.voice_line);
                        self.chogath.attack(self.dmg);
                    }
                }
            }
        }

        let reg = proc_layer::build_reg!(Gwen, Chogath);

        print!("\n");
        reg.dispatch(&Action::Fight);
    }
}
