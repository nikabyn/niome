use std::time::Duration;

use ::winit::window::{Fullscreen, WindowAttributes};
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::element::{Element, RenderElement};
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::winit::{self, WinitEvent, WinitGraphicsBackend};
use smithay::output::{Output, PhysicalProperties, Subpixel};
use smithay::reexports::calloop::EventLoop;
use smithay::utils::{Physical, Rectangle, Size, Transform};

use crate::state::Niome;

pub fn init_winit(
    event_loop: &mut EventLoop<Niome>,
    state: &mut Niome,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut backend, winit) = winit::init_from_attributes::<GlesRenderer>(
        WindowAttributes::default()
            .with_decorations(false)
            .with_fullscreen(Some(Fullscreen::Borderless(None))),
    )?;

    let output = Output::new(
        "winit".to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Niome".into(),
            model: "Winit".into(),
            serial_number: "Unknown".into(),
        },
    );
    let _global = output.create_global::<Niome>(&state.display_handle);
    state.output_config.mode.size = backend.window_size();
    output.change_current_state(
        Some(state.output_config.mode),
        Some(state.output_config.transform),
        Some(state.output_config.scale),
        Some((0, 0).into()),
    );
    output.set_preferred(state.output_config.mode);

    state.space.map_output(&output, (0, 0));

    let mut damage_tracker = OutputDamageTracker::from_output(&output);

    event_loop
        .handle()
        .insert_source(winit, move |event, _, state| match event {
            WinitEvent::Input(_input_event) => {}
            WinitEvent::Resized { size, .. } => resize(state, &output, size),
            WinitEvent::Redraw => redraw(state, &mut backend, &output, &mut damage_tracker),
            WinitEvent::CloseRequested => state.loop_signal.stop(),
            _ => {}
        })?;

    Ok(())
}

fn resize(state: &mut Niome, output: &Output, new_size: Size<i32, Physical>) {
    state.output_config.mode.size = new_size;
    output.change_current_state(Some(state.output_config.mode), None, None, None)
}

fn redraw(
    state: &mut Niome,
    backend: &mut WinitGraphicsBackend<GlesRenderer>,
    output: &Output,
    damage_tracker: &mut OutputDamageTracker,
) {
    let size = backend.window_size();
    let damage = Rectangle::from_size(size);

    {
        let (renderer, mut framebuffer) = backend.bind().unwrap();
        smithay::desktop::space::render_output(
            output,
            renderer,
            &mut framebuffer,
            1.0,
            0,
            [&state.space],
            &[] as &[WaylandSurfaceRenderElement<GlesRenderer>],
            damage_tracker,
            [0.1, 0.1, 0.1, 1.0],
        )
        .unwrap();
    }
    backend.submit(Some(&[damage])).unwrap();

    state.space.elements().for_each(|window| {
        window.send_frame(
            &output,
            state.start_time.elapsed(),
            Some(Duration::ZERO),
            |_, _| Some(output.clone()),
        )
    });

    state.space.refresh();
    // state.popups.cleanup();
    let _ = state.display_handle.flush_clients();

    // Ask for redraw to schedule new frame.
    backend.window().request_redraw();
}
