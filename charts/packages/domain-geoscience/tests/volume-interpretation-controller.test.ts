import { describe, expect, it } from "bun:test";
import { createMockVolumeInterpretationModel } from "@ophiolite/charts-data-models";
import type {
  VolumeInterpretationDataSource,
  VolumeInterpretationModel
} from "@ophiolite/charts-data-models";
import type {
  VolumeInterpretationPickDebugSnapshot,
  VolumeInterpretationPickResult,
  VolumeInterpretationRenderFrame,
  VolumeInterpretationRendererAdapter
} from "@ophiolite/charts-renderer";
import { VolumeInterpretationController } from "../src/volume-interpretation/controller";

class VolumeInterpretationTestRenderer implements VolumeInterpretationRendererAdapter {
  lastFrame: VolumeInterpretationRenderFrame | null = null;

  mount(_container: HTMLElement): void {}

  render(frame: VolumeInterpretationRenderFrame): void {
    this.lastFrame = frame;
  }

  pick(_screenX: number, _screenY: number): VolumeInterpretationPickResult | null {
    return null;
  }

  projectWorldToScreen(_worldX: number, _worldY: number, _worldZ: number): { x: number; y: number } | null {
    return null;
  }

  debugPick(_screenX: number, _screenY: number): VolumeInterpretationPickDebugSnapshot {
    return {
      pointerX: 0,
      pointerY: 0,
      renderPointerX: 0,
      renderPointerY: 0,
      renderScaleX: 1,
      renderScaleY: 1,
      actualWinner: null,
      actualPickedCount: 0,
      actualMatchedBy: null,
      syntheticWinner: null,
      winner: null,
      candidates: []
    };
  }

  dispose(): void {}
}

describe("VolumeInterpretationController", () => {
  it("accepts data-source handles without structured cloning them", () => {
    const renderer = new VolumeInterpretationTestRenderer();
    const controller = new VolumeInterpretationController(renderer);
    const model = withDataSource(createMockVolumeInterpretationModel());
    const source = model.volumes[0]!.dataSource!;

    controller.mount({} as HTMLElement);
    controller.setModel(model);

    const state = controller.getState();
    expect(state.model?.volumes[0]?.dataSource).toBe(source);
    expect(renderer.lastFrame?.state.model?.volumes[0]?.dataSource).toBe(source);
    expect(state.model?.horizons[0]?.points).toBe(model.horizons[0]!.points);
  });
});

function withDataSource(model: VolumeInterpretationModel): VolumeInterpretationModel {
  const source: VolumeInterpretationDataSource = {
    id: "controller-test-source",
    kind: "slice",
    preferredOwnership: "view",
    loadSlice: async (request) => ({
      volumeId: request.volumeId,
      fieldId: request.fieldId,
      axis: request.axis,
      position: request.position,
      lod: request.lod ?? 0,
      bounds: model.sceneBounds,
      dimensions: {
        width: 1,
        height: 1
      },
      sampleFormat: "f32",
      ownership: "view",
      values: new Float32Array([0])
    })
  };

  return {
    ...model,
    volumes: [
      {
        ...model.volumes[0]!,
        dataSource: source
      }
    ]
  };
}
