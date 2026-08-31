pub struct ElfuxState {
    pub running: bool,
}

impl ElfuxState {
    pub fn new() -> Self {
        Self { running: true }
    }
}