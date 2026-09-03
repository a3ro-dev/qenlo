#!/usr/bin/env bash
set -Eeuo pipefail

mkdir -p /usr/share/vulkan/icd.d
nvidia_glx=$(ldconfig -p | awk '/libGLX_nvidia.so.0/{print $NF; exit}')
if [[ -n "$nvidia_glx" ]]; then
  printf '{"file_format_version":"1.0.0","ICD":{"library_path":"%s","api_version":"1.3.280"}}\n' \
    "$nvidia_glx" > /usr/share/vulkan/icd.d/nvidia_icd.json
else
  printf 'NVIDIA GLX library not found\n' >&2
  exit 1
fi
vulkaninfo --summary
