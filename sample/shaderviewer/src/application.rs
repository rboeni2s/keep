use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

use crate::{REGISTRY, ServiceEvent, window::WindowEvent};
use anyhow::Context;
use plug::prelude::*;
use plugmap::DynBuffer;


#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub enum ApplicationEvent
{
    Start,
    Close,
}


#[service]
pub struct Application<ServiceEvent>
{
    #[event(WindowEvent)]
    window_events: EventSubscriber<WindowEvent>,

    #[layer]
    app_event_emitter: EventEmitter<ApplicationEvent>,

    #[value = AtomicBool::new(false)]
    should_quit: AtomicBool,

    #[default]
    tasks: DynBuffer<Box<dyn Fn(&Registry<ServiceEvent>)>>,
}


impl Application
{
    pub fn run_application(&self) -> anyhow::Result<()>
    {
        let reg = REGISTRY.get().context("Failed to fetch global registry")?;

        self.app_event_emitter.emit(ApplicationEvent::Start);

        loop
        {
            let frame_start = Instant::now();

            if self.should_quit.load(Ordering::Acquire)
            {
                break;
            }

            while let Some(window_event) = self.window_events.pop()
            {
                match &*window_event
                {
                    WindowEvent::Close => self.exit(),
                    WindowEvent::Created => (),
                }
            }

            // Run all main loop tasks...
            for task in self.tasks.snapshot().as_ref()
            {
                task(reg);
            }

            let sleep_time = Duration::from_millis(16) - frame_start.elapsed();
            thread::sleep(sleep_time);
        }

        Ok(())
    }

    /// Exits the application, respecting only the first call to exit.
    pub fn exit(&self)
    {
        if let Ok(false) =
            self.should_quit
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
        {
            self.app_event_emitter.emit(ApplicationEvent::Close);
        }
    }

    /// Adds a task to be executed repeatedly in the main-loop
    pub fn add_task(&self, task: impl Fn(&Registry<ServiceEvent>) + 'static)
    {
        let boxed: Box<dyn Fn(&Registry<ServiceEvent>)> = Box::new(task);
        self.tasks.push(boxed);
    }
}


impl SimpleDispatch<ServiceEvent> for Application {}
