use nix::mount::{mount, MsFlags};
use nix::sys::reboot::{reboot, RebootMode};
use nix::sys::signal::{self, SigHandler, Signal};
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
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

fn load_kernel_module(module_path: &str) -> io::Result<()> {
    let file = File::open(module_path)?;
    let params = CString::new("").expect("empty module params cannot contain null bytes");
    let result = unsafe { libc::syscall(libc::SYS_finit_module, file.as_raw_fd(), params.as_ptr(), 0) };

    if result == 0 {
        Ok(())
    } else {
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::EEXIST) {
            Ok(())
        } else {
            Err(err)
        }
    }
}

fn link_if_missing(source: &str, target: &str) {
    if Path::new(target).exists() || !Path::new(source).exists() {
        return;
    }

    if let Some(parent) = Path::new(target).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let _ = std::os::unix::fs::symlink(source, target);
}

fn expose_persistent_binaries(dir: &str) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(file_name) = path.file_name() {
                let target = std::path::Path::new("/bin").join(file_name);
                if !target.exists() {
                    let _ = std::os::unix::fs::symlink(&path, &target);
                }
            }
        }
    }
}

fn copy_dir_contents(source: &Path, dest: &Path) -> io::Result<()> {
    if !source.exists() {
        return Ok(());
    }

    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            copy_dir_contents(&source_path, &dest_path)?;
        } else if file_type.is_symlink() {
            if dest_path.exists() {
                continue;
            }
            let target = fs::read_link(&source_path)?;
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let _ = std::os::unix::fs::symlink(target, dest_path);
        } else if file_type.is_file() && !dest_path.exists() {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source_path, &dest_path)?;
            fs::set_permissions(&dest_path, entry.metadata()?.permissions())?;
        }
    }

    Ok(())
}

fn seed_persistent_base() {
    if Path::new("/mnt/var/lib/elfpkg/installed/base-system.list").exists()
        || !Path::new("/var/lib/elfpkg/installed/base-system.list").exists()
    {
        return;
    }

    boot_log("[INIT] ==> Seeding base userland onto persistent storage...");
    for dir in ["usr", "lib", "lib64", "etc", "var/lib/elfpkg"] {
        let source = Path::new("/").join(dir);
        let dest = Path::new("/mnt").join(dir);
        if let Err(e) = copy_dir_contents(&source, &dest) {
            boot_log(&format!("[WARN] ==> Failed to seed /mnt/{}: {}", dir, e));
        }
    }
}

fn shell_path() -> &'static str {
    if Path::new("/bin/bash").exists() {
        "/bin/bash"
    } else if Path::new("/mnt/usr/bin/bash").exists() {
        "/mnt/usr/bin/bash"
    } else {
        "/bin/sh"
    }
}

fn prefer_bash_as_sh() {
    if !Path::new("/bin/bash").exists() {
        return;
    }

    let _ = fs::remove_file("/bin/sh");
    let _ = std::os::unix::fs::symlink("/bin/bash", "/bin/sh");
}

