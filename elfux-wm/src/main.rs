mod state;

use std::time::Duration;

use calloop::EventLoop;
use smithay::backend::renderer::element::memory::MemoryRenderBuffer;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::winit::{self, WinitEvent};
use smithay::reexports::wayland_server::Display;
use smithay::utils::{Size, Transform};

use state::ElfuxState;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    //start logging
    tracing_subscriber::fmt::init();
    tracing::info!("[ELFUX-WM] ==> Initialising compositor engine...");
}
