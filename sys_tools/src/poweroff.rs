use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

fn main() {
    println!("[SYS] ==> Sending shutdown signal to PID 1...");
    unsafe { libc::sync(); }
    if let Err(e) = kill(Pid::from_raw(1), Signal::SIGUSR2) {
        eprintln!("[ERROR] ==> Failed to signal PID 1: {}", e);
    }
}