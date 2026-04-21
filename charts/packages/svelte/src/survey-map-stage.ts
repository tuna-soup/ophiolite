import { computeSurveyMapBounds, SURVEY_MAP_MARGIN } from "@ophiolite/charts-core";
import type { SurveyMapModel } from "@ophiolite/charts-data-models";

interface SurveyMapStageSize {
  width: number;
  height: number;
}

const MIN_PLOT_SIZE = 360;
const MAX_PLOT_SIZE = 460;
const DEFAULT_PLOT_SIZE = 420;
const MIN_STAGE_WIDTH = SURVEY_MAP_MARGIN.left + SURVEY_MAP_MARGIN.right + MIN_PLOT_SIZE;
const MAX_STAGE_WIDTH = SURVEY_MAP_MARGIN.left + SURVEY_MAP_MARGIN.right + MAX_PLOT_SIZE;
const MIN_STAGE_HEIGHT = SURVEY_MAP_MARGIN.top + SURVEY_MAP_MARGIN.bottom + MIN_PLOT_SIZE;
const MAX_STAGE_HEIGHT = SURVEY_MAP_MARGIN.top + SURVEY_MAP_MARGIN.bottom + MAX_PLOT_SIZE;

export function resolveSurveyMapStageSize(map: SurveyMapModel | null): SurveyMapStageSize {
  const bounds = computeSurveyMapBounds(map);
  const spanX = Math.max(1, (bounds?.maxX ?? 1) - (bounds?.minX ?? 0));
  const spanY = Math.max(1, (bounds?.maxY ?? 1) - (bounds?.minY ?? 0));
  const coverage = Math.min(spanX, spanY) / Math.max(spanX, spanY);
  const plotSize = clamp(DEFAULT_PLOT_SIZE + Math.round(coverage * 24), MIN_PLOT_SIZE, MAX_PLOT_SIZE);

  return {
    width: clamp(SURVEY_MAP_MARGIN.left + SURVEY_MAP_MARGIN.right + plotSize, MIN_STAGE_WIDTH, MAX_STAGE_WIDTH),
    height: clamp(SURVEY_MAP_MARGIN.top + SURVEY_MAP_MARGIN.bottom + plotSize, MIN_STAGE_HEIGHT, MAX_STAGE_HEIGHT)
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
