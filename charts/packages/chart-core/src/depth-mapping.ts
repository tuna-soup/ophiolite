import type { DepthMappingSample } from "@ophiolite/charts-data-models";

export function mapNativeDepthToPanelDepth(mapping: DepthMappingSample[], nativeDepth: number): number {
  return interpolateDepth(mapping, nativeDepth, "nativeDepth", "panelDepth");
}

export function mapPanelDepthToNativeDepth(mapping: DepthMappingSample[], panelDepth: number): number {
  return interpolateDepth(mapping, panelDepth, "panelDepth", "nativeDepth");
}

function interpolateDepth(
  mapping: DepthMappingSample[],
  value: number,
  fromKey: "nativeDepth" | "panelDepth",
  toKey: "nativeDepth" | "panelDepth"
): number {
  if (mapping.length === 0) {
    return value;
  }
  if (mapping.length === 1) {
    return mapping[0]![toKey];
  }

  if (value <= mapping[0]![fromKey]) {
    return projectLinear(mapping[0]!, mapping[1]!, value, fromKey, toKey);
  }
  if (value >= mapping[mapping.length - 1]![fromKey]) {
    return projectLinear(mapping[mapping.length - 2]!, mapping[mapping.length - 1]!, value, fromKey, toKey);
  }

  for (let index = 1; index < mapping.length; index += 1) {
    const left = mapping[index - 1]!;
    const right = mapping[index]!;
    if (value >= left[fromKey] && value <= right[fromKey]) {
      return projectLinear(left, right, value, fromKey, toKey);
    }
  }

  return value;
}

function projectLinear(
  left: DepthMappingSample,
  right: DepthMappingSample,
  value: number,
  fromKey: "nativeDepth" | "panelDepth",
  toKey: "nativeDepth" | "panelDepth"
): number {
  const span = right[fromKey] - left[fromKey];
  if (span === 0) {
    return left[toKey];
  }
  const ratio = (value - left[fromKey]) / span;
  return left[toKey] + ratio * (right[toKey] - left[toKey]);
}

