//! First-run setup wizard — automatic hardware probing and model download.
//!
//! When OSWispa launches for the first time (no model found), this module
//! runs a short local device probe, chooses a sensible default model, and
//! downloads it automatically. Manual selection is still available via an
//! environment override for power users.

use crate::models::{self, ModelInfo, AVAILABLE_MODELS};
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::hint::black_box;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};
use tracing::{debug, info};

// ---------------------------------------------------------------------------
// Hardware detection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GpuType {
    Nvidia,
    Amd,
    AppleSilicon,
    CpuOnly,
}

impl std::fmt::Display for GpuType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuType::Nvidia => write!(f, "NVIDIA GPU"),
            GpuType::Amd => write!(f, "AMD GPU"),
            GpuType::AppleSilicon => write!(f, "Apple Silicon"),
            GpuType::CpuOnly => write!(f, "CPU-only machine"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeAcceleration {
    Cuda,
    HipBlas,
    Metal,
    CpuOnly,
}

impl std::fmt::Display for RuntimeAcceleration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeAcceleration::Cuda => write!(f, "CUDA build enabled"),
            RuntimeAcceleration::HipBlas => write!(f, "ROCm/HIPBLAS build enabled"),
            RuntimeAcceleration::Metal => write!(f, "Metal build enabled"),
            RuntimeAcceleration::CpuOnly => write!(f, "CPU-only build"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CpuTier {
    Basic,
    Good,
    Fast,
    Workstation,
}

impl std::fmt::Display for CpuTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CpuTier::Basic => write!(f, "Basic"),
            CpuTier::Good => write!(f, "Good"),
            CpuTier::Fast => write!(f, "Fast"),
            CpuTier::Workstation => write!(f, "Workstation"),
        }
    }
}

#[derive(Debug, Clone)]
struct HardwareProfile {
    gpu: GpuType,
    acceleration: RuntimeAcceleration,
    /// Available VRAM in megabytes, if detectable.
    vram_mb: Option<u64>,
    /// Total system memory in megabytes, if detectable.
    system_memory_mb: Option<u64>,
    logical_cpus: usize,
    cpu_probe: Duration,
    cpu_tier: CpuTier,
}

fn detect_hardware() -> HardwareProfile {
    let logical_cpus = num_cpus::get().max(1);
    let cpu_probe = benchmark_cpu();
    let system_memory_mb = detect_system_memory_mb();
    let cpu_tier = classify_cpu_tier(logical_cpus, cpu_probe);

    // macOS: treat Apple Silicon separately because unified memory matters more
    // than discrete VRAM, and Metal support depends on the compiled build.
    if cfg!(target_os = "macos") {
        let is_arm = std::env::consts::ARCH == "aarch64";
        let gpu = if is_arm {
            GpuType::AppleSilicon
        } else {
            GpuType::CpuOnly
        };
        let acceleration = if is_arm && cfg!(feature = "gpu-metal") {
            RuntimeAcceleration::Metal
        } else {
            RuntimeAcceleration::CpuOnly
        };

        return HardwareProfile {
            gpu,
            acceleration,
            vram_mb: None,
            system_memory_mb,
            logical_cpus,
            cpu_probe,
            cpu_tier,
        };
    }

    // Linux / other: detect available GPU memory and only mark acceleration
    // active when the current binary was actually built with that backend.
    if let Some((total, free)) = detect_nvidia_vram() {
        return HardwareProfile {
            gpu: GpuType::Nvidia,
            acceleration: if cfg!(feature = "gpu-cuda") {
                RuntimeAcceleration::Cuda
            } else {
                RuntimeAcceleration::CpuOnly
            },
            vram_mb: Some(free.max(total / 2)),
            system_memory_mb,
            logical_cpus,
            cpu_probe,
            cpu_tier,
        };
    }

    if let Some(available) = detect_amd_vram_sysfs() {
        return HardwareProfile {
            gpu: GpuType::Amd,
            acceleration: if cfg!(feature = "gpu-hipblas") {
                RuntimeAcceleration::HipBlas
            } else {
                RuntimeAcceleration::CpuOnly
            },
            vram_mb: Some(available),
            system_memory_mb,
            logical_cpus,
            cpu_probe,
            cpu_tier,
        };
    }

    if let Some(available) = detect_amd_vram_rocm_smi() {
        return HardwareProfile {
            gpu: GpuType::Amd,
            acceleration: if cfg!(feature = "gpu-hipblas") {
                RuntimeAcceleration::HipBlas
            } else {
                RuntimeAcceleration::CpuOnly
            },
            vram_mb: Some(available),
            system_memory_mb,
            logical_cpus,
            cpu_probe,
            cpu_tier,
        };
    }

    HardwareProfile {
        gpu: GpuType::CpuOnly,
        acceleration: RuntimeAcceleration::CpuOnly,
        vram_mb: None,
        system_memory_mb,
        logical_cpus,
        cpu_probe,
        cpu_tier,
    }
}

