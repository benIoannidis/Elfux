use nix::sys::signal::{kill, Signal};
use nix::unistd::{getpid, Pid};
use std::io::{self, Write};

fn main() {
    let my_pid = getpid();
    println!("\n[POWEROFF BINARY] ==> Executing custom Rust poweroff (PID: {})...", my_pid);
    let _ = io::stdout().flush();

    println!("[POWEROFF BINARY] ==> Sending SIGPWR to PID 1...");
    let _ = io::stdout().flush();

    match kill(Pid::from_raw(1), Signal::SIGPWR) {
        Ok(_) => {
            println!("[POWEROFF BINARY] ==> SIGPWR send successfully.");
            let _ = io::stdout().flush();
        }
        Err(e) => {
            eprintln!("[POWEROFF BINARY ERROR] ==> Failed to send SIGPWR: {}", e);
            let _ = io::stdout().flush();
        }
    }
}