interface RockPhysicsStageSize {
  width: number;
  height: number;
}

const BASE_STAGE_SIZE: RockPhysicsStageSize = {
  width: 960,
  height: 560
};

export function resolveRockPhysicsStageSize(): RockPhysicsStageSize {
  return { ...BASE_STAGE_SIZE };
}

export function scaleRockPhysicsStageSize(
  size: RockPhysicsStageSize,
  scale: number | null | undefined
): RockPhysicsStageSize {
  const safeScale = Number.isFinite(scale) && (scale ?? 0) > 0 ? (scale as number) : 1;
  return {
    width: Math.max(1, Math.round(size.width * safeScale)),
    height: Math.max(1, Math.round(size.height * safeScale))
  };
}
