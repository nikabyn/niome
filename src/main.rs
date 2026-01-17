use smithay::reexports::calloop::EventLoop;
use smithay::reexports::wayland_server::Display;

mod state;
use state::{ClientState, Niome};

mod winit;
use winit::init_winit;

mod handlers;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    let mut event_loop: EventLoop<Niome> = EventLoop::try_new()?;
    let display: Display<Niome> = Display::new()?;
    let mut state = Niome::new(&mut event_loop, display);

    init_winit(&mut event_loop, &mut state)?;

    state.spawn_client("alacritty", &[]);

    event_loop.run(None, &mut state, |_| {})?;
    Ok(())
}

fn init_logging() {
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    } else {
        tracing_subscriber::fmt().init();
    }
}
