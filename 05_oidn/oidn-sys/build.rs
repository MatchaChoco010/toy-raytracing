use anyhow::Result;
use std::env;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

const WINDOWS_URL: &str =
    "https://github.com/OpenImageDenoise/oidn/releases/download/v2.1.0/oidn-2.1.0.x64.windows.zip";
const LIB_FILENAMES: [&str; 2] = ["OpenImageDenoise_core.lib", "OpenImageDenoise.lib"];

fn download_windows(out_dir: &Path) -> Result<()> {
    if LIB_FILENAMES
        .iter()
        .all(|filename| out_dir.join(filename).exists())
    {
        return Ok(());
    }

    let filename = WINDOWS_URL.split("/").last().unwrap();
    let archive = out_dir.join(&filename);

    if !archive.exists() {
        let response: reqwest::blocking::Response = reqwest::blocking::get(WINDOWS_URL)?;
        let bytes = response.bytes()?;
        let mut out = File::create(&archive)?;
        io::copy(&mut bytes.as_ref(), &mut out)?;
    }

    let f = File::open(&archive)?;
    let mut archive = ZipArchive::new(f)?;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        let path = file.mangled_name();
        if path.extension().is_some() && path.extension().unwrap() == "lib" {
            let name = path.file_name().unwrap();
            let out_path = out_dir.join(name).into_os_string().into_string().unwrap();
            let mut out_file = File::create(&out_path)?;
            io::copy(&mut file, &mut out_file)?;
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    if cfg!(target_os = "windows") {
        download_windows(&out_dir)?;
        println!("cargo:rustc-link-search=native={}", out_dir.display());
        println!("cargo:rustc-link-lib=OpenImageDenoise");
        Ok(())
    } else {
        println!("cargo:rustc-link-lib=OpenImageDenoise");
        Ok(())
    }
}
