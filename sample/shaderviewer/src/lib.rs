use std::sync::OnceLock;

use plug::prelude::Registry;

#[macro_use]
extern crate log;

pub mod application;
pub mod file_watcher;
pub mod window;

pub static REGISTRY: OnceLock<Registry<ServiceEvent>> = OnceLock::new();

#[non_exhaustive]
#[derive(Debug, Copy, Clone)]
pub enum ServiceEvent
{
    Init,
}


impl<T> plug::prelude::SimpleDispatch<ServiceEvent> for plug::prelude::EventEmitter<T> {}
