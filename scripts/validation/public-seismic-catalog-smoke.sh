#!/usr/bin/env bash
set -euo pipefail

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required" >&2
  exit 1
fi

if ! command -v rg >/dev/null 2>&1; then
  echo "rg is required" >&2
  exit 1
fi

fetch() {
  curl -L --max-time "${CURL_MAX_TIME:-20}" -s "$@"
}

header() {
  printf '\n== %s ==\n' "$1"
}

extract_json_string() {
  local key="$1"
  rg -o "\"${key}\"[[:space:]]*:[[:space:]]*\"[^\"]+\"" | head -n 1 | sed -E "s/\"${key}\"[[:space:]]*:[[:space:]]*\"([^\"]+)\"/\\1/"
}

header "Public Seismic Catalog Smoke"
printf 'verified_at=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"

header "NLOG"
nlog_cfg="$(fetch 'https://www.nlog.nl/nlog-mapviewer/rest/config')"
nlog_surveys="$(fetch 'https://www.nlog.nl/nlog-mapviewer/rest/smc/3d/surveys')"
nlog_version="$(printf '%s' "$nlog_cfg" | extract_json_string 'nlogloketVersion' || true)"
nlog_f3="$(printf '%s' "$nlog_surveys" | tr -d '\n' | rg -o '\{"id":[0-9]+,"name":"Z3NAM2006A".*?"blocks":"F02, F03".*?\}' | head -n 1 || true)"
printf 'version=%s\n' "${nlog_version:-unknown}"
printf 'survey_count=%s\n' "$(printf '%s' "$nlog_surveys" | rg -o '"id":' | wc -l | tr -d ' ')"
if [[ -n "${nlog_f3:-}" ]]; then
  printf 'f3_sample=%s\n' "$nlog_f3"
fi

header "SODIR"
sodir_root="$(fetch 'https://factmaps.sodir.no/api/rest/')"
sodir_service="$(fetch 'https://factmaps.sodir.no/api/rest/services/Factmaps/FactMapsWGS84/FeatureServer?f=pjson')"
sodir_count="$(fetch 'https://factmaps.sodir.no/api/rest/services/Factmaps/FactMapsWGS84/FeatureServer/405/query?where=1%3D1&returnCountOnly=true&f=json')"
sodir_ids="$(fetch 'https://factmaps.sodir.no/api/rest/services/Factmaps/FactMapsWGS84/FeatureServer/405/query?where=1%3D1&returnIdsOnly=true&f=json')"
sodir_first_id="$(printf '%s' "$sodir_ids" | rg -o '"objectIds":\[[0-9]+' | sed 's/.*\[//' | head -n 1 || true)"
printf 'rest_directory=%s\n' "$(printf '%s' "$sodir_root" | rg -o 'ArcGIS REST Services Directory' | head -n 1 || echo no)"
printf 'supports_export_formats=%s\n' "$(printf '%s' "$sodir_service" | extract_json_string 'supportedExportFormats' || true)"
printf 'ongoing_survey_count=%s\n' "$(printf '%s' "$sodir_count" | rg -o '"count":[0-9]+' | sed 's/.*://' || true)"
if [[ -n "${sodir_first_id:-}" ]]; then
  sodir_sample="$(fetch "https://factmaps.sodir.no/api/rest/services/Factmaps/FactMapsWGS84/FeatureServer/405/query?objectIds=${sodir_first_id}&outFields=seaName,seaStatus,seaCompanyReported,seaSurveyTypeMain,seaSurveyTypePart,sea3DKm2&returnGeometry=false&f=json")"
  printf 'sample=%s\n' "$sodir_sample"
fi

