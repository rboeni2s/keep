use crate::{
    service::{ServiceEvent, application::Application, renderer::Renderer},
    window::InputEvent,
};
use cgmath::{Point3, Vector3};
use keep::Keep;
use plug::prelude::*;
use winit::keyboard::KeyCode;


#[service]
pub struct Camera<ServiceEvent>
{
    #[layer]
    application: Application,

    #[layer]
    renderer: Renderer,

    #[event(InputEvent)]
    input_events: EventSubscriber<InputEvent>,

    #[value = Keep::new(Point3::new(0.0, 0.0, 2.0))]
    position: Keep<Point3<f32>>,

    #[value = Keep::new(Vector3::new(0.0, 0.0, 0.0))]
    drift_pos: Keep<Vector3<f32>>,
    #[value = Keep::new(Vector3::new(0.0, 0.0, 0.0))]
    drift_neg: Keep<Vector3<f32>>,
}


impl Camera
{
    pub const SPEED: f32 = 5.0;
    pub const MAX_DRIFT: f32 = 2.5;

    fn event_handler(reg: &Registry<ServiceEvent>, delta: f32)
    {
        let me = reg.get_unchecked::<Camera>();

        let mut drift_pos = *me.drift_pos.read();
        let mut drift_neg = *me.drift_neg.read();

        while let Some(event) = me.input_events.pop()
        {
            match &*event
            {
                InputEvent::Pressed(key) =>
                {
                    match key
                    {
                        KeyCode::KeyA | KeyCode::ArrowLeft => drift_pos.x = Self::SPEED,
                        KeyCode::KeyD | KeyCode::ArrowRight => drift_neg.x = Self::SPEED,
                        KeyCode::KeyW | KeyCode::ArrowUp => drift_neg.y = Self::SPEED,
                        KeyCode::KeyS | KeyCode::ArrowDown => drift_pos.y = Self::SPEED,
                        _ => (),
                    }
                }

                InputEvent::Released(key) =>
                {
                    match key
                    {
                        KeyCode::KeyA | KeyCode::ArrowLeft => drift_pos.x = 0.0,
                        KeyCode::KeyD | KeyCode::ArrowRight => drift_neg.x = 0.0,
                        KeyCode::KeyW | KeyCode::ArrowUp => drift_neg.y = 0.0,
                        KeyCode::KeyS | KeyCode::ArrowDown => drift_pos.y = 0.0,
                        _ => (),
                    }
                }
            }
        }

        let drift = drift_pos - drift_neg;
        let mut pos = *me.position.read() + (drift * delta);
        pos.x = pos.x.clamp(-Self::MAX_DRIFT, Self::MAX_DRIFT);
        pos.y = pos.y.clamp(-Self::MAX_DRIFT, Self::MAX_DRIFT);

        me.drift_pos.write(drift_pos);
        me.drift_neg.write(drift_neg);
        me.position.write(pos);
        me.renderer.set_camera_origin(pos);
    }
}


impl SimpleDispatch<ServiceEvent> for Camera
{
    fn simple_dispatch(&self, event: &ServiceEvent)
    {
        #[allow(irrefutable_let_patterns)]
        if let ServiceEvent::Init = event
        {
            self.application.add_task(Self::event_handler);
            info!("Registered camera event handler");
        }
    }
}
