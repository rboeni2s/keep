use winit::{
    application::ApplicationHandler,
    event::WindowEvent as PlatformWindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window as PlatformWindow, WindowAttributes, WindowId},
};

use crate::application::ApplicationEvent;
use plug::prelude::*;


#[derive(Debug, Clone, Copy)]
pub enum WindowEvent
{
    Close,
    Created,
}


pub struct WindowHandler
{
    window: Option<PlatformWindow>,
    window_event_emitter: Layer<EventEmitter<WindowEvent>>,
}


impl WindowHandler
{
    pub fn new(window_event_emitter: Layer<EventEmitter<WindowEvent>>) -> Self
    {
        Self {
            window: None,
            window_event_emitter,
        }
    }
}


impl ApplicationHandler<ApplicationEvent> for WindowHandler
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop)
    {
        match event_loop.create_window(WindowAttributes::default())
        {
            Ok(window) =>
            {
                self.window = Some(window);
                self.window_event_emitter.emit(WindowEvent::Created);
            }

            Err(e) =>
            {
                error!("Window creation failed: {e}");
                self.window_event_emitter.emit(WindowEvent::Close);
            }
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: PlatformWindowEvent,
    )
    {
        match event
        {
            PlatformWindowEvent::CloseRequested =>
            {
                info!("WindowClose requested");
                self.window_event_emitter.emit(WindowEvent::Close);
            }

            PlatformWindowEvent::RedrawRequested =>
            {
                //TODO
            }
            _ => (),
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ApplicationEvent)
    {
        if let ApplicationEvent::Close = event
        {
            event_loop.exit()
        }
    }
}
