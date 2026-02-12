use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

use crate::service::{REGISTRY, ServiceEvent};
use crate::window::WindowEvent;
use anyhow::Context;
use keep::Keep;
use plug::prelude::*;
use plugmap::DynBuffer;
use winit::event_loop::EventLoopProxy;


type TaskFn = Box<dyn Fn(&Registry<ServiceEvent>) + 'static>;


#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub enum ApplicationEvent
{
    Start,
    Close,
    TickWindow,
}


#[service]
pub struct Application<ServiceEvent>
{
    #[event(WindowEvent)]
    window_events: EventSubscriber<WindowEvent>,

    #[value = Keep::new(None)]
    window_loop_proxy: Keep<Option<EventLoopProxy<ApplicationEvent>>>,

    #[layer]
    app_event_emitter: EventEmitter<ApplicationEvent>,

    #[value = AtomicBool::new(false)]
    should_quit: AtomicBool,

    #[default]
    tasks: DynBuffer<TaskFn>,
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

            if let Some(w) = &*self.window_loop_proxy.read()
            {
                w.send_event(ApplicationEvent::TickWindow)?;
            }

            while let Some(window_event) = self.window_events.pop()
            {
                match &*window_event
                {
                    WindowEvent::Close => self.exit(),
                    _ => (),
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

    pub fn set_window_proxy(&self, proxy: EventLoopProxy<ApplicationEvent>)
    {
        self.window_loop_proxy.write(Some(proxy));
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
        let boxed: TaskFn = Box::new(task);
        self.tasks.push(boxed);
    }
}


impl SimpleDispatch<ServiceEvent> for Application {}
