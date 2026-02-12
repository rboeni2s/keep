#[macro_use]
extern crate log;

use plug::prelude::*;
use shaderviewer::service::application::{Application, ApplicationEvent};
use shaderviewer::service::file_watcher::Watcher;
use shaderviewer::service::renderer::Renderer;
use shaderviewer::service::{REGISTRY, ServiceEvent};
use shaderviewer::window::{WindowEvent, WindowHandler};
use std::thread;
use winit::event_loop::{ControlFlow, EventLoop, EventLoopProxy};


fn main()
{
    if let Err(e) = run()
    {
        error!("Fatal Application Error: {e}");
    }
}


fn run() -> anyhow::Result<()>
{
    // Init logger
    if let Err(e) = plug::logger::init()
    {
        eprintln!("Failed to initialize logger: {e}");
    }

    // Build the registry
    let reg: Registry<ServiceEvent> = build_reg!(Application, Watcher, Renderer);

    // Share the registry globally
    let reg = REGISTRY.get_or_init(move || reg);

    // Create the window handler
    let window_event_emitter = reg.get_unchecked::<EventEmitter<WindowEvent>>();
    let mut window_handler = WindowHandler::new(window_event_emitter);

    // Create the event loop
    let event_loop: EventLoop<ApplicationEvent> = EventLoop::with_user_event().build()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    // Transfer a event loop proxy to the application
    reg.get_unchecked::<Application>()
        .set_window_proxy(event_loop.create_proxy());

    // Set Ctrl-C handler
    let proxy = event_loop.create_proxy();
    if let Err(e) = ctrlc::set_handler(move || {
        let proxy = proxy.clone();
        ctrlc_handler(proxy);
    })
    {
        error!("Failed to set Ctrl-C handler: {e}");
    }

    // Start the application on a different thread;
    let proxy = event_loop.create_proxy();
    let application_thread = thread::spawn(move || {
        info!("Starting Application Thread");
        reg.dispatch(&ServiceEvent::Init);
        let result = reg.get_unchecked::<Application>().run_application();
        let _ = proxy.send_event(ApplicationEvent::Close);
        info!("Exiting Application Thread");
        result
    });

    // Backup event-loop proxy
    let proxy = event_loop.create_proxy();

    // Run the window event loop
    info!("Starting Window Thread");
    let _ = event_loop.run_app(&mut window_handler);
    info!("Exiting Window Thread");

    // Wait for the application to exit and handle any errors
    match application_thread.join()
    {
        Ok(Err(e)) =>
        {
            error!("Fatal Application Error: {e}");
            let _ = proxy.send_event(ApplicationEvent::Close);
        }

        Err(e) =>
        {
            error!("Fatal Application Error: {e:?}");
            let _ = proxy.send_event(ApplicationEvent::Close);
        }

        _ => (),
    }

    Ok(())
}


/// Terminate the application and window threads
fn ctrlc_handler(proxy: EventLoopProxy<ApplicationEvent>)
{
    let _ = proxy.send_event(ApplicationEvent::Close);

    REGISTRY
        .get()
        .inspect(|r| _ = r.get::<Application>().inspect(|a| a.exit()));
}
