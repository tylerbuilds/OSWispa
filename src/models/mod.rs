//! Whisper model management
//!
//! Handles downloading, listing, and switching between Whisper models.

use anyhow::Result;
use std::io::Read;
use std::path::{Path, PathBuf};
use tracing::info;

const MIN_EXPECTED_MODEL_PERCENT: u64 = 80;
const MIN_CUSTOM_MODEL_BYTES: u64 = 1024 * 1024;

/// Available Whisper models with metadata
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: &'static str,
    pub filename: &'static str,
    pub size_mb: u32,
    pub url: &'static str,
    pub description: &'static str,
}

/// Lightweight model profile derived from model file size.
#[derive(Debug, Clone)]
pub struct ModelBenchmark {
    pub size_mb: f64,
    pub speed_tier: &'static str,
    pub accuracy_tier: &'static str,
}

/// All available models from Hugging Face
pub const AVAILABLE_MODELS: &[ModelInfo] = &[
    ModelInfo {
        name: "Tiny (English)",
        filename: "ggml-tiny.en.bin",
        size_mb: 75,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
        description: "Fastest, basic accuracy",
    },
    ModelInfo {
        name: "Tiny (Multilingual)",
        filename: "ggml-tiny.bin",
        size_mb: 75,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
        description: "Fastest, supports all languages",
    },
    ModelInfo {
        name: "Base (English)",
        filename: "ggml-base.en.bin",
        size_mb: 142,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
        description: "Fast, good accuracy (recommended)",
    },
    ModelInfo {
        name: "Base (Multilingual)",
        filename: "ggml-base.bin",
        size_mb: 142,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
        description: "Fast, good accuracy, all languages",
    },
    ModelInfo {
        name: "Small (English)",
        filename: "ggml-small.en.bin",
        size_mb: 466,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",
        description: "Medium speed, better accuracy",
    },
    ModelInfo {
        name: "Small (Multilingual)",
        filename: "ggml-small.bin",
        size_mb: 466,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        description: "Medium speed, better accuracy, all languages",
    },
    ModelInfo {
        name: "Medium (English)",
        filename: "ggml-medium.en.bin",
        size_mb: 1500,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin",
        description: "Slow, high accuracy",
    },
    ModelInfo {
        name: "Distil Large v3",
        filename: "ggml-distil-large-v3.bin",
        size_mb: 1530,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-distil-large-v3.bin",
        description: "Best speed/accuracy for high-end English dictation",
    },
    ModelInfo {
        name: "Large v3",
        filename: "ggml-large-v3.bin",
        size_mb: 3000,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
        description: "Slowest, best accuracy, all languages",
    },
];

/// Get the models directory path
pub fn get_models_dir() -> PathBuf {
    crate::get_data_dir().join("models")
}

/// Check if a specific model is installed
pub fn is_model_installed(model: &ModelInfo) -> bool {
    validate_downloaded_model(model, &get_models_dir().join(model.filename)).is_ok()
}

/// Get full path for a model
pub fn get_model_path(model: &ModelInfo) -> PathBuf {
    get_models_dir().join(model.filename)
}

fn validate_model_filename(model: &ModelInfo) -> Result<()> {
    if model.filename.contains('/')
        || model.filename.contains('\\')
        || model.filename.contains("..")
    {
        anyhow::bail!("Invalid model filename: {}", model.filename);
    }
    Ok(())
}

fn minimum_expected_model_bytes(model: &ModelInfo) -> u64 {
    model.size_mb as u64 * 1024 * 1024 * MIN_EXPECTED_MODEL_PERCENT / 100
}

fn validate_downloaded_model(model: &ModelInfo, path: &Path) -> Result<()> {
    let size = std::fs::metadata(path)?.len();
    let minimum = minimum_expected_model_bytes(model);
    if size < minimum {
        anyhow::bail!(
            "Downloaded model is incomplete: got {} bytes, expected at least {}",
            size,
            minimum
        );
    }

    validate_model_magic(path)
}

fn validate_model_magic(path: &Path) -> Result<()> {
    let mut magic = [0_u8; 4];
    std::fs::File::open(path)?.read_exact(&mut magic)?;
    if &magic != b"lmgg" && &magic != b"GGUF" {
        anyhow::bail!("File is not a recognised GGML/GGUF model");
    }
    Ok(())
}

