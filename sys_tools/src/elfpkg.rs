use flate2::read::GzDecoder;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::Duration;
use tar::Archive;
use xz2::read::XzDecoder;
use zstd::stream::read::Decoder as ZstdDecoder;

const PKG_DB_DIR: &str = "/var/lib/elfpkg/installed";
const CACHE_DIR: &str = "/var/cache/elfpkg";
const LOCAL_REPO_PATH: &str = "/etc/elfux_package_repo.json";

const REPO_URL: &str = "https://github.com/benIoannidis/Elfux/releases/download/Package-Repo/elfux_package_repo.json";

#[derive(Deserialize, Debug)]
struct PackageMeta {
    version: String,
    url: String,
    description: String,
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    strip_components: Option<usize>,
    #[serde(default)]
    binary_links: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
struct RepoIndex {
    packages: HashMap<String, PackageMeta>,
}

fn install_root() -> &'static Path {
    if Path::new("/mnt").is_dir() {
        Path::new("/mnt")
    } else {
        Path::new("/")
    }
}

fn rooted_path(path: &str) -> std::path::PathBuf {
    let relative = path.trim_start_matches('/');
    install_root().join(relative)
}

fn expose_runtime_file(path: &Path, binary_links: &HashMap<String, String>) {
    let relative = match path.strip_prefix(install_root()) {
        Ok(relative) => relative,
        Err(_) => return,
    };

    if let Some(link_target) = binary_links.get(&relative.to_string_lossy().to_string()) {
        let link_path = Path::new(link_target);
        if !link_path.exists() {
            if let Some(parent) = link_path.parent() {
                let _ = fs::create_dir_all(parent);
            }

            let _ = std::os::unix::fs::symlink(path, link_path);
        }

        return;
    }

    if !relative.starts_with("bin/") && !relative.starts_with("lib64/") {
        return;
    }

    let link_path = Path::new("/").join(relative);
    if link_path.exists() {
        return;
    }

    if let Some(parent) = link_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let _ = std::os::unix::fs::symlink(path, link_path);
}

fn fetch_repo_index() -> Result<RepoIndex, String> {
    println!("[ELFPKG] ==> Syncing repo index...");
    let repo_file = env::var("ELFPKG_REPO_FILE").unwrap_or_else(|_| LOCAL_REPO_PATH.to_string());
    if Path::new(&repo_file).exists() {
        let file = File::open(&repo_file)
            .map_err(|e| format!("Failed to open local repo index: {}", e))?;
        return serde_json::from_reader(file)
            .map_err(|e| format!("Invalid local repo index JSON: {}", e));
    }

    let repo_url = env::var("ELFPKG_REPO_URL").unwrap_or_else(|_| REPO_URL.to_string());
    let response = ureq::get(&repo_url)
        .timeout(Duration::from_secs(30))
        .call()
        .map_err(|e| format!("Failed to reach repo mirror: {}", e))?;

    let repo: RepoIndex = response
        .into_json()
        .map_err(|e| format!("Invalid repo index JSON: {}", e))?;

    Ok(repo)
}

fn cache_file_name(pkg_name: &str, url: &str) -> String {
    url.rsplit('/')
        .next()
        .and_then(|name| name.split('?').next())
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string())
        .unwrap_or_else(|| format!("{}.tar.gz", pkg_name))
}

fn download_file(url: &str, dest_path: &Path) -> Result<(), String> {
    println!("[ELFPKG] Downloading: {}", url);
    let response = ureq::get(url)
        .timeout(Duration::from_secs(90))
        .call()
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    let mut reader = response.into_reader();
    let mut out_file = File::create(dest_path)
        .map_err(|e| format!("Failed to create cache file: {}", e))?;
    io::copy(&mut reader, &mut out_file)
        .map_err(|e| format!("Failed to save package file: {}", e))?;

    Ok(())
}

