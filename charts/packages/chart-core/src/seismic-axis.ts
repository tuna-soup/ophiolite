import type { AxisPresentationRow, SectionPayload } from "@ophiolite/charts-data-models";

export function buildSeismicTickIndices(start: number, end: number, maxTicks: number): number[] {
  const count = end - start;
  if (count <= 0) {
    return [];
  }

  const tickCount = Math.min(maxTicks, count);
  const ticks = new Set<number>();
  for (let index = 0; index < tickCount; index += 1) {
    const ratio = tickCount === 1 ? 0 : index / (tickCount - 1);
    ticks.add(start + Math.round(ratio * (count - 1)));
  }
  return [...ticks].sort((left, right) => left - right);
}

export function buildSeismicTopAxisRows(section: SectionPayload): AxisPresentationRow[] {
  if (section.presentation?.topAxisRows?.length) {
    return section.presentation.topAxisRows;
  }

  if (isArbitrarySeismicSection(section) && section.inlineAxis && section.xlineAxis) {
    return [
      { label: "Trace", values: section.horizontalAxis },
      { label: "IL", values: section.inlineAxis },
      { label: "XL", values: section.xlineAxis }
    ];
  }

  return [
    {
      label: section.axis === "inline" ? "Xline" : "Inline",
      values: section.horizontalAxis
    }
  ];
}

export function resolveSeismicSectionTitle(section: SectionPayload): string {
  return (
    section.presentation?.title ??
    (isArbitrarySeismicSection(section)
      ? "Arbitrary Section"
      : `${capitalize(section.axis)}: ${formatSeismicAxisValue(section.coordinate.value)}`)
  );
}

export function resolveSeismicSampleAxisTitle(section: SectionPayload): string {
  const sampleAxisLabel = section.presentation?.sampleAxisLabel ?? "Sample";
  return section.units?.sample ? `${sampleAxisLabel} (${section.units.sample})` : sampleAxisLabel;
}

export function formatSeismicAxisValue(value: number): string {
  if (Math.abs(value) >= 100) {
    return Math.round(value).toString();
  }
  return value.toFixed(1);
}

export function isArbitrarySeismicSection(section: SectionPayload): boolean {
  return hasAxisVariation(section.inlineAxis) && hasAxisVariation(section.xlineAxis);
}

function hasAxisVariation(axis: Float64Array | undefined): boolean {
  if (!axis || axis.length < 2) {
    return false;
  }

  const first = axis[0]!;
  for (let index = 1; index < axis.length; index += 1) {
    if (Math.abs(axis[index]! - first) > 1e-6) {
      return true;
    }
  }
  return false;
}

function capitalize(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1);
}
