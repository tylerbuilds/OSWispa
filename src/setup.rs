//! First-run setup wizard — interactive terminal-based model selection and download.
//!
//! When OSWispa launches for the first time (no model found), this module
//! guides the user through hardware detection, model recommendation, and
//! automatic download — all from the terminal, no GUI required.

use crate::models::{self, ModelInfo, AVAILABLE_MODELS};
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use tracing::{debug, info};

// ---------------------------------------------------------------------------
// Hardware detection
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum GpuType {
    Nvidia,
    Amd,
    AppleMetal,
    CpuOnly,
}

impl std::fmt::Display for GpuType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuType::Nvidia => write!(f, "NVIDIA (CUDA)"),
            GpuType::Amd => write!(f, "AMD (ROCm)"),
            GpuType::AppleMetal => write!(f, "Apple Silicon (Metal)"),
            GpuType::CpuOnly => write!(f, "CPU only"),
        }
    }
}

#[derive(Debug)]
struct HardwareProfile {
    gpu: GpuType,
    /// Available VRAM in megabytes, if detectable.
    vram_mb: Option<u64>,
}

fn detect_hardware() -> HardwareProfile {
    // macOS: check for Apple Silicon
    if cfg!(target_os = "macos") {
        let is_arm = std::env::consts::ARCH == "aarch64";
        return HardwareProfile {
            gpu: if is_arm {
                GpuType::AppleMetal
            } else {
                GpuType::CpuOnly
            },
            // macOS unified memory — can't meaningfully separate VRAM
            vram_mb: if is_arm { Some(8192) } else { None },
        };
    }

    // Linux / other: try nvidia-smi first, then AMD sysfs, then rocm-smi
    if let Some((total, free)) = detect_nvidia_vram() {
        return HardwareProfile {
            gpu: GpuType::Nvidia,
            vram_mb: Some(free.max(total / 2)), // use free, floor at half total
        };
    }

    if let Some(available) = detect_amd_vram_sysfs() {
        return HardwareProfile {
            gpu: GpuType::Amd,
            vram_mb: Some(available),
        };
    }

    if let Some(available) = detect_amd_vram_rocm_smi() {
        return HardwareProfile {
            gpu: GpuType::Amd,
            vram_mb: Some(available),
        };
    }

    HardwareProfile {
        gpu: GpuType::CpuOnly,
        vram_mb: None,
    }
}

/// nvidia-smi query → (total_mb, free_mb)
fn detect_nvidia_vram() -> Option<(u64, u64)> {
    let output = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=memory.total,memory.free",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;

    let stdout = String::from_utf8(output.stdout).ok()?;
    let line = stdout.lines().next()?.trim();
    let mut parts = line.split(',');
    let total: u64 = parts.next()?.trim().parse().ok()?;
    let free: u64 = parts.next()?.trim().parse().ok()?;
    debug!("nvidia-smi: total={}MB, free={}MB", total, free);
    Some((total, free))
}

/// AMD sysfs → available MB (total − used)
fn detect_amd_vram_sysfs() -> Option<u64> {
    let card_dirs = [
        "/sys/class/drm/card1/device",
        "/sys/class/drm/card0/device",
    ];

    for dir in &card_dirs {
        let total_path = format!("{}/mem_info_vram_total", dir);
        let used_path = format!("{}/mem_info_vram_used", dir);

        if let (Ok(total_str), Ok(used_str)) = (
            std::fs::read_to_string(&total_path),
            std::fs::read_to_string(&used_path),
        ) {
            if let (Ok(total), Ok(used)) = (
                total_str.trim().parse::<u64>(),
                used_str.trim().parse::<u64>(),
            ) {
                // Skip integrated GPUs (< 1 GB)
                if total > 1_073_741_824 {
                    let available_mb = total.saturating_sub(used) / (1024 * 1024);
                    debug!("AMD sysfs: available={}MB", available_mb);
                    return Some(available_mb);
                }
            }
        }
    }

    None
}

