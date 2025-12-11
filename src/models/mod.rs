//! Whisper model management
//!
//! Handles downloading, listing, and switching between Whisper models.

use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

/// Available Whisper models with metadata
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: &'static str,
    pub filename: &'static str,
    pub size_mb: u32,
    pub url: &'static str,
    pub description: &'static str,
    pub english_only: bool,
}

/// All available models from Hugging Face
pub const AVAILABLE_MODELS: &[ModelInfo] = &[
    ModelInfo {
        name: "Tiny (English)",
        filename: "ggml-tiny.en.bin",
        size_mb: 75,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin",
        description: "Fastest, basic accuracy",
        english_only: true,
    },
    ModelInfo {
        name: "Tiny (Multilingual)",
        filename: "ggml-tiny.bin",
        size_mb: 75,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
        description: "Fastest, supports all languages",
        english_only: false,
    },
    ModelInfo {
        name: "Base (English)",
        filename: "ggml-base.en.bin",
        size_mb: 142,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin",
        description: "Fast, good accuracy (recommended)",
        english_only: true,
    },
    ModelInfo {
        name: "Base (Multilingual)",
        filename: "ggml-base.bin",
        size_mb: 142,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
        description: "Fast, good accuracy, all languages",
        english_only: false,
    },
    ModelInfo {
        name: "Small (English)",
        filename: "ggml-small.en.bin",
        size_mb: 466,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.en.bin",
        description: "Medium speed, better accuracy",
        english_only: true,
    },
    ModelInfo {
        name: "Small (Multilingual)",
        filename: "ggml-small.bin",
        size_mb: 466,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        description: "Medium speed, better accuracy, all languages",
        english_only: false,
    },
    ModelInfo {
        name: "Medium (English)",
        filename: "ggml-medium.en.bin",
        size_mb: 1500,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.en.bin",
        description: "Slow, high accuracy",
        english_only: true,
    },
    ModelInfo {
        name: "Large",
        filename: "ggml-large.bin",
        size_mb: 3000,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
        description: "Slowest, best accuracy, all languages",
        english_only: false,
    },
];

/// Get the models directory path
pub fn get_models_dir() -> PathBuf {
    crate::get_data_dir().join("models")
}

/// List installed models
pub fn get_installed_models() -> Vec<&'static ModelInfo> {
    let models_dir = get_models_dir();
    AVAILABLE_MODELS
        .iter()
        .filter(|m| models_dir.join(m.filename).exists())
        .collect()
}

/// Check if a specific model is installed
pub fn is_model_installed(model: &ModelInfo) -> bool {
    get_models_dir().join(model.filename).exists()
}

/// Get full path for a model
pub fn get_model_path(model: &ModelInfo) -> PathBuf {
    get_models_dir().join(model.filename)
}

/// Download a model with progress callback
#[cfg(feature = "gui")]
pub async fn download_model<F>(model: &ModelInfo, progress_callback: F) -> Result<PathBuf>
where
    F: Fn(u64, u64) + Send + 'static,
{
    use futures_util::StreamExt;
    use std::io::Write;

    let models_dir = get_models_dir();
    std::fs::create_dir_all(&models_dir)?;

    let dest_path = models_dir.join(model.filename);
    let temp_path = models_dir.join(format!("{}.downloading", model.filename));

    info!("Downloading {} to {:?}", model.name, dest_path);

    let client = reqwest::Client::new();
    let response = client.get(model.url).send().await?;

    let total_size = response.content_length().unwrap_or(model.size_mb as u64 * 1024 * 1024);
    let mut downloaded: u64 = 0;

    let mut file = std::fs::File::create(&temp_path)?;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        progress_callback(downloaded, total_size);
    }

    // Rename temp file to final name
    std::fs::rename(&temp_path, &dest_path)?;

    info!("Download complete: {:?}", dest_path);
    Ok(dest_path)
}

/// Delete a model
pub fn delete_model(model: &ModelInfo) -> Result<()> {
    let path = get_model_path(model);
    if path.exists() {
        std::fs::remove_file(&path)?;
        info!("Deleted model: {}", model.name);
    }
    Ok(())
}
