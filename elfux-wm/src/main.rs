mod state;

use std::time::Duration;

use calloop::EventLoop;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::{Frame, Renderer};
use smithay::backend::winit::{self, WinitEvent};
use smithay::reexports::wayland_server::Display;
use smithay::utils::{Size, Transform};

use state::ElfuxState;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize Logging
    tracing_subscriber::fmt::init();
    tracing::info!("[ELFUX-WM] ==> Launching Elfux Desktop Compositor on KDE Plasma...");

    // 2. Setup Calloop Event Loop & Wayland Server Display
    let mut event_loop: EventLoop<ElfuxState> = EventLoop::try_new()?;
    let mut display: Display<ElfuxState> = Display::new()?;
    let mut state = ElfuxState::new();

    // 3. Initialize Winit Backend with Explicit Window Attributes
    let (mut backend, mut winit_event_loop) = winit::init_from_builder::<GlesRenderer, _>(
        winit::WinitGraphicsBackend::builder()
            .with_title("Elfux Desktop Environment")
            .with_inner_size(1280, 800),
        None,
    )
    .map_err(|e| format!("Failed to initialize render backend: {:?}", e))?;

    // Force window to take focus and be visible on host compositor
    backend.window().set_visible(true);
    backend.window().focus_window();

    tracing::info!("[ELFUX-WM] ==> Host window mapped and visible.");

    // Helper to draw and present a frame
    let render_frame = |backend: &mut smithay::backend::winit::WinitGraphicsBackend<GlesRenderer>| {
        let size = backend.window_size();
        if let Ok((renderer, mut target)) = backend.bind() {
            let render_result = renderer.render(
                &mut target,
                Size::from((size.w, size.h)),
                Transform::Normal,
            );

            if let Ok(mut frame) = render_result {
                // Clear screen to muted green (#596e59)
                let _ = frame.clear([0.35, 0.43, 0.35, 1.0].into(), &[]);
                let _ = frame.finish();
            }
        }
    };

    // Render immediate first frame
    render_frame(&mut backend);

    // 4. Main Compositor Engine Loop
    while state.running {
        // Dispatch pending Wayland client messages
        display.dispatch_clients(&mut state)?;

        // Process Winit input and redraw events
        let _ = winit_event_loop.dispatch_new_events(|event| match event {
            WinitEvent::Resized { size, .. } => {
                tracing::info!("[ELFUX-WM] ==> Window resized to: {:?}", size);
                backend.window().request_redraw();
            }
            WinitEvent::CloseRequested => {
                tracing::info!("[ELFUX-WM] ==> Exit signal received.");
                state.running = false;
            }
            WinitEvent::Redraw => {
                render_frame(&mut backend);
            }
            _ => {}
        });

        // Trigger continuously at 60 FPS
        render_frame(&mut backend);
        backend.window().request_redraw();

        // Flush updates to Wayland clients
        display.flush_clients()?;

        // Tick event loop
        event_loop.dispatch(Some(Duration::from_millis(16)), &mut state)?;
    }

    tracing::info!("[ELFUX-WM] ==> Compositor loop closed cleanly.");
    Ok(())
}