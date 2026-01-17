use std::ffi::OsString;
use std::sync::Arc;
use std::time::Instant;

use smithay::desktop::{Space, Window};
use smithay::input::{Seat, SeatState};
use smithay::output;
use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::{self, EventLoop, Interest, LoopSignal, PostAction};
use smithay::reexports::wayland_server::Display;
use smithay::reexports::wayland_server::DisplayHandle;
use smithay::reexports::wayland_server::backend::{ClientData, ClientId, DisconnectReason};
use smithay::utils::{Size, Transform};
use smithay::wayland::compositor::{CompositorClientState, CompositorState};
use smithay::wayland::output::OutputManagerState;
use smithay::wayland::selection::data_device::DataDeviceState;
use smithay::wayland::shell::xdg::XdgShellState;
use smithay::wayland::shm::ShmState;
use smithay::wayland::socket::ListeningSocketSource;
use tracing::{error, info};

pub struct Niome {
    pub start_time: Instant,
    pub output_config: OutputConfig,

    // Used to stop the event_loop
    pub loop_signal: LoopSignal,
    pub space: Space<Window>,
    pub socket_name: OsString,

    pub display_handle: DisplayHandle,
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<Self>,
    pub data_device_state: DataDeviceState,

    pub seat: Seat<Self>,
}

impl Niome {
    pub fn new(event_loop: &mut EventLoop<Self>, display: Display<Self>) -> Self {
        let start_time = std::time::Instant::now();
        let display_handle = display.handle();

        let compositor_state = CompositorState::new::<Niome>(&display_handle);
        let shm_state = ShmState::new::<Niome>(&display_handle, vec![]);
        let xdg_shell_state = XdgShellState::new::<Niome>(&display_handle);
        let data_device_state = DataDeviceState::new::<Niome>(&display_handle);

        let output_manager_state =
            OutputManagerState::new_with_xdg_output::<Niome>(&display_handle);

        let mut seat_state = SeatState::new();
        let mut seat = seat_state.new_wl_seat(&display_handle, "winit");
        seat.add_keyboard(Default::default(), 200, 200).unwrap();
        seat.add_pointer();

        let space = Space::default();

        let socket_name = init_wayland_listener(display, event_loop);

        let loop_signal = event_loop.get_signal();

        Niome {
            start_time,
            output_config: OutputConfig::default(),

            space,
            loop_signal,
            socket_name,

            display_handle,
            compositor_state,
            xdg_shell_state,
            shm_state,
            output_manager_state,
            seat_state,
            data_device_state,
            seat,
        }
    }

    pub fn spawn_client(&self, command: &str, args: &[&str]) {
        let result = std::process::Command::new(command)
            .args(args)
            .env("WAYLAND_DISPLAY", &self.socket_name)
            .spawn();
        let command_formatted =
            command.to_owned() + if args.is_empty() { "" } else { " " } + &args.join(" ");
        match result {
            Ok(_) => info!("Spawned `{command_formatted}`"),
            Err(err) => error!("Failed to spawn `{command_formatted}`: {err}"),
        }
    }
}

fn init_wayland_listener(display: Display<Niome>, event_loop: &mut EventLoop<Niome>) -> OsString {
    // Creates a new listening socket, automatically choosing the next available `wayland` socket name.
    let listening_socket = ListeningSocketSource::new_auto().unwrap();

    // Get the name of the listening socket.
    // Clients will connect to this socket.
    let socket_name = listening_socket.socket_name().to_os_string();

    let loop_handle = event_loop.handle();

    loop_handle
        .insert_source(listening_socket, move |client_stream, _, state| {
            // Inside the callback, you should insert the client into the display.
            //
            // You may also associate some data with the client when inserting the client.
            state
                .display_handle
                .insert_client(client_stream, Arc::new(ClientState::default()))
                .unwrap();
        })
        .expect("Failed to init the wayland event source.");

    // You also need to add the display itself to the event loop, so that client events will be processed by wayland-server.
    loop_handle
        .insert_source(
            Generic::new(display, Interest::READ, calloop::Mode::Level),
            |_, display, state| {
                // Safety: we don't drop the display
                unsafe {
                    display.get_mut().dispatch_clients(state).unwrap();
                }
                Ok(PostAction::Continue)
            },
        )
        .unwrap();

    socket_name
}

#[derive(Debug)]
pub struct OutputConfig {
    pub mode: output::Mode,
    pub transform: Transform,
    pub scale: output::Scale,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            mode: output::Mode {
                size: Size::new(0, 0),
                refresh: 60_000,
            },
            transform: Transform::Flipped180,
            scale: output::Scale::Fractional(1.25),
        }
    }
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}
impl ClientData for ClientState {
    fn initialized(&self, client_id: ClientId) {
        info!("Client({client_id:?}) initialized");
    }

    fn disconnected(&self, client_id: ClientId, reason: DisconnectReason) {
        info!("Client({client_id:?}) diconnected: {reason:?}");
    }
}
