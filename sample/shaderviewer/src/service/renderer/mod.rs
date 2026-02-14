use crate::service::ServiceEvent;
use crate::service::application::Application;
use crate::window::WindowEvent;
use keep::Keep;
use plug::prelude::*;
use winit::window::Window as PlatformWindow;


mod pipeline;
mod state;
use state::RenderState;


#[service]
pub struct Renderer<ServiceEvent>
{
    #[event(WindowEvent)]
    window_events: EventSubscriber<WindowEvent>,

    #[layer]
    application: Application,

    #[value = Keep::new(None)]
    state: Keep<Option<RenderState>>,
}


impl SimpleDispatch<ServiceEvent> for Renderer
{
    fn simple_dispatch(&self, event: &ServiceEvent)
    {
        match event
        {
            ServiceEvent::Init =>
            {
                self.application.add_task(Self::event_handler);
                info!("Registered renderer event handler")
            }
        }
    }
}


impl Renderer
{
    pub fn init_from_window(&self, window: Guard<PlatformWindow>) -> anyhow::Result<()>
    {
        let render_state = pollster::block_on(RenderState::new(window))?;
        self.state.write(Some(render_state));
        Ok(())
    }

    fn event_handler(reg: &Registry<ServiceEvent>)
    {
        let renderer = reg.get_unchecked::<Renderer>();

        let mut resize_done = false;
        let mut frame_done = false;

        let state = renderer.state.read();
        let state = match &*state
        {
            Some(state) => state,
            None => return,
        };


        while let Some(e) = renderer.window_events.pop()
        {
            if !resize_done && let WindowEvent::Resize(w, h) = &*e
            {
                let _ = state.resize(*w, *h);
                resize_done = true;
            }

            if !frame_done && let WindowEvent::Redraw = &*e
            {
                let _ = state.render();
                frame_done = true;
            }
        }
    }
}
