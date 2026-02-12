use plug::prelude::Registry;
use std::sync::OnceLock;


pub mod application;
pub mod file_watcher;
pub mod renderer;


pub static REGISTRY: OnceLock<Registry<ServiceEvent>> = OnceLock::new();


#[non_exhaustive]
#[derive(Debug, Copy, Clone)]
pub enum ServiceEvent
{
    Init,
}


impl<T> plug::prelude::SimpleDispatch<ServiceEvent> for plug::prelude::EventEmitter<T> {}
