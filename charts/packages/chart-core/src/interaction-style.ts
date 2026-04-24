import type { ChartInteractionBinding, ChartInteractionStyle, InteractionTrigger } from "@ophiolite/charts-data-models";
import { matchesInteractionBinding } from "@ophiolite/charts-data-models";

export function defineInteractionStyle(style: ChartInteractionStyle): ChartInteractionStyle {
  return {
    ...style,
    manipulators: [...style.manipulators],
    bindings: style.bindings.map((binding) => ({ ...binding }))
  };
}

export function resolveInteractionBinding(
  style: ChartInteractionStyle | null,
  trigger: InteractionTrigger
): ChartInteractionBinding | null {
  if (!style) {
    return null;
  }
  return style.bindings.find((binding) => matchesInteractionBinding(binding, trigger)) ?? null;
}
