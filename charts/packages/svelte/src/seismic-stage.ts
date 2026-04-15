import { PLOT_MARGIN } from "@ophiolite/charts-renderer";

type SeismicStageKind = "section" | "gather";
type SeismicRenderMode = "heatmap" | "wiggle";

interface SeismicStageSize {
  width: number;
  height: number;
}

const SECTION_HEATMAP_TRACE_SPACING_PX = 2.25;
const SECTION_WIGGLE_TRACE_SPACING_PX = 6;
const GATHER_TRACE_SPACING_PX = 34;
const SECTION_MIN_WIDTH = 520;
const SECTION_MAX_WIDTH = 760;
const SECTION_WIGGLE_MAX_WIDTH = 1200;
const GATHER_MIN_WIDTH = 520;
const GATHER_MAX_WIDTH = 760;
const SAMPLE_SPACING_PX = 1.15;
const MIN_STAGE_HEIGHT = 420;
const MAX_STAGE_HEIGHT = 420;

export function resolveSeismicStageSize(
  kind: SeismicStageKind,
  traceCount: number | null | undefined,
  sampleCount: number | null | undefined,
  renderMode: SeismicRenderMode
): SeismicStageSize {
  const safeTraceCount = Math.max(1, Math.round(traceCount ?? 0));
  const safeSampleCount = Math.max(1, Math.round(sampleCount ?? 0));
  const traceSpacing =
    kind === "gather"
      ? GATHER_TRACE_SPACING_PX
      : renderMode === "wiggle"
        ? SECTION_WIGGLE_TRACE_SPACING_PX
        : SECTION_HEATMAP_TRACE_SPACING_PX;
  const minWidth = kind === "gather" ? GATHER_MIN_WIDTH : SECTION_MIN_WIDTH;
  const maxWidth =
    kind === "gather"
      ? GATHER_MAX_WIDTH
      : renderMode === "wiggle"
        ? SECTION_WIGGLE_MAX_WIDTH
        : SECTION_MAX_WIDTH;

  return {
    width: clamp(PLOT_MARGIN.left + PLOT_MARGIN.right + safeTraceCount * traceSpacing, minWidth, maxWidth),
    height: clamp(PLOT_MARGIN.top + PLOT_MARGIN.bottom + safeSampleCount * SAMPLE_SPACING_PX, MIN_STAGE_HEIGHT, MAX_STAGE_HEIGHT)
  };
}

export function scaleSeismicStageSize(size: SeismicStageSize, scale: number | null | undefined): SeismicStageSize {
  const safeScale = Number.isFinite(scale) && (scale ?? 0) > 0 ? (scale as number) : 1;
  return {
    width: Math.max(1, Math.round(size.width * safeScale)),
    height: Math.max(1, Math.round(size.height * safeScale))
  };
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