/// rocm-smi fallback → available MB
fn detect_amd_vram_rocm_smi() -> Option<u64> {
    let output = std::process::Command::new("rocm-smi")
        .args(["--showmeminfo", "vram"])
        .output()
        .ok()?;

    let stdout = String::from_utf8(output.stdout).ok()?;
    let mut total: u64 = 0;
    let mut used: u64 = 0;

    for line in stdout.lines() {
        if line.contains("Total Memory") {
            if let Some(val) = line.split(':').last() {
                total = val.trim().parse().unwrap_or(0);
            }
        }
        if line.contains("Total Used") {
            if let Some(val) = line.split(':').last() {
                used = val.trim().parse().unwrap_or(0);
            }
        }
    }

    if total > 0 {
        let available_mb = total.saturating_sub(used) / (1024 * 1024);
        debug!("rocm-smi: available={}MB", available_mb);
        Some(available_mb)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Model recommendation
// ---------------------------------------------------------------------------

struct ModelRecommendation {
    model: &'static ModelInfo,
    recommended: bool,
    reason: &'static str,
}

fn recommend_models(hw: &HardwareProfile) -> Vec<ModelRecommendation> {
    let vram = hw.vram_mb.unwrap_or(0);

    // Determine ideal model size based on VRAM
    let ideal_mb: u32 = match (&hw.gpu, vram) {
        (GpuType::CpuOnly, _) => 142,       // CPU: keep it small
        (_, v) if v >= 6000 => 1500,         // 6+ GB: medium
        (_, v) if v >= 3000 => 466,          // 3+ GB: small
        _ => 142,                            // < 3 GB: base
    };

    // Filter to English models only for the wizard (simpler UX for first-timers)
    let english_models: Vec<&ModelInfo> = AVAILABLE_MODELS
        .iter()
        .filter(|m| m.filename.contains(".en.") || m.filename == "ggml-large.bin")
        .collect();

    english_models
        .into_iter()
        .map(|m| {
            let is_recommended = m.size_mb == ideal_mb
                || (ideal_mb >= 1500 && m.size_mb == 1500)
                || (ideal_mb <= 142 && m.size_mb == 142);

            let reason = if is_recommended {
                match (&hw.gpu, vram) {
                    (GpuType::CpuOnly, _) => "Best for CPU — fast and reliable",
                    (_, v) if v >= 6000 => "Best for your GPU — high accuracy",
                    (_, v) if v >= 3000 => "Good fit for your VRAM",
                    _ => "Best for your hardware",
                }
            } else if m.size_mb as u64 > vram && !matches!(hw.gpu, GpuType::CpuOnly) {
                "May exceed your VRAM"
            } else {
                ""
            };

            ModelRecommendation {
                model: m,
                recommended: is_recommended,
                reason,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Setup wizard entry point
// ---------------------------------------------------------------------------

/// Run the interactive first-time setup wizard.
///
/// Returns the path to the downloaded model on success.
pub fn run_first_time_setup() -> Result<PathBuf> {
    eprintln!();
    eprintln!("  ┌─────────────────────────────────────┐");
    eprintln!("  │     OSWispa — First-Time Setup       │");
    eprintln!("  └─────────────────────────────────────┘");
    eprintln!();

    // 1. Detect hardware
    let hw = detect_hardware();

    eprintln!("  Hardware detected:");
    eprintln!("    GPU:  {}", hw.gpu);
    if let Some(vram) = hw.vram_mb {
        if !matches!(hw.gpu, GpuType::CpuOnly) {
            eprintln!("    VRAM: {} MB available", vram);
        }
    }
    eprintln!();

    // 2. Get recommended models
    let recommendations = recommend_models(&hw);

    eprintln!("  Available models:");
    eprintln!();
    for (i, rec) in recommendations.iter().enumerate() {
        let star = if rec.recommended { " ★" } else { "  " };
        let size_str = if rec.model.size_mb >= 1000 {
            format!("{:.1} GB", rec.model.size_mb as f64 / 1000.0)
        } else {
            format!("{} MB", rec.model.size_mb)
        };

        eprintln!(
            "  {} [{}] {} ({}) — {}",
            star,
            i + 1,
            rec.model.name,
            size_str,
            rec.model.description
        );
        if !rec.reason.is_empty() {
            eprintln!("        {}", rec.reason);
        }
    }

    // Find default (recommended) index
    let default_idx = recommendations
        .iter()
        .position(|r| r.recommended)
        .unwrap_or(0);

    eprintln!();
    eprint!(
        "  Select a model [1-{}, default={}]: ",
        recommendations.len(),
        default_idx + 1
    );
    io::stderr().flush()?;

    let choice = loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            break default_idx;
        }

        match input.parse::<usize>() {
            Ok(n) if n >= 1 && n <= recommendations.len() => break n - 1,
            _ => {
                eprint!(
                    "  Invalid choice. Enter 1-{} (or press Enter for {}): ",
                    recommendations.len(),
                    default_idx + 1
                );
                io::stderr().flush()?;
            }
        }
    };

    let selected = &recommendations[choice];

    eprintln!();
    eprintln!("  Downloading {}...", selected.model.name);
    eprintln!();

    // 3. Download with progress bar
    let path = download_model_with_progress(selected.model)?;

    info!("First-time setup: downloaded model to {:?}", path);

    eprintln!();
    eprintln!("  ✓ Model installed successfully.");
    eprintln!("  ✓ OSWispa is ready — starting up...");
    eprintln!();

    Ok(path)
}

/// Download a model file with a terminal progress bar.
fn download_model_with_progress(model: &ModelInfo) -> Result<PathBuf> {
    // Reject filenames with path separators or parent-directory components
    if model.filename.contains('/') || model.filename.contains('\\') || model.filename.contains("..") {
        anyhow::bail!("Invalid model filename: {}", model.filename);
    }

    let models_dir = models::get_models_dir();
    std::fs::create_dir_all(&models_dir)?;

    let dest_path = models_dir.join(model.filename);
    let temp_path = models_dir.join(format!("{}.downloading", model.filename));

    // Clean up stale temp file from a previous interrupted download
    if temp_path.exists() {
        let _ = std::fs::remove_file(&temp_path);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(None) // large file, no overall timeout
        .build()?;

    let response = client
        .get(model.url)
        .send()
        .map_err(|e| anyhow::anyhow!("Download failed: {}", e))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Download failed: HTTP {} from {}",
            response.status(),
            model.url
        );
    }

    let total_size = response
        .content_length()
        .unwrap_or(model.size_mb as u64 * 1024 * 1024);

    // Set up progress bar
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::with_template(
            "  [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})",
        )
        .unwrap()
        .progress_chars("█▓░"),
    );

    let mut file = std::fs::File::create(&temp_path)?;
    let mut reader = response;
    let mut downloaded: u64 = 0;
    let mut buf = [0u8; 32768]; // 32 KB buffer

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        downloaded += n as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message("done");
    drop(file);

    // Atomic rename to final path
    std::fs::rename(&temp_path, &dest_path)?;

    Ok(dest_path)
}