header "NZP&M GIS"
nzpam_service="$(fetch 'https://gis.nzpam.govt.nz/server/rest/services/Public/GeodataCatalogue_Layers/MapServer?f=pjson')"
nzpam_layer="$(fetch 'https://gis.nzpam.govt.nz/server/rest/services/Public/GeodataCatalogue_Layers/MapServer/7?f=pjson')"
nzpam_sample="$(fetch 'https://gis.nzpam.govt.nz/server/rest/services/Public/GeodataCatalogue_Layers/MapServer/7/query?where=1%3D1&outFields=Title,Survey_Subtype,Dimension,Operator,Environment,Open_File&returnGeometry=false&resultRecordCount=2&f=json')"
printf 'has_seismic_2d=%s\n' "$(printf '%s' "$nzpam_service" | rg -o '"name"[[:space:]]*:[[:space:]]*"Seismic Surveys 2D"' | head -n 1 || echo no)"
printf 'has_seismic_3d=%s\n' "$(printf '%s' "$nzpam_service" | rg -o '"name"[[:space:]]*:[[:space:]]*"Seismic Surveys 3D"' | head -n 1 || echo no)"
printf 'supported_extensions=%s\n' "$(printf '%s' "$nzpam_service" | extract_json_string 'supportedExtensions' || true)"
printf 'sample=%s\n' "$nzpam_sample"

header "NZP&M Catalogue"
nzpam_catalog_root="$(fetch 'https://geodata.nzpam.govt.nz/')"
nzpam_catalog_search="$(fetch -A 'Mozilla/5.0' 'https://geodata.nzpam.govt.nz/dataset/?q=seismic')"
nzpam_catalog_api="$(fetch 'https://geodata.nzpam.govt.nz/api/3/action/package_search?q=seismic&rows=1' || true)"
printf 'catalog_generator=%s\n' "$(printf '%s' "$nzpam_catalog_root" | rg -o 'ckan[[:space:]]+2\.[0-9]+\.[0-9]+' | head -n 1 || echo unknown)"
printf 'search_has_segy_facet=%s\n' "$(printf '%s' "$nzpam_catalog_search" | rg -o '>SEGY<|>SGY<|>SEGD<' | tr -d '<>' | sort -u | tr '\n' ',' | sed 's/,$//' || true)"
if printf '%s' "$nzpam_catalog_api" | rg -q 'The request is blocked'; then
  printf 'api_status=blocked\n'
else
  printf 'api_status=reachable\n'
fi

header "Poseidon AWS"
poseidon_registry="$(fetch 'https://registry.opendata.aws/tgs-opendata-poseidon/')"
poseidon_root="$(fetch 'https://tgs-opendata-poseidon.s3.amazonaws.com/?list-type=2&prefix=&delimiter=/')"
printf 'registry_mentions_velocity=%s\n' "$(printf '%s' "$poseidon_registry" | rg -o 'stacking velocity field' | head -n 1 || echo no)"
printf 'root_prefixes=%s\n' "$(printf '%s' "$poseidon_root" | rg -o '<Prefix>[^<]+' | sed 's/<Prefix>//' | tr '\n' ',' | sed 's/,$//')"

header "NOPIMS"
nopims_discovery="$(fetch 'https://www.ga.gov.au/about/projects/resources/the-repository/discovery-and-access')"
nopims_houtman="$(fetch 'https://www.ga.gov.au/about/projects/resources/northern-houtman-sub-basin-project')"
nopims_barrow="$(fetch 'https://www.ga.gov.au/nopims/news/barrow-dampier-ccs-presdm-repro-2022-3d')"
printf 'direct_access_phrase=%s\n' "$(printf '%s' "$nopims_discovery" | rg -o 'Data is accessible either directly via NOPIMS' | head -n 1 || echo no)"
printf 'houtman_download_phrase=%s\n' "$(printf '%s' "$nopims_houtman" | rg -o 'available for download via the National Offshore Petroleum Information Management System' | head -n 1 || echo no)"
printf 'barrow_rich_products=%s\n' "$(printf '%s' "$nopims_barrow" | rg -o 'raw and final migrated angle stacks, velocity model, AVO products and migrated gathers' | head -n 1 || echo no)"
printf 'barrow_reference=%s\n' "$(printf '%s' "$nopims_barrow" | rg -o 'ENO[0-9]+' | head -n 1 || echo unknown)"

header "NAMSS"
namss_page="$(fetch 'https://walrus.wr.usgs.gov/namss/web-services/')"
printf 'wms_endpoint=%s\n' "$(printf '%s' "$namss_page" | rg -o 'https://walrus\.wr\.usgs\.gov/namss/wms\?request=GetCapabilities[^<"]+' | head -n 1 || true)"
