use crate::{
    service::ServiceEvent,
    service::application::{Application, ApplicationEvent},
};
use plug::prelude::*;


#[service]
pub struct Watcher<ServiceEvent>
{
    #[layer]
    app: Application,

    #[event(ApplicationEvent)]
    app_events: EventSubscriber<ApplicationEvent>,
}


impl Watcher
{
    fn tick(_reg: &Registry<ServiceEvent>, _delta: f32)
    {
        // info!("i am being ticked...");
    }
}


impl SimpleDispatch<ServiceEvent> for Watcher
{
    fn simple_dispatch(&self, event: &ServiceEvent)
    {
        #[allow(irrefutable_let_patterns)]
        if let ServiceEvent::Init = event
        {
            self.app.add_task(Self::tick);
        }
    }
}
