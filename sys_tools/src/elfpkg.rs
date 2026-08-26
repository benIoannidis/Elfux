use flate2::read::GzDecoder;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use tar::Archive;

const PKG_DB_DIR: &str = "/var/lib/elfpkg/installed";

fn install_package(tar_path: &str) -> io::Result<()> {
    println!("[ELFPKG] ==> Installing package from: {}", tar_path);

    let file = File::open(tar_path)?;
    let gz = GzDecoder::new(file);
    let mut archive = Archive::new(gz);

    let file_name = Path::new(tar_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("package.tar.gz");

    let pkg_name = file_name
        .trim_end_matches(".tar.gz")
        .trim_end_matches(".tgz");

    let db_path = Path::new(PKG_DB_DIR);
    fs::create_dir_all(db_path)?;

    let manifest_path = db_path.join(format!("{}.list", pkg_name));
    let mut manifest = File::create(manifest_path)?;

    let root_path = Path::new("/");

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_path_buf();

        println!("==> Installing: /{}", path.display());
        writeln!(manifest, "/{}", path.display())?;

        entry.unpack_in(root_path)?;
    }

    println!("[ELFPKG] ==> Successfully installed '{}'.", pkg_name);
    Ok(())
}

fn uninstall_package(pkg_name: &str) -> io::Result<()> {
    println!("[ELFPKG] ==> Uninstalling package: {}", pkg_name);

    let db_path = Path::new(PKG_DB_DIR);
    let manifest_path = db_path.join(format!("{}.list", pkg_name));

    if !manifest_path.exists() {
        eprintln!("[ERROR] ==> Package '{}' is not installed.", pkg_name);
        return Ok(());
    }

    let file = File::open(&manifest_path)?;
    let reader = BufReader::new(file);
    let mut removed_count = 0;

    //read every file path listed in the manifest and remove it 
    for line in reader.lines() {
        if let Ok(file_path_str) = line {
            let path = Path::new(&file_path_str);

            if path.exists() && !path.is_dir() {
                if let Err(e) = fs::remove_file(path) {
                    eprintln!("    => [WARN] Failed to remove {}: {}", file_path_str, e);
                } else {
                    println!("    => Removed: {}", file_path_str);
                    removed_count += 1;
                }
            }
        }
    }

    fs::remove_file(manifest_path)?;

    println!(
        "[ELFPKG] ==> Successfully uninstalled '{}' ({} files removed).",
        pkg_name, removed_count
    );
    Ok(())
}
fn list_installed() -> io::Result<()> {
    println!("\nInstalled Packages:");
    println!("<===================================================>");
    let db_path = Path::new(PKG_DB_DIR);
    if !db_path.exists() {
        println!("No packages installed yet.");
        return Ok(());
    }

    let mut count = 0;
    for entry in fs::read_dir(db_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("list") {
            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                println!(" ==> {}", name);
                count += 1;
            }
        }
    }
    if count == 0 {
        println!("No packages installed yet.");
    } else {
        println!("{} packages installed.", count);
    }
    println!();
    Ok(())
}

fn print_usage() {
    println!("elfpkg - Custom Package Manager for Elfux OS");
    println!("Usage:");
    println!("  elfpkg -i <archive.tar.gz>      Install a package");
    println!("  elfpkg -ui <pkg_name>           Uninstall a package");
    println!("  elfpkg -l                       List installed packages");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "-i" => {
            if args.len() < 3 {
                eprintln!("[ERROR] ==> Please specify a .tar.gz file to install.");
                return;
            }
            if let Err(e) = install_package(&args[2]) {
                eprintln!("[ERROR] ==> Failed to install package: {}", e);
            }
        }
        "-ui" => {
            if args.len() < 3 {
                eprintln!("[ERROR] ==> Please specify a package to uninstall.");
                return;
            }
            if let Err(e) = uninstall_package(&args[2]) {
                eprintln!("[ERROR] ==> Failed to remove package: {}", e);
            }
        }
        "-l" => {
            if let Err(e) = list_installed() {
                eprintln!("[ERROR] ==> Failed to list packages: {}", e);
            }
        }
        _ => print_usage()
    }
}