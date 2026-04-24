import type { ChartInteractionCommand, ChartInteractionStyle, InteractionTrigger } from "@ophiolite/charts-data-models";
import { resolveInteractionBinding } from "./interaction-style";

export interface InteractionCommandContext {
  emit: (command: ChartInteractionCommand) => void;
}

export interface InteractionManipulator {
  readonly id: string;
  handles: (command: ChartInteractionCommand) => boolean;
  execute: (command: ChartInteractionCommand, context: InteractionCommandContext) => void;
}

export class InteractionDispatcher {
  private readonly manipulators: readonly InteractionManipulator[];

  constructor(manipulators: readonly InteractionManipulator[]) {
    this.manipulators = manipulators;
  }

  dispatchTrigger(
    style: ChartInteractionStyle | null,
    trigger: InteractionTrigger,
    context: InteractionCommandContext
  ): ChartInteractionCommand | null {
    const binding = resolveInteractionBinding(style, trigger);
    if (!binding) {
      return null;
    }
    const command: ChartInteractionCommand = {
      type: "semanticAction",
      command: binding.command
    };
    this.dispatchCommand(command, context);
    return command;
  }

  dispatchCommand(command: ChartInteractionCommand, context: InteractionCommandContext): void {
    let handled = false;
    for (const manipulator of this.manipulators) {
      if (!manipulator.handles(command)) {
        continue;
      }
      manipulator.execute(command, context);
      handled = true;
    }
    if (!handled) {
      context.emit(command);
    }
  }
}
