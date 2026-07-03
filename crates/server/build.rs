use std::{
    env,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

fn main() -> io::Result<()> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir exists"));
    let asset_dir = manifest_dir.join("web-dist");
    println!("cargo:rerun-if-changed={}", asset_dir.display());

    let mut assets = Vec::new();
    collect_assets(&asset_dir, &asset_dir, &mut assets)?;
    assets.sort_by(|left, right| left.0.cmp(&right.0));

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir exists"));
    let mut output = File::create(out_dir.join("web_assets.rs"))?;
    writeln!(
        output,
        "pub(crate) struct EmbeddedAsset {{ pub(crate) path: &'static str, pub(crate) bytes: &'static [u8] }}"
    )?;
    writeln!(
        output,
        "pub(crate) static WEB_ASSETS: &[EmbeddedAsset] = &["
    )?;
    for (path, absolute) in assets {
        writeln!(
            output,
            "    EmbeddedAsset {{ path: {:?}, bytes: include_bytes!({:?}) }},",
            path,
            absolute.display().to_string()
        )?;
    }
    writeln!(output, "];")?;
    Ok(())
}

fn collect_assets(root: &Path, dir: &Path, assets: &mut Vec<(String, PathBuf)>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_assets(root, &path, assets)?;
            continue;
        }
        if path.is_file() {
            let relative = path
                .strip_prefix(root)
                .expect("asset path is below root")
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            assets.push((relative, path));
        }
    }
    Ok(())
}
