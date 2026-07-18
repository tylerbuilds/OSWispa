#!/bin/bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source "$ROOT_DIR/scripts/install_helpers.sh"

fail() {
    echo "install helper test failed: $1" >&2
    exit 1
}

assert_eq() {
    local expected="$1"
    local actual="$2"
    local message="$3"
    [ "$actual" = "$expected" ] || fail "$message (expected '$expected', got '$actual')"
}

targets=$(oswispa_rocm_targets_from_stream <<'EOF'
  Name:                    gfx1036
  Name:                    gfx1100
  Name:                    gfx1036
  Name:                    gfx90a
EOF
)
assert_eq "gfx1036;gfx1100;gfx90a" "$targets" "ROCm targets should be unique and include letter suffixes"

gpu=$(oswispa_largest_rocm_gpu_from_stream <<'EOF'
GPU[0]          : VRAM Total Memory (B): 536870912
GPU[0]          : VRAM Total Used Memory (B): 1000
GPU[1]          : VRAM Total Memory (B): 21458059264
GPU[1]          : VRAM Total Used Memory (B): 2000
EOF
)
assert_eq "1" "$gpu" "the discrete GPU with the most VRAM should be selected"

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT
model="$tmp_dir/model.bin"
printf 'lmgg' > "$model"
truncate -s 1024 "$model"

oswispa_validate_model_file "$model" 1024 || fail "valid GGML model fixture was rejected"
if oswispa_validate_model_file "$model" 2048; then
    fail "undersized model fixture was accepted"
fi

printf 'html' > "$model"
truncate -s 1024 "$model"
if oswispa_validate_model_file "$model" 1024; then
    fail "non-model payload was accepted"
fi

echo "install helper tests passed"