fn main() -> ! {
    println!("*-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-*");
    println!("*-    *- ELFUX DISTRO INITIALISING -*    -*");
    println!("*-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-_-*");
    let _ = io::stdout().flush();
    thread::sleep(Duration::from_millis(500));

    unsafe {
        signal::signal(Signal::SIGPWR, SigHandler::Handler(handle_sigpwr)).ok();
        signal::signal(Signal::SIGUSR2, SigHandler::Handler(handle_sigpwr)).ok();
        signal::signal(Signal::SIGTERM, SigHandler::Handler(handle_sigterm)).ok();
    }

    //mount pseudo filesystem
    //linux kernel requires /proc and /sys to expose system info and process
    //mount proc
    boot_log("[INIT] ==> Mounting /proc...");
    if let Err(e) = mount(Some("proc"),"/proc",Some("proc"),MsFlags::empty(),None::<&str>) {
        eprintln!("[ERROR] ==> Failed to mount /proc: {}", e);
    }
    //mount sys
    boot_log("[INIT] ==> Mounting /sys...");
    if let Err(e) = mount(Some("sysfs"),"/sys",Some("sysfs"),MsFlags::empty(),None::<&str>) {
        eprintln!("[ERROR] ==> Failed to mount /sys: {}", e);
    }
    //mount dev
    boot_log("[INIT] ==> Mounting /dev (devtmpfs)...");
    let _ = mount(Some("devtmpfs"), "/dev", Some("devtmpfs"), MsFlags::empty(), None::<&str>);

    boot_log("[INIT] ==> Waiting for block device /dev/vda...");
    let mut drive_found = false;
    let mut drive_path = String::new();

    for candidate in &["/dev/vda", "/dev/sda"] { //retry for upto 2.5 seconds
        if Path::new(candidate).exists() {
            drive_found = true;
            drive_path = candidate.to_string();
            boot_log(&format!("[OK] ==> Drive found at: '{}'", drive_path));
            break;
        } 
    }
    //mount drive
    let _ = std::fs::create_dir_all("/mnt");
    if drive_found {
        if let Err(e) = mount(
            Some(drive_path.as_str()),
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

    // Mounted /dev/vda persistent drive at /mnt
    boot_log("[INIT] ==> Setting up persistent filesystem structure...");
    
    // Create target directories on persistent storage
    let _ = std::fs::create_dir_all("/mnt/tmp");
    let _ = std::fs::create_dir_all("/mnt/var/cache");
    let _ = std::fs::create_dir_all("/mnt/lib");
    let _ = std::fs::create_dir_all("/mnt/lib64");
    let _ = std::fs::create_dir_all("/mnt/usr/bin");
    let _ = std::fs::create_dir_all("/mnt/usr/lib");
    let _ = std::fs::create_dir_all("/mnt/bin");
    seed_persistent_base();

    // Redirect temp & cache
    let _ = std::fs::remove_dir_all("/tmp");
    let _ = std::os::unix::fs::symlink("/mnt/tmp", "/tmp");
    let _ = std::fs::create_dir_all("/var");
    let _ = std::os::unix::fs::symlink("/mnt/var/cache", "/var/cache");

    //check lib64 interpreter exists for glibc binaries
    let _ = std::fs::create_dir_all("/lib64");
    link_if_missing("/mnt/lib64/ld-linux-x86-64.so.2", "/lib64/ld-linux-x86-64.so.2");
    link_if_missing("/mnt/usr/lib/ld-linux-x86-64.so.2", "/lib64/ld-linux-x86-64.so.2");
    link_if_missing("/usr/lib/ld-linux-x86-64.so.2", "/lib64/ld-linux-x86-64.so.2");
    link_if_missing("/mnt/usr/bin/bash", "/bin/bash");
    prefer_bash_as_sh();

        unsafe {
            std::env::set_var("PATH", "/mnt/bin:/mnt/usr/bin:/bin:/usr/bin:/sbin:/usr/sbin");
            std::env::set_var("LD_LIBRARY_PATH", "/mnt/usr/lib:/mnt/lib:/usr/lib:/lib");
        }
    //symlink installed binaries from /mnt/bin into rootfs /bin
    expose_persistent_binaries("/mnt/bin");
    expose_persistent_binaries("/mnt/usr/bin");

    // Load VirtIO Network Driver Stack
    boot_log("[INIT] ==> Loading virtio_net driver stack...");
    let kver = "7.1.9-arch1-2";
    let base_path = format!("/lib/modules/{}/kernel/drivers/net", kver);
    let modules = ["failover.ko", "net_failover.ko", "virtio_net.ko"];

    for mod_file in &modules {
        let mod_path = format!("{}/{}", base_path, mod_file);
        if Path::new(&mod_path).exists() {
            match load_kernel_module(&mod_path) {
                Ok(()) => boot_log(&format!("[OK]   ==> Loaded {}", mod_file)),
                Err(e) => boot_log(&format!("[WARN] ==> Failed loading {}: {}", mod_file, e)),
            }
        } else {
            boot_log(&format!("[ERROR] ==> Missing required module file: {}", mod_path));
        }
    }

    boot_log("[INIT] ==> Loading DRM/KMS stack...");
    let drm_modules = [
        "/lib/modules/7.1.9-arch1-2/kernel/drivers/virtio/virtio_dma_buf.ko",
        "/lib/modules/7.1.9-arch1-2/kernel/driversgpu/drm/simpledrm.ko",
        "/lib/modules/7.1.9-arch1-2/kernel/drivers/gpu/drm/virtio/virtio-gpu.ko",
    ];

    for mod_path in &drm_modules {
        if Path::new(mod_path).exists() {
            match load_kernel_module(mod_path) {
                Ok(()) => boot_log(&format!("[OK] ==> Loaded {}", mod_path)),
                Err(e) => boot_log(&format!("[WARN] ==> Failed loading {}: {}", mod_path, e)),
            }
        }
    }
    /* boot_log("[OK]   ==> simpledrm is built into this kernel");
    for mod_path in [
        "/lib/modules/7.1.9-arch1-2/kernel/drivers/virtio/virtio_dma_buf.ko",
        "/lib/modules/7.1.9-arch1-2/kernel/drivers/gpu/drm/virtio/virtio-gpu.ko",
    ] {
        if Path::new(mod_path).exists() {
            match load_kernel_module(mod_path) {
                Ok(()) => boot_log(&format!("[OK]   ==> Loaded {}", mod_path)),
                Err(e) => boot_log(&format!("[WARN] ==> Failed loading {}: {}", mod_path, e)),
            }
        } else {
            boot_log(&format!("[WARN] ==> Missing DRM module file: {}", mod_path));
        }
    } */

    // Pause to allow kernel PCI probe to instantiate eth0 in sysfs
    thread::sleep(Duration::from_millis(500));

    // Network Setup
    let _ = std::fs::create_dir_all("/etc");
    let _ = std::fs::write("/etc/resolv.conf", "nameserver 10.0.2.3\nnameserver 8.8.8.8\n");

    let _ = Command::new("/bin/ip").args(["link", "set", "lo", "up"]).status();

    let mut net_iface: Option<String> = None;
    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            boot_log(&format!("[INIT] ==> Found network node: {}", name));
            if name != "lo" {
                net_iface = Some(name);
            }
        }
    }

    match net_iface {
        Some(iface) => {
            boot_log(&format!("[INIT] ==> Bringing up interface: {}", iface));
            let _ = Command::new("/bin/ip").args(["link", "set", &iface, "up"]).status();

            boot_log("[INIT] ==> Requesting DHCP lease...");
            let dhcp_status = Command::new("/bin/udhcpc")
                .args(["-i", &iface, "-n", "-q", "-s", "/usr/share/udhcpc/default.script"])
                .status();

            if !matches!(dhcp_status, Ok(s) if s.success()) {
                boot_log(&format!("[WARN] ==> DHCP failed. Setting static IP on {}...", iface));
                let _ = Command::new("/bin/ip").args(["addr", "add", "10.0.2.15/24", "dev", &iface]).status();
                let _ = Command::new("/bin/ip").args(["route", "add", "default", "via", "10.0.2.2", "dev", &iface]).status();
            }
        }
        None => {
            boot_log("[ERROR] ==> No physical network interface found in /sys/class/net!");
        }
    }

    println!("++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++");
    println!("+   __          ________ _      _____ ____  __  __ ______   _______ ____    ______ _      ______ _    ___   __   +");
    println!("+   \\ \\        / /  ____| |    / ____/ __ \\|  \\/  |  ____| |__   __/ __ \\  |  ____| |    |  ____| |  | \\ \\ / /   +");
    println!("+    \\ \\  /\\  / /| |__  | |   | |   | |  | | \\  / | |__       | | | |  | | | |__  | |    | |__  | |  | |\\ V /    +");
    println!("+     \\ \\/  \\/ / |  __| | |   | |   | |  | | |\\/| |  __|      | | | |  | | |  __| | |    |  __| | |  | | > <     +");
    println!("+      \\  /\\  /  | |____| |___| |___| |__| | |  | | |____     | | | |__| | | |____| |____| |    | |__| |/ . \\    +");
    println!("+       \\/  \\/   |______|______\\_____\\____/|_|  |_|______|    |_|  \\____/  |______|______|_|     \\____//_/ \\_\\   +");
    println!("+                                                                                                                +");
    println!("++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++");
                                                                                                           
                                                                                                        
    if Path::new("/bin/elfetch").exists() {
        let _ = Command::new("/bin/elfetch").status();
    }

    //spawn interactive rust shell/userland
    boot_log("[INIT] ==> Spawning primary user shell...");
    loop {
        let console = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/console")
            .expect("Failed to open /dev/console");

        let mut shell = Command::new(shell_path());
        shell
            .arg("-l")//login shell reads /etc/profile
            .stdin(Stdio::from(console.try_clone().expect("Failed to clone console for stdin")))
            .stdout(Stdio::from(console.try_clone().expect("Failed to clone console for stdout")))
            .stderr(Stdio::from(console));

        unsafe {
            shell.pre_exec(|| {
                if libc::setsid() < 0 {
                    return Err(io::Error::last_os_error());
                }

                if libc::ioctl(0, libc::TIOCSCTTY as _, 0) < 0 {
                    return Err(io::Error::last_os_error());
                }

                Ok(())
            });
        }

        let mut child = shell.spawn().expect("Failed to start user shell");

        loop {
            if SHUTDOWN_REQUESTED.load(Ordering::SeqCst) {
                boot_log("[INIT] ==> Poweroff signal received.");
                boot_log("[INIT] ==> Terminating user processes...");
                let _ = child.kill();
                let _ = child.wait();

                boot_log("[INIT] ==> Syncing filesystems...");
                unsafe { libc::sync(); }

                boot_log("[INIT] ==> Unmounting /mnt persistent storage...");
                let _ = nix::mount::umount("/mnt");
                let _ = nix::mount::umount("/proc");
                let _ = nix::mount::umount("/sys");

                boot_log("[INIT] ==> Powering off hardware.");
                thread::sleep(Duration::from_millis(200));
                let _ = reboot(RebootMode::RB_POWER_OFF);
            }

            if REBOOT_REQUESTED.load(Ordering::SeqCst) {
                boot_log("[INIT] ==> Reboot signal received.");
                boot_log("[INIT] ==> Terminating user processes...");
                let _ = child.kill();
                let _ = child.wait();

                boot_log("[INIT] ==> Syncing filesystems...");
                unsafe { libc::sync(); }

                boot_log("[INIT] ==> Unmounting /mnt persistent storage...");
                let _ = nix::mount::umount("/mnt");
                
                boot_log("[INIT] ==> Restarting system.");
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