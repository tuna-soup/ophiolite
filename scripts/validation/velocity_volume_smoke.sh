#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  cat <<'EOF' >&2
usage: scripts/validation/velocity_volume_smoke.sh <input.sgy|input.segy> <output.tbvol> [interval|average|rms] [time|depth]

Runs a real-data smoke path for a velocity SEG-Y volume:
1. inspect the raw input
2. ingest it into tbvol
3. describe the resulting velocity volume
4. open the resulting dataset summary

Set DELETE_INPUT_ON_SUCCESS=1 to pass --delete-input-on-success to the ingest step.
Set OVERWRITE_EXISTING=1 to pass --overwrite-existing to the ingest step.
Set VERTICAL_UNIT, VERTICAL_START, and/or VERTICAL_STEP to override vertical-axis metadata.
Set INLINE_BYTE, CROSSLINE_BYTE, THIRD_AXIS_BYTE and matching *_TYPE values (`i16` or `i32`)
to pass SEG-Y geometry overrides into the velocity ingest path.
EOF
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
INPUT_PATH="$1"
OUTPUT_PATH="$2"
VELOCITY_KIND="${3:-interval}"
VERTICAL_DOMAIN="${4:-time}"

if [[ ! -f "$INPUT_PATH" ]]; then
  echo "input file does not exist: $INPUT_PATH" >&2
  exit 2
fi

INSPECT_JSON="${OUTPUT_PATH%.tbvol}.inspect.json"
INGEST_JSON="${OUTPUT_PATH%.tbvol}.ingest.json"
DESCRIBE_JSON="${OUTPUT_PATH%.tbvol}.describe.json"
OPEN_JSON="${OUTPUT_PATH%.tbvol}.open.json"

ingest_args=(
  run
  -p
  traceboost-app
  --
  ingest-velocity-volume
  "$INPUT_PATH"
  "$OUTPUT_PATH"
  --velocity-kind
  "$VELOCITY_KIND"
  --vertical-domain
  "$VERTICAL_DOMAIN"
)

if [[ "${OVERWRITE_EXISTING:-0}" == "1" ]]; then
  ingest_args+=(--overwrite-existing)
fi

if [[ "${DELETE_INPUT_ON_SUCCESS:-0}" == "1" ]]; then
  ingest_args+=(--delete-input-on-success)
fi

if [[ -n "${VERTICAL_UNIT:-}" ]]; then
  ingest_args+=(--vertical-unit "$VERTICAL_UNIT")
fi

if [[ -n "${VERTICAL_START:-}" ]]; then
  ingest_args+=(--vertical-start "$VERTICAL_START")
fi

if [[ -n "${VERTICAL_STEP:-}" ]]; then
  ingest_args+=(--vertical-step "$VERTICAL_STEP")
fi

if [[ -n "${INLINE_BYTE:-}" ]]; then
  ingest_args+=(--inline-byte "$INLINE_BYTE" --inline-type "${INLINE_TYPE:-i32}")
fi

if [[ -n "${CROSSLINE_BYTE:-}" ]]; then
  ingest_args+=(--crossline-byte "$CROSSLINE_BYTE" --crossline-type "${CROSSLINE_TYPE:-i32}")
fi

if [[ -n "${THIRD_AXIS_BYTE:-}" ]]; then
  ingest_args+=(--third-axis-byte "$THIRD_AXIS_BYTE" --third-axis-type "${THIRD_AXIS_TYPE:-i32}")
fi

echo "==> inspect raw SEG-Y"
cargo run -p traceboost-app -- inspect "$INPUT_PATH" | tee "$INSPECT_JSON"

echo "==> ingest velocity volume"
cargo "${ingest_args[@]}" | tee "$INGEST_JSON"

describe_args=(
  run
  -p
  traceboost-app
  --
  describe-velocity-volume
  "$OUTPUT_PATH"
  --velocity-kind
  "$VELOCITY_KIND"
  --vertical-domain
  "$VERTICAL_DOMAIN"
)

if [[ -n "${VERTICAL_UNIT:-}" ]]; then
  describe_args+=(--vertical-unit "$VERTICAL_UNIT")
fi

if [[ -n "${VERTICAL_START:-}" ]]; then
  describe_args+=(--vertical-start "$VERTICAL_START")
fi

if [[ -n "${VERTICAL_STEP:-}" ]]; then
  describe_args+=(--vertical-step "$VERTICAL_STEP")
fi

echo "==> describe velocity volume"
cargo "${describe_args[@]}" | tee "$DESCRIBE_JSON"

echo "==> open dataset summary"
cargo run -p traceboost-app -- open-dataset "$OUTPUT_PATH" | tee "$OPEN_JSON"

echo "==> wrote"
printf '%s\n' "$INSPECT_JSON" "$INGEST_JSON" "$DESCRIBE_JSON" "$OPEN_JSON"