/// A tiny CPU probe run once at first launch to estimate responsiveness.
fn benchmark_cpu() -> Duration {
    let iterations = 18_000_000_u64;
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    let start = Instant::now();

    for i in 0..iterations {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407 ^ i);
        state ^= state >> 17;
    }

    black_box(state);
    start.elapsed()
}

fn classify_cpu_tier(logical_cpus: usize, cpu_probe: Duration) -> CpuTier {
    let ms = cpu_probe.as_secs_f64() * 1000.0;

    if logical_cpus >= 10 && ms <= 90.0 {
        CpuTier::Workstation
    } else if logical_cpus >= 8 && ms <= 130.0 {
        CpuTier::Fast
    } else if logical_cpus >= 4 && ms <= 220.0 {
        CpuTier::Good
    } else {
        CpuTier::Basic
    }
}

fn detect_system_memory_mb() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
        for line in meminfo.lines() {
            if let Some(rest) = line.strip_prefix("MemTotal:") {
                let kb = rest.split_whitespace().next()?.parse::<u64>().ok()?;
                return Some(kb / 1024);
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let output = Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let bytes = String::from_utf8(output.stdout)
            .ok()?
            .trim()
            .parse::<u64>()
            .ok()?;
        return Some(bytes / (1024 * 1024));
    }

    None
}

