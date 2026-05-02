import { describe, expect, it } from "bun:test";
import {
  cloneVolumeInterpretationModel,
  resolveActiveVolumeScalarField
} from "../src/volume-interpretation";
import type { VolumeInterpretationDataSource } from "../src/volume-interpretation";
import { adaptOphioliteVolumeInterpretationToChart } from "../src/ophiolite-volume-interpretation-adapter";
import {
  createMockOphioliteVolumeInterpretationSource,
  createMockVolumeInterpretationModel
} from "../src/volume-interpretation-mock";

describe("volume interpretation data-source model", () => {
  it("tracks active scalar metadata separately from volume geometry", () => {
    const model = createMockVolumeInterpretationModel();
    const volume = model.volumes[0]!;

    expect(resolveActiveVolumeScalarField(volume)?.id).toBe("amplitude");
    expect(resolveActiveVolumeScalarField({ ...volume, activeFieldId: "missing" })?.id).toBe("amplitude");
  });

  it("adapts the resolved Ophiolite volume source into the chart model boundary", () => {
    const source = createMockOphioliteVolumeInterpretationSource();
    const model = adaptOphioliteVolumeInterpretationToChart(source);

    expect(model.id).toBe(source.id);
    expect(model.sampleDomain).toBe(source.sample_domain);
    expect(model.volumes[0]?.activeFieldId).toBe("amplitude");
    expect(model.volumes[0]?.dataSource).toBe(source.volumes[0]?.data_source);
    expect(model.slicePlanes[0]?.volumeId).toBe(source.slice_planes[0]?.volume_id);
    expect(model.horizons[0]?.style.showContours).toBe(source.horizons[0]?.style.show_contours);
  });

  it("mock volume source feeds slices through the chart data-source contract", async () => {
    const model = createMockVolumeInterpretationModel();
    const volume = model.volumes[0]!;
    const field = resolveActiveVolumeScalarField(volume)!;

    const payload = await volume.dataSource?.loadSlice?.({
      volumeId: volume.id,
      fieldId: field.id,
      axis: "inline",
      position: 430,
      lod: 0
    });

    expect(payload?.sampleFormat).toBe("f32");
    expect(payload?.ownership).toBe("view");
    expect(payload?.dimensions).toEqual({ width: 128, height: 256 });
    expect(payload?.values.byteLength).toBe(128 * 256 * Float32Array.BYTES_PER_ELEMENT);
  });

  it("clones semantic scene state while preserving binary ownership handles", () => {
    const model = createMockVolumeInterpretationModel();
    const source: VolumeInterpretationDataSource = {
      id: "test-source",
      kind: "slice-and-brick",
      preferredOwnership: "view",
      loadSlice: async () => {
        throw new Error("not used");
      }
    };
    const horizonPoints = model.horizons[0]!.points;
    const wellPoints = model.wells[0]!.points;
    model.volumes[0] = {
      ...model.volumes[0]!,
      dataSource: source
    };

    const cloned = cloneVolumeInterpretationModel(model);

    expect(cloned).not.toBe(model);
    expect(cloned.sceneBounds).not.toBe(model.sceneBounds);
    expect(cloned.volumes[0]).not.toBe(model.volumes[0]);
    expect(cloned.volumes[0]!.dataSource).toBe(source);
    expect(cloned.horizons[0]!.points).toBe(horizonPoints);
    expect(cloned.wells[0]!.points).toBe(wellPoints);
  });
});
