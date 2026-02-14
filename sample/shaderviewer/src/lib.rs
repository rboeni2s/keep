#[macro_use]
extern crate log;

pub mod service;
pub mod window;


#[macro_export]
macro_rules! rel {
    ($path:expr) => {
        concat!(env!("CARGO_MANIFEST_DIR"), $path)
    };
}
