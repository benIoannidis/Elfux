use smithay::reexports::wayland_server::DisplayHandle;

pub struct ElfuxState {
    pub running: bool,
    pub display_handle: DisplayHandle,
}

impl ElfuxState {
    pub fn new(display_handle: DisplayHandle) -> Self {
        Self {
            running: true,
            display_handle,
        }
    }
}