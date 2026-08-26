use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

fn main() {
    println!("[ELFUX] ==> Requesting reboot...");
    //send SIGTERM to PID 1
    if let Err(e) = kill(Pid::from_raw(1), Signal::SIGTERM) {
        eprintln!("[ERROR] ==> Failed to signal PID 1: {}", e);
    }
}