/// Validate a configured or imported model before attempting to load it.
pub fn validate_model_path(path: &Path) -> Result<()> {
    if !path.is_file() {
        anyhow::bail!("Model path is not a regular file: {:?}", path);
    }

    if let Some(model) = AVAILABLE_MODELS.iter().find(|model| {
        path.file_name()
            .map(|name| name == std::ffi::OsStr::new(model.filename))
            .unwrap_or(false)
    }) {
        return validate_downloaded_model(model, path);
    }

    let size = std::fs::metadata(path)?.len();
    if size < MIN_CUSTOM_MODEL_BYTES {
        anyhow::bail!("Model is incomplete: got {} bytes", size);
    }
    validate_model_magic(path)
}

fn install_validated_model(temp_path: &Path, dest_path: &Path) -> Result<()> {
    // Windows cannot rename over an existing file. If an older release left an
    // invalid final file behind, remove only that invalid file after the new
    // payload has passed validation.
    if dest_path.exists() && validate_model_path(dest_path).is_err() {
        std::fs::remove_file(dest_path)?;
    }
    std::fs::rename(temp_path, dest_path)?;
    Ok(())
}

fn is_supported_model_path(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    matches!(ext, "bin" | "gguf")
}

/// Import a local model file into OSWispa's managed model directory.
pub fn import_model_from_path(source: &Path) -> Result<PathBuf> {
    if !source.exists() {
        anyhow::bail!("Model file does not exist: {:?}", source);
    }
    if !source.is_file() {
        anyhow::bail!("Model path is not a file: {:?}", source);
    }
    if !is_supported_model_path(source) {
        anyhow::bail!("Unsupported model extension. Use .bin or .gguf");
    }
    validate_model_path(source)?;

    let file_name = source
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Model file name is invalid"))?;

    let models_dir = get_models_dir();
    std::fs::create_dir_all(&models_dir)?;
    let dest = models_dir.join(file_name);

    let source_canon = std::fs::canonicalize(source).ok();
    let dest_canon = std::fs::canonicalize(&dest).ok();
    if source_canon.is_some() && source_canon == dest_canon {
        return Ok(dest);
    }

    let temp_dest = dest.with_extension("importing");
    std::fs::copy(source, &temp_dest)?;
    std::fs::rename(&temp_dest, &dest)?;

    info!("Imported model {:?} to {:?}", source, dest);
    Ok(dest)
}

/// List custom local models that are not in the curated built-in list.
pub fn list_custom_models() -> Vec<PathBuf> {
    let models_dir = get_models_dir();
    let mut models = Vec::new();

    let built_in_names: std::collections::HashSet<&str> =
        AVAILABLE_MODELS.iter().map(|m| m.filename).collect();

    if let Ok(entries) = std::fs::read_dir(models_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() || !is_supported_model_path(&path) {
                continue;
            }

            let file_name = path
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or_default();
            if built_in_names.contains(file_name) {
                continue;
            }

            models.push(path);
        }
    }

    models.sort();
    models
}

/// Estimate model speed/accuracy tiers based on on-disk size.
pub fn estimate_model_benchmark(path: &Path) -> Result<ModelBenchmark> {
    let metadata = std::fs::metadata(path)?;
    let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);

    let (speed_tier, accuracy_tier) = if size_mb <= 100.0 {
        ("Very Fast", "Basic")
    } else if size_mb <= 300.0 {
        ("Fast", "Good")
    } else if size_mb <= 700.0 {
        ("Balanced", "Better")
    } else if size_mb <= 1800.0 {
        ("Moderate", "High")
    } else {
        ("Slow", "Highest")
    };

    Ok(ModelBenchmark {
        size_mb,
        speed_tier,
        accuracy_tier,
    })
}

