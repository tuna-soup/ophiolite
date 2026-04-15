import type { VolumeInterpretationModel } from "@ophiolite/charts-data-models";

export interface VolumeInterpretationStageSize {
  width: number;
  height: number;
}

export function resolveVolumeInterpretationStageSize(
  model: VolumeInterpretationModel | null
): VolumeInterpretationStageSize {
  if (!model) {
    return {
      width: 820,
      height: 560
    };
  }

  const spanX = Math.max(1, model.sceneBounds.maxX - model.sceneBounds.minX);
  const spanY = Math.max(1, model.sceneBounds.maxY - model.sceneBounds.minY);
  const aspect = spanX / spanY;
  return {
    width: Math.max(760, Math.round(760 * Math.max(0.85, Math.min(1.35, aspect)))),
    height: 560
  };
}

export function scaleVolumeInterpretationStageSize(
  size: VolumeInterpretationStageSize,
  scale: number
): VolumeInterpretationStageSize {
  const factor = Number.isFinite(scale) && scale > 0 ? scale : 1;
  return {
    width: Math.round(size.width * factor),
    height: Math.round(size.height * factor)
  };
}
