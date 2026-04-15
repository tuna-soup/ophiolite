import { computeSurveyMapBounds, SURVEY_MAP_MARGIN } from "@ophiolite/charts-core";
import type { SurveyMapModel } from "@ophiolite/charts-data-models";

interface SurveyMapStageSize {
  width: number;
  height: number;
}

const PLOT_HEIGHT = 320;
const MIN_STAGE_WIDTH = 520;
const MAX_STAGE_WIDTH = 760;
const MIN_STAGE_HEIGHT = 372;

export function resolveSurveyMapStageSize(map: SurveyMapModel | null): SurveyMapStageSize {
  const bounds = computeSurveyMapBounds(map);
  const spanX = Math.max(1, (bounds?.maxX ?? 1) - (bounds?.minX ?? 0));
  const spanY = Math.max(1, (bounds?.maxY ?? 1) - (bounds?.minY ?? 0));
  const aspect = spanX / spanY;
  const intrinsicWidth = SURVEY_MAP_MARGIN.left + SURVEY_MAP_MARGIN.right + PLOT_HEIGHT * aspect;

  return {
    width: clamp(intrinsicWidth, MIN_STAGE_WIDTH, MAX_STAGE_WIDTH),
    height: MIN_STAGE_HEIGHT
  };
}

export function scaleSurveyMapStageSize(size: SurveyMapStageSize, scale: number | null | undefined): SurveyMapStageSize {
  const safeScale = Number.isFinite(scale) && (scale ?? 0) > 0 ? (scale as number) : 1;
  return {
    width: Math.max(1, Math.round(size.width * safeScale)),
    height: Math.max(1, Math.round(size.height * safeScale))
  };
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}

