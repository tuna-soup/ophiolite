#!/usr/bin/env bash
set -euo pipefail

ROOT_URL="${ROOT_URL:-https://tgs-opendata-poseidon.s3.us-west-2.amazonaws.com}"
PREFIX="${PREFIX:-far_stack.mdio}"
OUTPUT_ROOT="${1:-/Users/sc/Downloads/SubsurfaceData/poseidon/${PREFIX}}"

download() {
  local relative_path="$1"
  local target_path="${OUTPUT_ROOT}/${relative_path}"
  mkdir -p "$(dirname "${target_path}")"
  echo "==> ${relative_path}"
  curl -fL --retry 3 --retry-delay 2 -o "${target_path}" "${ROOT_URL}/${PREFIX}/${relative_path}"
}

# Core MDIO metadata.
download ".zattrs"
download ".zgroup"
download ".zmetadata"

# Full axis arrays are tiny and useful for adapter validation.
download "inline/.zarray"
download "inline/.zattrs"
download "inline/0"
download "crossline/.zarray"
download "crossline/.zattrs"
download "crossline/0"
download "time/.zarray"
download "time/.zattrs"
download "time/0"

# Coordinate grids are small enough to include for spatial descriptor fitting.
download "cdp-x/.zarray"
download "cdp-x/.zattrs"
download "cdp-x/0/0"
download "cdp-y/.zarray"
download "cdp-y/.zattrs"
download "cdp-y/0/0"

# Trace mask + headers for a small 2x2 chunk neighborhood.
download "trace_mask/.zarray"
download "trace_mask/.zattrs"
download "trace_mask/0/0"
download "trace_mask/0/1"
download "trace_mask/1/0"
download "trace_mask/1/1"

download "headers/.zarray"
download "headers/.zattrs"
download "headers/0/0"
download "headers/0/1"
download "headers/1/0"
download "headers/1/1"

# Eight real seismic chunks around the early live region.
download "seismic/.zarray"
download "seismic/.zattrs"
download "seismic/0/1/1"
download "seismic/0/1/2"
download "seismic/0/2/1"
download "seismic/0/2/2"
download "seismic/1/1/1"
download "seismic/1/1/2"
download "seismic/1/2/1"
download "seismic/1/2/2"

echo
echo "Working set downloaded under: ${OUTPUT_ROOT}"
du -sh "${OUTPUT_ROOT}"
