//! Offline recognition of the control words (Vosk) and the speech models in data\models.

pub mod vosk;

use std::path::{Path, PathBuf};

use futures_util::StreamExt;

use crate::error::{AppError, Result};
use crate::paths::Paths;

const MODELS_URL: &str = "https://alphacephei.com/vosk/models";

/// libvosk.dll shipped with the program.
pub fn library(paths: &Paths) -> PathBuf {
    let name = if cfg!(windows) { "libvosk.dll" } else { "libvosk.so" };
    paths.bundled(&format!("vosk/{name}"))
}

pub fn model_present(paths: &Paths, name: &str) -> bool {
    paths.models().join(name).join("am").is_dir() || paths.models().join(name).join("conf").is_dir()
}

/// Downloads and unpacks a model ("vosk-model-small-pl-0.22", about 50 MB) into data\models.
/// `progress` gets the fraction done.
pub async fn download_model(paths: &Paths, name: &str, progress: &(dyn Fn(f64) + Sync)) -> Result<PathBuf> {
    if name.is_empty() || name.contains(['/', '\\', ':']) || name.contains("..") {
        return Err(AppError::msg(format!("Niepoprawna nazwa modelu: {name}")));
    }
    let dir = paths.models();
    std::fs::create_dir_all(&dir)?;
    let zip_path = dir.join(format!("{name}.zip.part"));
    let resp = crate::runtime::http(1800)?
        .get(format!("{MODELS_URL}/{name}.zip"))
        .send()
        .await?
        .error_for_status()?;
    let total = resp.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(&zip_path).await?;
    let mut done = 0u64;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
        done += chunk.len() as u64;
        if total > 0 {
            progress(done as f64 / total as f64);
        }
    }
    drop(file);
    let result = unpack(&zip_path, &dir);
    let _ = std::fs::remove_file(&zip_path);
    result?;
    Ok(dir.join(name))
}

fn unpack(zip_path: &Path, dir: &Path) -> Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    archive.extract(dir)?;
    Ok(())
}
