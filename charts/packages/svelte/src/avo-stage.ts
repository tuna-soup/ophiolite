interface AvoStageSize {
  width: number;
  height: number;
}

const BASE_STAGE_SIZE: AvoStageSize = {
  width: 960,
  height: 560
};

export function resolveAvoStageSize(): AvoStageSize {
  return { ...BASE_STAGE_SIZE };
}

export function scaleAvoStageSize(size: AvoStageSize, scale: number | null | undefined): AvoStageSize {
  const safeScale = Number.isFinite(scale) && (scale ?? 0) > 0 ? (scale as number) : 1;
  return {
    width: Math.max(1, Math.round(size.width * safeScale)),
    height: Math.max(1, Math.round(size.height * safeScale))
  };
}
