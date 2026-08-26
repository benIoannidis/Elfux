use nix::mount::{mount, MsFlags};
use nix::sys::reboot::{reboot, RebootMode};
use nix::sys::signal::{self, SigHandler, Signal};
use std::io::{self, Write};
use std::process::Command;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);
static REBOOT_REQUESTED: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_sigpwr(_: libc::c_int) {
    SHUTDOWN_REQUESTED.store(true, Ordering::SeqCst);
}

extern "C" fn handle_sigterm(_: libc::c_int) {
    REBOOT_REQUESTED.store(true, Ordering::SeqCst);
}

fn boot_log(msg: &str) {
    println!("{}", msg);
    let _ = io::stdout().flush();
    //delay
    thread::sleep(Duration::from_millis(50));
}
fn main() -> ! {
    println!("*-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-*");
    println!("*-    *- ELFUX DISTRO INITIALISING -*    -*");
    println!("*-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-*");
    let _ = io::stdout().flush();
    thread::sleep(Duration::from_millis(200));

    unsafe {
        signal::signal(Signal::SIGPWR, SigHandler::Handler(handle_sigpwr)).ok();
        signal::signal(Signal::SIGTERM, SigHandler::Handler(handle_sigterm)).ok();
    }

    //mount pseudo filesystem
    //linux kernel requires /proc and /sys to expose system info and process
    boot_log("[INIT] ==> Mounting /proc...");
    if let Err(e) = mount(
        Some("proc"),
        "/proc",
        Some("proc"),
        MsFlags::empty(),
        None::<&str>
    ) {
        eprintln!("[ERROR] ==> Failed to mount /proc: {}", e);
    }

    boot_log("[INIT] ==> Mounting /sys...");
    if let Err(e) = mount(
        Some("sysfs"),
        "/sys",
        Some("sysfs"),
        MsFlags::empty(),
        None::<&str>
    ) {
        eprintln!("[ERROR] ==> Failed to mount /sys: {}", e);
    }

    //mount dev
    boot_log("[INIT] ==> Mousing /dev (devtmpfs)...");
    let _ = mount(Some("devtmpfs"), "/dev", Some("devtmpfs"), MsFlags::empty(), None::<&str>);

    boot_log("[INIT] ==> Waiting for block device /dev/vda...");
    let mut drive_found = false;
    let mut drive_path = "";

    for _ in 0..50 { //retry for upto 2.5 seconds
        if Path::new("/dev/vda").exists() {
            drive_found = true;
            drive_path = "/dev/vda";
            boot_log(&format!("[OK] ==> Drive found at: '{}'", drive_path));
            break;
        } else if Path::new("/dev/sda").exists() {
            drive_found = true;
            drive_path = "/dev/sda";
            boot_log(&format!("[OK] ==> Drive found at: '{}'", drive_path));
            break;
        }
        boot_log("[INIT] ==> Waiting...");    
        thread::sleep(Duration::from_millis(50));
    }
    //mount drive
    let _ = std::fs::create_dir_all("/mnt");
    if drive_found {
        if let Err(e) = mount(
            Some(drive_path),
            "/mnt",
            Some("ext4"),
             MsFlags::empty(),
            None::<&str>,
        ) {
            boot_log(&format!("[WARN] ==> Could not mount {}: {}", drive_path, e))
        } else {
            boot_log(&format!("[OK] ==> Persistent storage ({}) mounted at /mnt.", drive_path));
        }
    } else {
        boot_log("[WARN] ==> /dev/sda not found on bus.");
    }


    //spawn interactive rust shell/userland
    boot_log("[INIT] ==> Spawning primary user shell...");
    loop {
        let mut child = Command::new("/bin/sh")
            .arg("-l")//login shell reads /etc/profile
            .spawn()
            .expect("Failed to start user shell");

        loop {
            if SHUTDOWN_REQUESTED.load(Ordering::SeqCst) {
                boot_log("[INIT] ==> Poweroff signal received! Syncing and shutting down...");
                let _ = child.kill();
                unsafe { libc::sync(); }
                thread::sleep(Duration::from_millis(200));
                let _ = reboot(RebootMode::RB_POWER_OFF);
            }

            if REBOOT_REQUESTED.load(Ordering::SeqCst) {
                boot_log("[INIT] ==> Reboot signal received! Syncing and restarting...");
                let _ = child.kill();
                unsafe { libc::sync(); }
                thread::sleep(Duration::from_millis(200));
                let _ = reboot(RebootMode::RB_AUTOBOOT);
            }

            if let Ok(Some(_)) = child.try_wait() {
                boot_log("[INIT] ==> Shell exited. Respawning...");
                break;
            }

            thread::sleep(Duration::from_millis(100));
        }
    }
}