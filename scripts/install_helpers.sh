#!/bin/bash

# Shared, side-effect-free helpers used by install.sh and its fixture tests.

OSWISPA_BASE_MODEL_MIN_BYTES=$((100 * 1024 * 1024))

oswispa_rocm_targets_from_stream() {
    grep -oE 'gfx[0-9a-f]+' | LC_ALL=C sort -u | paste -sd ';' -
}

oswispa_largest_rocm_gpu_from_stream() {
    sed -nE 's/.*GPU\[([0-9]+)\].*VRAM Total Memory[^:]*:[[:space:]]*([0-9]+).*/\1 \2/p' \
        | sort -k2,2nr -k1,1n \
        | head -n 1 \
        | cut -d ' ' -f 1
}

oswispa_validate_model_file() {
    local file="$1"
    local minimum_bytes="$2"
    local size
    local magic

    [ -f "$file" ] || return 1
    size=$(wc -c < "$file") || return 1
    [ "$size" -ge "$minimum_bytes" ] || return 1

    magic=$(od -An -tx1 -N4 "$file" | tr -d '[:space:]') || return 1
    [ "$magic" = "6c6d6767" ] || [ "$magic" = "47475546" ]
}