fn install_tar_archive<R: Read>(
    reader: R,
    pkg_name: &str,
    strip_components: usize,
    binary_links: &HashMap<String, String>,
) -> io::Result<()> {
    println!("[ELFPKG] ==> Unpacking and installing '{}'...", pkg_name);

    let mut archive = Archive::new(reader);
    let root_path = install_root();
    let db_path = root_path.join("var/lib/elfpkg/installed");
    fs::create_dir_all(&db_path)?;

    let manifest_path = db_path.join(format!("{}.list", pkg_name));
    let mut manifest = File::create(&manifest_path)?;

    for entry in archive.entries()? {
        let mut entry = entry?;
        let orig_path = entry.path()?.to_path_buf();
        let components: Vec<_> = orig_path.components().skip(strip_components).collect();
        if components.is_empty() {
            continue;
        }

        let stripped_path: std::path::PathBuf = components.iter().collect();
        let target_dest = root_path.join(&stripped_path);

        if let Some(parent) = target_dest.parent() {
            fs::create_dir_all(parent)?;
        }

        let header = entry.header();
        if header.entry_type().is_dir() {
            fs::create_dir_all(&target_dest)?;
            continue;
        }

        if header.entry_type().is_symlink() && target_dest.exists() {
            continue;
        }

        if header.entry_type().is_hard_link() {
            if let Some(link_name) = entry.link_name()? {
                let link_components: Vec<_> = link_name.components().skip(strip_components).collect();
                if !link_components.is_empty() {
                    let stripped_link_path: std::path::PathBuf = link_components.iter().collect();
                    let link_source = root_path.join(stripped_link_path);

                    if link_source.exists() {
                        fs::copy(&link_source, &target_dest)?;
                        expose_runtime_file(&target_dest, binary_links);
                    }
                }
            }

            continue;
        }

        println!("    => Installing: /{}", stripped_path.display());
        writeln!(manifest, "/{}", stripped_path.display())?;

        entry.unpack(&target_dest)?;
        expose_runtime_file(&target_dest, binary_links);
    }

    println!("[ELFPKG] ==> '{}' installed successfully.", pkg_name);
    Ok(())
}

fn install_binary(mut source: File, pkg_name: &str, binary_links: &HashMap<String, String>) -> io::Result<()> {
    println!("[ELFPKG] ==> Installing binary '{}'...", pkg_name);

    let root_path = install_root();
    let db_path = root_path.join("var/lib/elfpkg/installed");
    fs::create_dir_all(&db_path)?;

    let install_path = binary_links
        .values()
        .next()
        .map(|path| path.as_str())
        .unwrap_or_else(|| "/bin/unknown");
    let target_dest = rooted_path(install_path);

    if let Some(parent) = target_dest.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut dest = File::create(&target_dest)?;
    io::copy(&mut source, &mut dest)?;
    fs::set_permissions(&target_dest, fs::Permissions::from_mode(0o755))?;

    let manifest_path = db_path.join(format!("{}.list", pkg_name));
    let mut manifest = File::create(&manifest_path)?;
    writeln!(manifest, "{}", install_path)?;

    println!("[ELFPKG] ==> '{}' installed successfully.", pkg_name);
    Ok(())
}

fn install_package_archive(
    archive_path: &Path,
    pkg_name: &str,
    format: Option<&str>,
    strip_components: usize,
    binary_links: &HashMap<String, String>,
) -> io::Result<()> {
    if format == Some("meta") {
        return install_meta_package(pkg_name);
    }

    let file = File::open(archive_path)?;
    match format {
        Some("binary") => install_binary(file, pkg_name, binary_links),
        Some("tar.xz") => install_tar_archive(XzDecoder::new(file), pkg_name, strip_components, binary_links),
        Some("tar.zst") | Some("pkg.tar.zst") => {
            let decoder = ZstdDecoder::new(file)?;
            install_tar_archive(decoder, pkg_name, strip_components, binary_links)
        }
        Some("tar.gz") | Some("tgz") | None => install_tar_archive(GzDecoder::new(file), pkg_name, strip_components, binary_links),
        Some(other) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported package format '{}'", other),
        )),
    }
}