/// Download a model with progress callback (async, requires GUI feature for tokio runtime)
#[cfg(feature = "gui")]
pub async fn download_model<F>(model: &ModelInfo, progress_callback: F) -> Result<PathBuf>
where
    F: Fn(u64, u64) + Send + 'static,
{
    use futures_util::StreamExt;
    use std::io::Write;

    validate_model_filename(model)?;

    let models_dir = get_models_dir();
    std::fs::create_dir_all(&models_dir)?;

    let dest_path = models_dir.join(model.filename);
    let temp_path = models_dir.join(format!("{}.downloading", model.filename));
    let _ = std::fs::remove_file(&temp_path);

    info!("Downloading {} to {:?}", model.name, dest_path);

    let client = reqwest::Client::new();
    let response = client.get(model.url).send().await?.error_for_status()?;

    if response
        .content_length()
        .map(|length| length < minimum_expected_model_bytes(model))
        .unwrap_or(false)
    {
        anyhow::bail!("Model server returned an incomplete payload");
    }

    let total_size = response
        .content_length()
        .unwrap_or(model.size_mb as u64 * 1024 * 1024);
    let mut downloaded: u64 = 0;

    let result: Result<()> = async {
        let mut file = std::fs::File::create(&temp_path)?;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            file.write_all(&chunk)?;
            downloaded += chunk.len() as u64;
            progress_callback(downloaded, total_size);
        }

        file.sync_all()?;
        drop(file);
        validate_downloaded_model(model, &temp_path)?;
        install_validated_model(&temp_path, &dest_path)?;
        Ok(())
    }
    .await;

    if let Err(err) = result {
        let _ = std::fs::remove_file(&temp_path);
        return Err(err);
    }

    info!("Download complete: {:?}", dest_path);
    Ok(dest_path)
}

/// Blocking model download for CLI use (no tokio/GUI required).
///
/// Downloads the model file with a simple progress callback and atomic rename.
pub fn download_model_blocking<F>(model: &ModelInfo, progress_callback: F) -> Result<PathBuf>
where
    F: Fn(u64, u64),
{
    use std::io::Write;

    validate_model_filename(model)?;

    let models_dir = get_models_dir();
    std::fs::create_dir_all(&models_dir)?;

    let dest_path = models_dir.join(model.filename);
    let temp_path = models_dir.join(format!("{}.downloading", model.filename));
    let _ = std::fs::remove_file(&temp_path);

    info!("Downloading {} to {:?}", model.name, dest_path);

    let client = reqwest::blocking::Client::builder().timeout(None).build()?;

    let response = client.get(model.url).send()?.error_for_status()?;

    if response
        .content_length()
        .map(|length| length < minimum_expected_model_bytes(model))
        .unwrap_or(false)
    {
        anyhow::bail!("Model server returned an incomplete payload");
    }

    let total_size = response
        .content_length()
        .unwrap_or(model.size_mb as u64 * 1024 * 1024);

    let result = (|| -> Result<()> {
        let mut file = std::fs::File::create(&temp_path)?;
        let mut reader = response;
        let mut downloaded: u64 = 0;
        let mut buf = [0u8; 32768];

        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n])?;
            downloaded += n as u64;
            progress_callback(downloaded, total_size);
        }

        file.sync_all()?;
        drop(file);
        validate_downloaded_model(model, &temp_path)?;
        install_validated_model(&temp_path, &dest_path)?;
        Ok(())
    })();

    if let Err(err) = result {
        let _ = std::fs::remove_file(&temp_path);
        return Err(err);
    }

    info!("Download complete: {:?}", dest_path);
    Ok(dest_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Seek, SeekFrom, Write};

    fn fixture_model() -> ModelInfo {
        ModelInfo {
            name: "Fixture",
            filename: "fixture.bin",
            size_mb: 1,
            url: "https://example.invalid/fixture.bin",
            description: "test fixture",
        }
    }

    #[test]
    fn model_filename_rejects_path_traversal() {
        let mut model = fixture_model();
        model.filename = "../fixture.bin";
        assert!(validate_model_filename(&model).is_err());
    }

    #[test]
    fn downloaded_model_requires_expected_size_and_magic() {
        let model = fixture_model();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(model.filename);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(b"lmgg").unwrap();
        file.seek(SeekFrom::Start(minimum_expected_model_bytes(&model) - 1))
            .unwrap();
        file.write_all(&[0]).unwrap();
        drop(file);

        assert!(validate_downloaded_model(&model, &path).is_ok());

        std::fs::write(&path, b"<html>not a model</html>").unwrap();
        assert!(validate_downloaded_model(&model, &path).is_err());
    }

    #[test]
    fn custom_model_path_requires_minimum_size_and_magic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("custom.gguf");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(b"GGUF").unwrap();
        file.seek(SeekFrom::Start(MIN_CUSTOM_MODEL_BYTES - 1))
            .unwrap();
        file.write_all(&[0]).unwrap();
        drop(file);

        assert!(validate_model_path(&path).is_ok());

        let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.write_all(b"html").unwrap();
        drop(file);
        assert!(validate_model_path(&path).is_err());
    }
}
