//! Whisper model management
//!
//! Handles downloading, listing, and switching between Whisper models.

use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::info;

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
        name: "Large",
        filename: "ggml-large.bin",
        size_mb: 3000,
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
        description: "Slowest, best accuracy, all languages",

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

fn is_supported_model_path(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or_default();
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
