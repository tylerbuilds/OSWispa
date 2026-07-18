//! Shared GPU inventory helpers.

use std::collections::BTreeMap;
use std::process::Command;
use tracing::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VramInfo {
    index: usize,
    total_bytes: u64,
    used_bytes: u64,
}

impl VramInfo {
    fn available_bytes(self) -> u64 {
        self.total_bytes.saturating_sub(self.used_bytes)
    }
}

fn parse_rocm_smi_vram(output: &str, preferred_index: Option<usize>) -> Option<VramInfo> {
    let mut devices: BTreeMap<usize, (Option<u64>, Option<u64>)> = BTreeMap::new();

    for line in output.lines() {
        let Some((_, after_prefix)) = line.split_once("GPU[") else {
            continue;
        };
        let Some((index, _)) = after_prefix.split_once(']') else {
            continue;
        };
        let Ok(index) = index.parse::<usize>() else {
            continue;
        };
        let Some(value) = line
            .rsplit(':')
            .next()
            .and_then(|value| value.trim().parse::<u64>().ok())
        else {
            continue;
        };
        let entry = devices.entry(index).or_default();

        if line.contains("VRAM Total Memory") {
            entry.0 = Some(value);
        } else if line.contains("VRAM Total Used Memory") {
            entry.1 = Some(value);
        }
    }

    let complete = devices.into_iter().filter_map(|(index, (total, used))| {
        Some(VramInfo {
            index,
            total_bytes: total?,
            used_bytes: used?,
        })
    });

    if let Some(preferred) = preferred_index {
        complete
            .into_iter()
            .find(|device| device.index == preferred)
    } else {
        complete.max_by_key(|device| device.total_bytes)
    }
}

pub fn rocm_visible_device_index() -> Option<usize> {
    std::env::var("ROCR_VISIBLE_DEVICES")
        .ok()?
        .split(',')
        .next()?
        .trim()
        .parse()
        .ok()
}

pub fn detect_rocm_smi_available_bytes(preferred_index: Option<usize>) -> Option<u64> {
    let output = Command::new("rocm-smi")
        .args(["--showmeminfo", "vram"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let info = parse_rocm_smi_vram(&stdout, preferred_index)?;
    debug!(
        "rocm-smi GPU {}: total={}, used={}, available={}",
        info.index,
        info.total_bytes,
        info.used_bytes,
        info.available_bytes()
    );
    Some(info.available_bytes())
}

pub fn detect_amd_sysfs_available_bytes() -> Option<u64> {
    let entries = std::fs::read_dir("/sys/class/drm").ok()?;
    let mut best: Option<(u64, u64, String)> = None;

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(index) = name.strip_prefix("card") else {
            continue;
        };
        if index.is_empty() || !index.chars().all(|character| character.is_ascii_digit()) {
            continue;
        }

        let device = entry.path().join("device");
        let total = std::fs::read_to_string(device.join("mem_info_vram_total"))
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok());
        let used = std::fs::read_to_string(device.join("mem_info_vram_used"))
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok());
        let (Some(total), Some(used)) = (total, used) else {
            continue;
        };

        if best
            .as_ref()
            .map(|(best_total, _, _)| total > *best_total)
            .unwrap_or(true)
        {
            best = Some((total, used, name));
        }
    }

    let (total, used, name) = best?;
    let available = total.saturating_sub(used);
    debug!(
        "AMD sysfs {}: total={}, used={}, available={}",
        name, total, used, available
    );
    Some(available)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MULTI_GPU: &str = r#"
GPU[0]          : VRAM Total Memory (B): 536870912
GPU[0]          : VRAM Total Used Memory (B): 1000
GPU[1]          : VRAM Total Memory (B): 21458059264
GPU[1]          : VRAM Total Used Memory (B): 2000
"#;

    #[test]
    fn rocm_parser_selects_largest_vram_by_default() {
        let info = parse_rocm_smi_vram(MULTI_GPU, None).unwrap();
        assert_eq!(info.index, 1);
        assert_eq!(info.available_bytes(), 21_458_057_264);
    }

    #[test]
    fn rocm_parser_honours_selected_device() {
        let info = parse_rocm_smi_vram(MULTI_GPU, Some(0)).unwrap();
        assert_eq!(info.index, 0);
        assert_eq!(info.available_bytes(), 536_869_912);
    }

    #[test]
    fn rocm_parser_rejects_incomplete_inventory() {
        assert!(parse_rocm_smi_vram("GPU[0]: VRAM Total Memory (B): 1024", None).is_none());
    }
}