fn install_meta_package(pkg_name: &str) -> io::Result<()> {
    println!("[ELFPKG] ==> Marking metapackage '{}' as installed...", pkg_name);

    let db_path = install_root().join("var/lib/elfpkg/installed");
    fs::create_dir_all(&db_path)?;

    let manifest_path = db_path.join(format!("{}.list", pkg_name));
    File::create(&manifest_path)?;

    println!("[ELFPKG] ==> '{}' installed successfully.", pkg_name);
    Ok(())
}

fn get_cache_dir() -> std::path::PathBuf {
    rooted_path(CACHE_DIR)
}

fn is_installed(pkg_name: &str) -> bool {
    rooted_path(PKG_DB_DIR)
        .join(format!("{}.list", pkg_name))
        .exists()
}

fn repair_installed_links(pkg_name: &str, binary_links: &HashMap<String, String>) -> io::Result<()> {
    let manifest_path = rooted_path(PKG_DB_DIR).join(format!("{}.list", pkg_name));
    if !manifest_path.exists() {
        return Ok(());
    }

    let file = File::open(&manifest_path)?;
    for line in BufReader::new(file).lines() {
        let file_path_str = line?;
        let path = rooted_path(&file_path_str);
        expose_runtime_file(&path, binary_links);
    }

    Ok(())
}

fn package_dependencies(pkg_name: &str, pkg_meta: &PackageMeta) -> Vec<String> {
    if !pkg_meta.dependencies.is_empty() {
        return pkg_meta.dependencies.clone();
    }

    match pkg_name {
        "neovim" => vec!["glibc".to_string(), "gcc-libs".to_string()],
        _ => Vec::new(),
    }
}

fn confirm_dependency_install(pkg_name: &str, dependencies: &[String]) -> io::Result<bool> {
    println!("[ELFPKG] ==> '{}' requires:", pkg_name);
    for dependency in dependencies {
        println!("    => {}", dependency);
    }

    if env::var("ELFPKG_ASSUME_YES").is_ok() {
        println!("[ELFPKG] ==> Installing missing dependencies automatically.");
        return Ok(true);
    }

    print!("[ELFPKG] ==> Install missing dependencies? [Y/n] ");
    io::stdout().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    let answer = answer.trim().to_ascii_lowercase();

    Ok(answer.is_empty() || answer == "y" || answer == "yes")
}

fn install_repo_package(
    pkg_name: &str,
    repo: &RepoIndex,
    installing: &mut HashSet<String>,
) -> Result<(), String> {
    if !installing.insert(pkg_name.to_string()) {
        return Err(format!("Dependency cycle detected while installing '{}'", pkg_name));
    }

    let result = (|| {
        let pkg_meta = repo
            .packages
            .get(pkg_name)
            .ok_or_else(|| format!("'{}' was not found in the repository.", pkg_name))?;

        let missing_dependencies: Vec<String> = package_dependencies(pkg_name, pkg_meta)
            .iter()
            .filter(|dependency| !is_installed(dependency))
            .cloned()
            .collect();

        if !missing_dependencies.is_empty() {
            let install_dependencies = confirm_dependency_install(pkg_name, &missing_dependencies)
                .map_err(|e| format!("Failed to read dependency prompt: {}", e))?;

            if !install_dependencies {
                return Err(format!("Installation cancelled; '{}' dependencies were not installed.", pkg_name));
            }

            for dependency in &missing_dependencies {
                if !repo.packages.contains_key(dependency) {
                    return Err(format!(
                        "'{}' requires '{}', but that dependency is not in the repository index.",
                        pkg_name, dependency
                    ));
                }

                install_repo_package(dependency, repo, installing)?;
            }
        }

        if is_installed(pkg_name) {
            repair_installed_links(pkg_name, &pkg_meta.binary_links)
                .map_err(|e| format!("Failed to repair package links: {}", e))?;
            println!("[ELFPKG] ==> '{}' is already installed.", pkg_name);
            return Ok(());
        }

        println!(
            "[ELFPKG] Found '{}' ver.{} ({})",
            pkg_name, pkg_meta.version, pkg_meta.description
        );

        let cache_dir = get_cache_dir();
        fs::create_dir_all(&cache_dir).ok();

        if pkg_meta.format.as_deref() == Some("meta") {
            return install_meta_package(pkg_name).map_err(|e| format!("Installation failed: {}", e));
        }

        let cached_archive = cache_dir.join(cache_file_name(pkg_name, &pkg_meta.url));

        download_file(&pkg_meta.url, &cached_archive)?;
        install_package_archive(
            &cached_archive,
            pkg_name,
            pkg_meta.format.as_deref(),
            pkg_meta.strip_components.unwrap_or(1),
            &pkg_meta.binary_links,
        )
        .map_err(|e| format!("Installation failed: {}", e))
    })();

    installing.remove(pkg_name);
    result
}