/// nvidia-smi query -> (total_mb, free_mb)
fn detect_nvidia_vram() -> Option<(u64, u64)> {
    let output = Command::new("nvidia-smi")
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

/// AMD sysfs -> available MB (total - used)
fn detect_amd_vram_sysfs() -> Option<u64> {
    let card_dirs = ["/sys/class/drm/card1/device", "/sys/class/drm/card0/device"];

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

/// rocm-smi fallback -> available MB
fn detect_amd_vram_rocm_smi() -> Option<u64> {
    let output = Command::new("rocm-smi")
        .args(["--showmeminfo", "vram"])
        .output()
        .ok()?;

    let stdout = String::from_utf8(output.stdout).ok()?;
    let mut total: u64 = 0;
    let mut used: u64 = 0;

    for line in stdout.lines() {
        if line.contains("Total Memory") {
            if let Some(val) = line.split(':').next_back() {
                total = val.trim().parse().unwrap_or(0);
            }
        }
        if line.contains("Total Used") {
            if let Some(val) = line.split(':').next_back() {
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
    reason: String,
}

fn find_model(filename: &str) -> &'static ModelInfo {
    AVAILABLE_MODELS
        .iter()
        .find(|model| model.filename == filename)
        .unwrap_or_else(|| panic!("Missing built-in model definition for {}", filename))
}

fn wizard_models() -> Vec<&'static ModelInfo> {
    [
        "ggml-tiny.en.bin",
        "ggml-base.en.bin",
        "ggml-small.en.bin",
        "ggml-medium.en.bin",
        "ggml-distil-large-v3.bin",
        "ggml-large.bin",
    ]
    .iter()
    .map(|filename| find_model(filename))
    .collect()
}

fn choose_recommended_model(hw: &HardwareProfile) -> (&'static ModelInfo, String) {
    let total_mem = hw.system_memory_mb.unwrap_or(0);
    let vram = hw.vram_mb.unwrap_or(0);

    match hw.acceleration {
        RuntimeAcceleration::Cuda | RuntimeAcceleration::HipBlas => {
            if vram >= 10_000 && total_mem >= 16_000 {
                (
                    find_model("ggml-distil-large-v3.bin"),
                    "Fastest high-accuracy option while leaving healthy GPU headroom.".to_string(),
                )
            } else if vram >= 5_000 {
                (
                    find_model("ggml-medium.en.bin"),
                    "Good quality without overcommitting GPU memory.".to_string(),
                )
            } else if vram >= 2_000 {
                (
                    find_model("ggml-small.en.bin"),
                    "Keeps local dictation responsive on a smaller GPU budget.".to_string(),
                )
            } else {
                (
                    find_model("ggml-base.en.bin"),
                    "Safest choice because GPU memory headroom is tight.".to_string(),
                )
            }
        }
        RuntimeAcceleration::Metal => {
            if total_mem >= 24_000 && matches!(hw.cpu_tier, CpuTier::Fast | CpuTier::Workstation) {
                (
                    find_model("ggml-distil-large-v3.bin"),
                    "Strong Apple Silicon with ample unified memory can handle the fastest high-accuracy model.".to_string(),
                )
            } else if total_mem >= 12_000 && !matches!(hw.cpu_tier, CpuTier::Basic) {
                (
                    find_model("ggml-medium.en.bin"),
                    "Best balance for this Mac's unified memory and CPU speed.".to_string(),
                )
            } else {
                (
                    find_model("ggml-small.en.bin"),
                    "Keeps Apple Silicon responsive on lighter memory budgets.".to_string(),
                )
            }
        }
        RuntimeAcceleration::CpuOnly => {
            if total_mem >= 16_000
                && hw.logical_cpus >= 8
                && matches!(hw.cpu_tier, CpuTier::Fast | CpuTier::Workstation)
            {
                (
                    find_model("ggml-small.en.bin"),
                    "Fast enough for a CPU-only workstation without making dictation feel heavy."
                        .to_string(),
                )
            } else {
                (
                    find_model("ggml-base.en.bin"),
                    "Prioritises quick local response on a CPU-only build.".to_string(),
                )
            }
        }
    }
}

fn recommendation_reason(model: &ModelInfo, hw: &HardwareProfile, selected: &ModelInfo) -> String {
    if model.filename == selected.filename {
        return choose_recommended_model(hw).1;
    }

    match model.filename {
        "ggml-tiny.en.bin" => "Fastest option, but accuracy is noticeably weaker.".to_string(),
        "ggml-base.en.bin" if hw.acceleration != RuntimeAcceleration::CpuOnly => {
            "Very safe fallback if you want lower memory use than the default.".to_string()
        }
        "ggml-small.en.bin" if hw.acceleration == RuntimeAcceleration::CpuOnly => {
            "Viable on faster CPUs, but not the safest default for every machine.".to_string()
        }
        "ggml-medium.en.bin" if hw.acceleration == RuntimeAcceleration::CpuOnly => {
            "Usually too heavy for a CPU-only first-time default.".to_string()
        }
        "ggml-distil-large-v3.bin" if hw.acceleration == RuntimeAcceleration::CpuOnly => {
            "Not recommended as the first model on a CPU-only build.".to_string()
        }
        "ggml-distil-large-v3.bin" => {
            "Excellent accuracy, but only worth it when the machine has real headroom.".to_string()
        }
        "ggml-large.bin" => {
            "Highest accuracy, but not the best default if you care about speed.".to_string()
        }
        _ => String::new(),
    }
}

fn recommend_models(hw: &HardwareProfile) -> Vec<ModelRecommendation> {
    let (selected, selected_reason) = choose_recommended_model(hw);

    wizard_models()
        .into_iter()
        .map(|model| ModelRecommendation {
            model,
            recommended: model.filename == selected.filename,
            reason: if model.filename == selected.filename {
                selected_reason.clone()
            } else {
                recommendation_reason(model, hw, selected)
            },
        })
        .collect()
}

fn manual_selection_requested() -> bool {
    matches!(
        std::env::var("OSWISPA_SETUP_MANUAL")
            .ok()
            .as_deref()
            .map(|value| value.to_ascii_lowercase()),
        Some(value) if matches!(value.as_str(), "1" | "true" | "yes" | "manual")
    )
}

// ---------------------------------------------------------------------------
// Setup wizard entry point
// ---------------------------------------------------------------------------

/// Run the first-time setup flow.
///
/// Returns the path to the downloaded model on success.
pub fn run_first_time_setup() -> Result<PathBuf> {
    eprintln!();
    eprintln!("  ┌─────────────────────────────────────┐");
    eprintln!("  │     OSWispa — First-Time Setup      │");
    eprintln!("  └─────────────────────────────────────┘");
    eprintln!();

    let hw = detect_hardware();
    let recommendations = recommend_models(&hw);
    let default_idx = recommendations
        .iter()
        .position(|r| r.recommended)
        .unwrap_or(0);
    let selected = if manual_selection_requested() {
        prompt_for_model(&recommendations, default_idx)?
    } else {
        default_idx
    };
    let chosen = &recommendations[selected];

    eprintln!("  Device probe:");
    eprintln!("    Machine: {}", hw.gpu);
    eprintln!("    Runtime: {}", hw.acceleration);
    if let Some(total_mem) = hw.system_memory_mb {
        eprintln!("    Memory: {:.1} GB", total_mem as f64 / 1024.0);
    }
    if let Some(vram) = hw.vram_mb {
        eprintln!("    Free GPU memory: {:.1} GB", vram as f64 / 1024.0);
    }
    eprintln!(
        "    CPU: {} logical cores ({})",
        hw.logical_cpus, hw.cpu_tier
    );
    eprintln!(
        "    Quick test: {:.0} ms",
        hw.cpu_probe.as_secs_f64() * 1000.0
    );
    eprintln!();

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

    eprintln!();
    if manual_selection_requested() {
        eprintln!("  Manual mode enabled via OSWISPA_SETUP_MANUAL.");
    } else {
        eprintln!("  Auto-selected: {}", chosen.model.name);
        if !chosen.reason.is_empty() {
            eprintln!("        {}", chosen.reason);
        }
        eprintln!("  Set OSWISPA_SETUP_MANUAL=1 if you want to override this choice.");
    }
    eprintln!();
    eprintln!("  Downloading {}...", chosen.model.name);
    eprintln!();

    let path = download_model_with_progress(chosen.model)?;
    info!("First-time setup: downloaded model to {:?}", path);

    eprintln!();
    eprintln!("  ✓ Model installed successfully.");
    eprintln!("  ✓ OSWispa is ready — starting up...");
    eprintln!();

    Ok(path)
}

fn prompt_for_model(recommendations: &[ModelRecommendation], default_idx: usize) -> Result<usize> {
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

    Ok(choice)
}

/// Download a model file with a terminal progress bar.
fn download_model_with_progress(model: &ModelInfo) -> Result<PathBuf> {
    if model.filename.contains('/')
        || model.filename.contains('\\')
        || model.filename.contains("..")
    {
        anyhow::bail!("Invalid model filename: {}", model.filename);
    }

    let models_dir = models::get_models_dir();
    std::fs::create_dir_all(&models_dir)?;

    let dest_path = models_dir.join(model.filename);
    let temp_path = models_dir.join(format!("{}.downloading", model.filename));

    if temp_path.exists() {
        let _ = std::fs::remove_file(&temp_path);
    }

    let client = reqwest::blocking::Client::builder().timeout(None).build()?;

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
    let mut buf = [0u8; 32768];

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
    std::fs::rename(&temp_path, &dest_path)?;

    Ok(dest_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(
        acceleration: RuntimeAcceleration,
        system_memory_mb: Option<u64>,
        vram_mb: Option<u64>,
        logical_cpus: usize,
        cpu_tier: CpuTier,
    ) -> HardwareProfile {
        HardwareProfile {
            gpu: GpuType::CpuOnly,
            acceleration,
            vram_mb,
            system_memory_mb,
            logical_cpus,
            cpu_probe: Duration::from_millis(80),
            cpu_tier,
        }
    }

    #[test]
    fn cpu_only_default_stays_lightweight() {
        let selected = choose_recommended_model(&profile(
            RuntimeAcceleration::CpuOnly,
            Some(8_192),
            None,
            4,
            CpuTier::Good,
        ));

        assert_eq!(selected.0.filename, "ggml-base.en.bin");
    }

    #[test]
    fn cpu_only_workstation_can_take_small_model() {
        let selected = choose_recommended_model(&profile(
            RuntimeAcceleration::CpuOnly,
            Some(32_768),
            None,
            12,
            CpuTier::Workstation,
        ));

        assert_eq!(selected.0.filename, "ggml-small.en.bin");
    }

    #[test]
    fn metal_mac_uses_medium_before_distil() {
        let selected = choose_recommended_model(&profile(
            RuntimeAcceleration::Metal,
            Some(16_384),
            None,
            8,
            CpuTier::Fast,
        ));

        assert_eq!(selected.0.filename, "ggml-medium.en.bin");
    }

    #[test]
    fn big_gpu_prefers_distil_large() {
        let selected = choose_recommended_model(&profile(
            RuntimeAcceleration::Cuda,
            Some(32_768),
            Some(12_288),
            16,
            CpuTier::Workstation,
        ));

        assert_eq!(selected.0.filename, "ggml-distil-large-v3.bin");
    }

    #[test]
    fn wizard_model_list_includes_distil() {
        let models = wizard_models();
        assert!(models
            .iter()
            .any(|model| model.filename == "ggml-distil-large-v3.bin"));
    }
}
