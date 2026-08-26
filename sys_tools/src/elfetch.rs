use std::fs;

const ASCII_LOGO: &str = r#"                                                           
                             ‡‰uuu‰‰‰‰‰‰‰‰‰‰‰uuu‰‰                         
                          ‰u‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰u‰½                     
                       ‡u‰‰‰uu‰‰uuu‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰                   
                     ½uu‰ ¼u‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰              
                       ½u‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰            
                     ‰u‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰u‰           
                    ‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰½          
                   ‰u‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰¼ ½u‰‰‰‰‰          
                  ‡u‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰u‰    ½u‰‰‰½          
                  u‰‰‰‰‰‰‰‰  u½‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‡  ‰uu  u‰‰u           
                 ½‰‰‰‰‰‰‰‰‰  ‰    ½‰‰‰‰‰‰‰‰‰‰‰‰u‰    ‰‰‰‰uu u‰‰¼           
                 ‰‰‰‰‰‰‰‰‰‰  u‰‰‰½   ‰u‰‰‰u‰     ¼u‰‰‰‰‰‰u¼ uu‡            
                 ‰‰‰‰‰‰‰‰‰‰‰ ‡u‰‰‰‰u½  ‰u¼  uuu‰‰‰‰‰‰‰‰‰‰u  ‰‰             
                 ‰‰‰‰‰‰‰‰‰‰‰z ‰‰‰‰‰‰‰u‰   ‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰u‰                
                 ‰‰‰‰‰‰‰‰‰‰‰u¼ ½u‰‰‰‰‰‰u½u‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰u                
                 ‰‰‰‰‰‰‰‰‰‰‰‰u‰  u‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰u¼              
                 ¼‰‰‰‰‰‰‰‰‰‰‰‰‰‰  ‰u‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰u‰             
                 ‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰u   ‰u‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰u             
                 ¼‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰uu½  ‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰u                
                 ½‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰u½  ½u‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰u               
                ¼u‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰u  ¼ ½u‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰                
               ‡‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰  ‰u‰ ‰uu‰‰‰‰‰‰‰‰‰‰‰‰‰u¼                
             ‰‰u‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰u‡ ‰‰‰‰‰¼ ¼u‰‰‰‰‰‰‰‰‰‰‰u                  
       ‰u‰u‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰u‰ ‰‰‰‰‰‰u‰½  ½‰‰‰‰‰‰‰‰‰u                  
         ½‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰‰  ‰‰‰‰‰‰‰‰‰‰‰     ½uu‰‰u¼                  
            ‰u‰‰‰‰‰‰‰‰‰‰‰‰‰‰u‰  ½u‰‰‰‰‰‰‰‰‰‰‰                              
                ‰‰‰u‰‰‰‰u‰½   ‰uu‰‰‰‰‰‰‰‰‰‰‰½                              
                             ½‰½‰‰u‰‰‰‰‰‰‰‰‰½                              
                                    ‰uu‰‰‰‰‰u                              
                                       ¼u‰‰‰‰‰                             
                                          ‰‰u‰u‡                           
"#;
fn get_cpu_model() -> String {
    if let Ok(content) = fs::read_to_string("/proc/cpuinfo") {
        for line in content.lines() {
            if line.starts_with("model name") {
                if let Some((_, value)) = line.split_once(':') {
                    return value.trim().to_string();
                }
            }
        }
    }
    "x86_64 Compatible CPU".to_string()
}

fn get_gpu_info() -> String {
    //check /sys/class/drm for display devices
    if let Ok(entries) = fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let path = entry.path().join("device/vendor");
            if path.exists() {
                return "QEMU Virtual Video Controller".to_string();
            }
        }
    }

    if let Ok(entries) = fs::read_dir("/sys/bus/pci/devices") {
        for entry in entries.flatten() {
            let class_path = entry.path().join("class");
            if let Ok(class_hex) = fs::read_to_string(class_path) {
                //0x030000 is Display/VGA controller class
                if class_hex.trim().starts_with("0x03") {
                    return "Standard VGA Graphics Adapter".to_string();
                }
            }
        }
    }

    "Integrated / Virtual Graphics".to_string()
}


