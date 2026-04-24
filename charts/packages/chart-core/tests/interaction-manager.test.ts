import { describe, expect, it } from "bun:test";
import type { ChartInteractionStyle, InteractionCapabilities } from "@ophiolite/charts-data-models";
import { InteractionManager } from "../src/interaction-manager";

const CAPABILITIES: InteractionCapabilities = {
  primaryModes: ["cursor", "panZoom"],
  modifiers: ["crosshair"]
};

const STYLE: ChartInteractionStyle = {
  id: "survey-map-navigation",
  label: "Survey Map Navigation",
  manipulators: ["viewport-navigation"],
  bindings: [
    {
      trigger: "pointer-primary",
      primaryMode: "panZoom",
      command: "viewport.pan"
    },
    {
      trigger: "keyboard",
      key: "ArrowLeft",
      command: "viewport.panLeft"
    }
  ]
};

describe("InteractionManager.resolveTriggerCommand", () => {
  it("resolves semantic commands from the active style", () => {
    const manager = new InteractionManager(CAPABILITIES, "panZoom", [], STYLE);

    expect(
      manager.resolveTriggerCommand({
        type: "pointer-primary",
        primaryMode: "panZoom",
        modifiers: []
      })
    ).toEqual({
      type: "semanticAction",
      command: "viewport.pan"
    });
  });

  it("returns null when the trigger does not match the style bindings", () => {
    const manager = new InteractionManager(CAPABILITIES, "cursor", [], STYLE);

    expect(
      manager.resolveTriggerCommand({
        type: "pointer-primary",
        primaryMode: "cursor",
        modifiers: []
      })
    ).toBeNull();
  });
});
