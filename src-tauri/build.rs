use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn copy_dir(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn sync_wechat_ocr_resource() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let repo_root = manifest_dir.parent().unwrap_or(&manifest_dir);
    let src = repo_root
        .join("scripts")
        .join("ocr_host")
        .join("bin")
        .join("Release")
        .join("net8.0");
    if !src.join("wcocr.exe").exists() || !src.join("wco_data").join("WeChatOCR.exe").exists() {
        println!("cargo:warning=WeChatOCR resource not found; run dotnet build scripts/ocr_host/ClipNest.OcrHost.csproj -c Release");
        return;
    }

    let dst = manifest_dir.join("resources").join("ocr_host");
    let _ = fs::remove_dir_all(&dst);
    if let Err(err) = copy_dir(&src, &dst) {
        println!("cargo:warning=failed to sync WeChatOCR resource: {err}");
    }
}

fn main() {
    sync_wechat_ocr_resource();
    tauri_build::build()
}