fn get_memory_info() -> String {
    let mut total_kb = 0u64;
    let mut free_kb = 0u64;
    let mut buffers_kb = 0u64;
    let mut cached_kb = 0u64;

    if let Ok(content) = fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let val = parts[1].parse::<u64>().unwrap_or(0);
                match parts[0] {
                    "MemTotal:" => total_kb = val,
                    "MemFree:" => free_kb = val,
                    "Buffers:" => buffers_kb = val,
                    "Cached:" => cached_kb = val,
                    _ => {}
                }
            }
        }
    }

    let used_kb = total_kb.saturating_sub(free_kb + buffers_kb + cached_kb);
    let used_mb = used_kb / 1024;
    let total_mb = total_kb / 1024;
    let pct = if total_kb > 0 {
        (used_kb as f64 / total_kb as f64) * 100.0
    } else {
        0.0
    };

    format!("{} MiB / {} MiB ({:.1}%)", used_mb, total_mb, pct)
}

fn get_disk_info() -> String {
    let target_path = if std::path::Path::new("/mnt").exists() {
        "/mnt"
    } else {
        "/"
    };

    let path = std::ffi::CString::new(target_path).unwrap();
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };

    if unsafe { libc::statvfs(path.as_ptr(), &mut stat) } == 0 {
        let block_size = stat.f_frsize as u64;
        let total_bytes = stat.f_blocks as u64 * block_size;
        let free_bytes = stat.f_bfree as u64 * block_size;
        let used_bytes = total_bytes.saturating_sub(free_bytes);

        let used_gb = used_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let total_gb = total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        let pct = if total_bytes > 0 {
            (used_bytes as f64 / total_bytes as f64) * 100.0
        } else {
            0.0
        };

        if total_gb >= 1.0 {
            format!("{:.2} GiB / {:.2} GiB ({:.1}%)",used_gb,total_gb, pct)
        } else {
            let used_mb = used_bytes / (1024 * 1024);
            let total_mb = total_bytes / (1024 * 1024);
            format!("{} MiB / {} MiB ({:.1}%)", used_mb, total_mb, pct)
        }
    }else {
        "N/A".to_string()
    }
}

fn get_uptime() -> String {
    if let Ok(content) = fs::read_to_string("/proc/uptime") {
        if let Some(first) = content.split_whitespace().next() {
            if let Ok(seconds) = first.parse::<f64>() {
                let total_secs = seconds.round() as u64;
                let secs = total_secs % 60;
                let mins = (total_secs / 60) % 60;
                let hours = (total_secs / 3600) % 24;
                let days = total_secs / 86400;

                if days > 0 {
                    return format!("{}d {}h {}m", days, hours, mins);
                } else if hours > 0 {
                    return format!("{}h {}m", hours, mins);
                } else {
                    return format!("{}m {}s", mins, secs);
                }
            }
        }
    }
    "Unknown".to_string()
}

fn get_kernel_version() -> String {
    if let Ok(content) = fs::read_to_string("/proc/version") {
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() >= 3 {
            return parts[2].to_string();
        }
    }
    "Linux".to_string()
}

fn main() {
    let os_name = "Elfux OS 1.0 (x86_64)";
    let kernel = get_kernel_version();
    let uptime = get_uptime();
    let cpu = get_cpu_model();
    let gpu = get_gpu_info();
    let memory = get_memory_info();
    let disk = get_disk_info();

    let info_lines = vec![
        format!("\x1b[1mroot@elfux\x1b[0m"),
        format!("----------"),
        format!("\x1b[36;1mOS:\x1b[0m     {}", os_name),
        format!("\x1b[36;1mKernel:\x1b[0m {}", kernel),
        format!("\x1b[36;1mUptime:\x1b[0m {}", uptime),
        format!("\x1b[36;1mCPU:\x1b[0m    {}", cpu),
        format!("\x1b[36;1mGPU:\x1b[0m    {}", gpu),
        format!("\x1b[36;1mMemory:\x1b[0m {}", memory),
        format!("\x1b[36;1mDisk:\x1b[0m   {}", disk),
    ];

    let ascii_lines: Vec<&str> = ASCII_LOGO.lines().filter(|l| !l.is_empty()).collect();

    // Dynamically calculate maximum ASCII line character length
    let max_art_width = ascii_lines
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(30);

    let padding = max_art_width + 4; // Add 4 spaces of spacing between art and info text
    let max_lines = std::cmp::max(ascii_lines.len(), info_lines.len());

    println!();
    for i in 0..max_lines {
        let art_part = ascii_lines.get(i).unwrap_or(&"");
        let info_part = info_lines.get(i).map(|s| s.as_str()).unwrap_or("");

        // Use dynamic padding width so art and text align cleanly
        println!(
            "\x1b[36;1m{:width$}\x1b[0m {}",
            art_part,
            info_part,
            width = padding
        );
    }
    println!();
}