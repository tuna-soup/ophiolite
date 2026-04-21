#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_DIR="${1:-$ROOT_DIR/target/external-import-acceptance}"
F02_WELLS_ROOT="${F02_WELLS_ROOT:-/Users/sc/Downloads/F02_wells_data}"
HORIZONS_ROOT="${HORIZONS_ROOT:-/Users/sc/Downloads/horizons}"
HORIZON_STORE_PATH="${HORIZON_STORE_PATH:-}"
WELL_PREVIEW_BASENAMES=()

if [[ ! -d "$F02_WELLS_ROOT" && -d "/Users/sc/Downloads/SubsurfaceData/F02_wells_data" ]]; then
  F02_WELLS_ROOT="/Users/sc/Downloads/SubsurfaceData/F02_wells_data"
fi

mkdir -p "$OUTPUT_DIR"

discover_well_folders() {
  find "$F02_WELLS_ROOT" -maxdepth 1 -mindepth 1 -type d | sort | sed -n '1,2p'
}

slugify_folder_name() {
  basename "$1" | tr '[:upper:]' '[:lower:]' | tr -cs 'a-z0-9' '-' | sed 's/^-//; s/-$//'
}

select_well_sources() {
  local folder_path="$1"
  find "$folder_path" -maxdepth 1 -type f \
    \( \
      -iname '*.las' -o \
      -iname '*.asc' -o \
      -iname '*.dlis' -o \
      -iname 'basisgegevens.txt' -o \
      -iname 'deviatie.txt' -o \
      -iname 'lithostratigrafie.txt' \
    \) | sort
}

run_well_source_preview() {
  local folder_path="$1"
  local output_name="$2"
  local selected_sources=()
  while IFS= read -r source_path; do
    selected_sources+=("$source_path")
  done < <(select_well_sources "$folder_path")
  if [[ ${#selected_sources[@]} -eq 0 ]]; then
    echo "No supported sources found in $folder_path" >&2
    return 1
  fi
  echo "Previewing selected well sources in $folder_path"
  cargo run -q -p ophiolite-cli -- preview-well-source-import "$folder_path" "${selected_sources[@]}" \
    > "$OUTPUT_DIR/$output_name.json"
  WELL_PREVIEW_BASENAMES+=("$output_name")
}

while IFS= read -r folder_path; do
  [[ -n "$folder_path" ]] || continue
  run_well_source_preview "$folder_path" "$(slugify_folder_name "$folder_path")-source-preview"
done < <(discover_well_folders)

if [[ ${#WELL_PREVIEW_BASENAMES[@]} -eq 0 ]]; then
  echo "No well folders found under $F02_WELLS_ROOT" >&2
  exit 1
fi

echo "Inspecting horizons in $HORIZONS_ROOT"
cargo run -q -p ophiolite-cli -- inspect-horizon-xyz "$HORIZONS_ROOT"/*.xyz \
  > "$OUTPUT_DIR/horizons-inspect.json"

if [[ -n "$HORIZON_STORE_PATH" ]]; then
  echo "Previewing horizons against survey store $HORIZON_STORE_PATH"
  cargo run -q -p ophiolite-cli -- preview-horizon-source-import "$HORIZON_STORE_PATH" "$HORIZONS_ROOT"/*.xyz \
    > "$OUTPUT_DIR/horizons-source-preview.json"
fi

if command -v jq >/dev/null 2>&1; then
  for preview_basename in "${WELL_PREVIEW_BASENAMES[@]}"; do
    jq '{
      folderName: .parsed.folderName,
      selectedSourceCount: (
        (.suggestedDraft.importPlan.selectedLogSourcePaths // [] | length)
        + (if .suggestedDraft.importPlan.topsMarkers == null then 0 else 1 end)
        + (if .suggestedDraft.importPlan.trajectory == null then 0 else 1 end)
      ),
      logs: {
        count: (.parsed.logs.files | length),
        selected: (.parsed.logs.files | map(select(.defaultSelected)) | length)
      },
      asciiLogs: {
        count: (.parsed.asciiLogs.files | length),
        selected: (.parsed.asciiLogs.files | map(select(.defaultDepthColumn != null and (.defaultValueColumns | length) > 0)) | length)
      },
      tops: {
        committable: .parsed.topsMarkers.committableRowCount,
        total: .parsed.topsMarkers.rowCount
      },
      trajectory: {
        status: .parsed.trajectory.status,
        commitEnabled: .parsed.trajectory.commitEnabled
      },
      unsupported: (.parsed.unsupportedSources | map(.fileName)),
      issueCount: (.parsed.issues | length)
    }' "$OUTPUT_DIR/$preview_basename.json" > "$OUTPUT_DIR/${preview_basename/-preview/-summary}.json"
  done

  jq 'map({
    name,
    parsed_point_count,
    invalid_row_count,
    x_min,
    x_max,
    y_min,
    y_max,
    z_min,
    z_max
  })' "$OUTPUT_DIR/horizons-inspect.json" > "$OUTPUT_DIR/horizons-summary.json"

  if [[ -f "$OUTPUT_DIR/horizons-source-preview.json" ]]; then
    jq '{
      canCommit: .parsed.can_commit,
      transformed: .parsed.transformed,
      issueCount: (.parsed.issues | length),
      fileCount: (.parsed.files | length),
      committableFiles: (.parsed.files | map(select(.can_commit)) | length)
    }' "$OUTPUT_DIR/horizons-source-preview.json" > "$OUTPUT_DIR/horizons-source-summary.json"
  fi
fi

echo "Acceptance artifacts written to $OUTPUT_DIR"