fn handle_install(target: &str) {
    let target_path = Path::new(target);

    if target_path.exists() && target.ends_with(".tar.gz") {
        let pkg_name = target_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("package")
            .trim_end_matches(".tar.gz");
        let binary_links = HashMap::new();

        if let Err(e) = install_package_archive(target_path, pkg_name, None, 1, &binary_links) {
            eprintln!("[ERROR] ==> Local installation failed: {}", e);
            std::process::exit(1);
        }
        return;
    }

    let repo = match fetch_repo_index() {
        Ok(index) => index,
        Err(e) => {
            eprintln!("[ERROR] ==> {}", e);
            std::process::exit(1);
        }
    };

    let mut installing = HashSet::new();
    if let Err(e) = install_repo_package(target, &repo, &mut installing) {
        eprintln!("[ERROR] ==> {}", e);
        std::process::exit(1);
    }
}

fn uninstall_package(pkg_name: &str) -> io::Result<()> {
    println!("[ELFPKG] ==> Uninstalling package: {}", pkg_name);

    let db_path = rooted_path(PKG_DB_DIR);
    let manifest_path = db_path.join(format!("{}.list", pkg_name));

    if !manifest_path.exists() {
        eprintln!("[ERROR] ==> Package '{}' is not installed.", pkg_name);
        return Ok(());
    }

    let file = File::open(&manifest_path)?;
    let reader = BufReader::new(file);
    let mut removed_count = 0;

    for line in reader.lines() {
        if let Ok(file_path_str) = line {
            let path = rooted_path(&file_path_str);

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
    let db_path = rooted_path(PKG_DB_DIR);
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
    println!("  elfpkg -i, --install <pkg_name | archive.tar.gz>    |   Install a package");
    println!("  elfpkg -ui, --remove <pkg_name>                     |   Uninstall a package");
    println!("  elfpkg -l, --list                                   |   List installed packages");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "-i" | "-install" | "--install" => {
            if args.len() < 3 {
                eprintln!("[ERROR] ==> Please specify a package or .tar.gz file to install.");
                return;
            }
            handle_install(&args[2]);
        }
        "-ui" | "-remove" | "--remove" => {
            if args.len() < 3 {
                eprintln!("[ERROR] ==> Please specify a package to uninstall.");
                return;
            }
            if let Err(e) = uninstall_package(&args[2]) {
                eprintln!("[ERROR] ==> Failed to remove package: {}", e);
                std::process::exit(1);
            }
        }
        "-l" | "-list" | "--list" => {
            if let Err(e) = list_installed() {
                eprintln!("[ERROR] ==> Failed to list packages: {}", e);
                std::process::exit(1);
            }
        }
        _ => print_usage(),
    }
}
