use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
	println!("cargo:rerun-if-changed=_assets");

	let out_dir = std::env::var("OUT_DIR")?;
	let out_zip = PathBuf::from(out_dir).join("assets.zip");

	let file = File::create(&out_zip)?;
	let mut zip = ZipWriter::new(BufWriter::new(file));
	let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Zstd);

	let assets_dir = Path::new("_assets");
	if assets_dir.exists() {
		for entry in WalkDir::new(assets_dir) {
			let entry = entry?;
			let path = entry.path();

			if path.is_file() {
				if let Some(file_name) = path.file_name()
					&& file_name == ".DS_Store"
				{
					continue;
				}

				let rel_path = path.strip_prefix(assets_dir)?;
				let rel_path_str = rel_path
					.components()
					.map(|c| c.as_os_str().to_string_lossy())
					.collect::<Vec<_>>()
					.join("/");

				zip.start_file(rel_path_str, options)?;
				let mut f = File::open(path)?;
				let mut buffer = Vec::new();
				f.read_to_end(&mut buffer)?;
				zip.write_all(&buffer)?;
			}
		}
	}

	zip.finish()?;

	println!("cargo:rustc-env=ASSETS_ZIP={}", out_zip.display());

	Ok(())
}
