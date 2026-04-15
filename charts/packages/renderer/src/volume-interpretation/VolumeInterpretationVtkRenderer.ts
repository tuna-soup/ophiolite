import "@kitware/vtk.js/Rendering/Profiles/Geometry";
import "@kitware/vtk.js/Rendering/Profiles/Volume";

import vtkDataArray from "@kitware/vtk.js/Common/Core/DataArray";
import vtkCellArray from "@kitware/vtk.js/Common/Core/CellArray";
import vtkPoints from "@kitware/vtk.js/Common/Core/Points";
import vtkImageData from "@kitware/vtk.js/Common/DataModel/ImageData";
import vtkPiecewiseFunction from "@kitware/vtk.js/Common/DataModel/PiecewiseFunction";
import vtkPolyData from "@kitware/vtk.js/Common/DataModel/PolyData";
import vtkTubeFilter from "@kitware/vtk.js/Filters/General/TubeFilter";
import vtkCubeSource from "@kitware/vtk.js/Filters/Sources/CubeSource";
import vtkSphereSource from "@kitware/vtk.js/Filters/Sources/SphereSource";
import vtkActor from "@kitware/vtk.js/Rendering/Core/Actor";
import vtkColorTransferFunction from "@kitware/vtk.js/Rendering/Core/ColorTransferFunction";
import vtkImageMapper from "@kitware/vtk.js/Rendering/Core/ImageMapper";
import vtkMapper from "@kitware/vtk.js/Rendering/Core/Mapper";
import vtkVolume from "@kitware/vtk.js/Rendering/Core/Volume";
import vtkVolumeMapper from "@kitware/vtk.js/Rendering/Core/VolumeMapper";
import vtkImageSlice from "@kitware/vtk.js/Rendering/Core/ImageSlice";
import vtkGenericRenderWindow from "@kitware/vtk.js/Rendering/Misc/GenericRenderWindow";
import type vtkRenderer from "@kitware/vtk.js/Rendering/Core/Renderer";
import type {
  VolumeInterpretationAnnotation,
  VolumeInterpretationBounds,
  VolumeInterpretationColorMap,
  VolumeInterpretationHorizonSurface,
  VolumeInterpretationMarker,
  VolumeInterpretationModel,
  VolumeInterpretationSelection,
  VolumeInterpretationSlicePlane,
  VolumeInterpretationView,
  VolumeInterpretationVolume,
  VolumeInterpretationWellTrajectory
} from "@ophiolite/charts-data-models";
import type {
  VolumeInterpretationPickResult,
  VolumeInterpretationRenderFrame,
  VolumeInterpretationRendererAdapter
} from "./adapter";

interface Point2D {
  x: number;
  y: number;
}

interface ProjectedPolygonTarget {
  type: "polygon";
  pick: VolumeInterpretationPickResult;
  points: Point2D[];
}

interface ProjectedPolylineTarget {
  type: "polyline";
  pick: VolumeInterpretationPickResult;
  points: Point2D[];
  strokeWidth: number;
}

interface ProjectedPointTarget {
  type: "point";
  pick: VolumeInterpretationPickResult;
  point: Point2D;
  radius: number;
}

type ProjectedTarget = ProjectedPolygonTarget | ProjectedPolylineTarget | ProjectedPointTarget;

interface HighlightableEntry {
  itemId: string;
  apply(selected: boolean): void;
}

interface SyntheticVolumeResource {
  imageData: vtkImageData;
  range: [number, number];
}

const BG_COLOR: [number, number, number] = [13 / 255, 24 / 255, 34 / 255];

export class VolumeInterpretationVtkRenderer implements VolumeInterpretationRendererAdapter {
  private container: HTMLElement | null = null;
  private host: HTMLDivElement | null = null;
  private genericRenderWindow: vtkGenericRenderWindow | null = null;
  private renderer: vtkRenderer | null = null;
  private projectedTargets: ProjectedTarget[] = [];
  private highlightables: HighlightableEntry[] = [];
  private lastModel: VolumeInterpretationModel | null = null;
  private readonly volumeResources = new Map<string, SyntheticVolumeResource>();

  mount(container: HTMLElement): void {
    this.container = container;
    this.host = document.createElement("div");
    this.host.style.width = "100%";
    this.host.style.height = "100%";
    this.host.style.display = "block";
    container.appendChild(this.host);

    this.genericRenderWindow = vtkGenericRenderWindow.newInstance({
      background: BG_COLOR,
      listenWindowResize: false
    });
    this.genericRenderWindow.setContainer(this.host);
    this.genericRenderWindow.resize();
    this.renderer = this.genericRenderWindow.getRenderer();
  }

  render(frame: VolumeInterpretationRenderFrame): void {
    if (!this.genericRenderWindow || !this.renderer || !this.host) {
      return;
    }

    const { model, selection, view } = frame.state;
    this.genericRenderWindow.resize();

    if (model !== this.lastModel) {
      this.rebuildScene(model);
      this.lastModel = model;
    }

    if (!model || !view) {
      this.projectedTargets = [];
      this.genericRenderWindow.getRenderWindow().render();
      return;
    }

    updateCamera(this.renderer, model.sceneBounds, view);
    this.renderer.resetCameraClippingRange();
    this.applySelection(selection);
    this.genericRenderWindow.getRenderWindow().render();
    this.projectedTargets = this.buildProjectedTargets(model);
  }

  pick(screenX: number, screenY: number): VolumeInterpretationPickResult | null {
    let best: { pick: VolumeInterpretationPickResult; score: number } | null = null;
    for (const target of this.projectedTargets) {
      const score = pickScore(target, screenX, screenY);
      if (score === null) {
        continue;
      }
      if (!best || score < best.score) {
        best = {
          pick: {
            ...target.pick,
            screenX,
            screenY
          },
          score
        };
      }
    }
    return best?.pick ?? null;
  }

  dispose(): void {
    this.projectedTargets = [];
    this.highlightables = [];
    this.lastModel = null;
    this.volumeResources.clear();
    this.renderer?.removeAllViewProps();
    this.genericRenderWindow?.delete();
    this.genericRenderWindow = null;
    this.renderer = null;
    if (this.host?.parentNode) {
      this.host.parentNode.removeChild(this.host);
    }
    this.host = null;
    this.container = null;
  }

  private rebuildScene(model: VolumeInterpretationModel | null): void {
    if (!this.renderer) {
      return;
    }

    this.renderer.removeAllViewProps();
    this.highlightables = [];
    this.projectedTargets = [];

    if (!model) {
      return;
    }

    const primaryVolume = model.volumes[0] ?? null;
    if (primaryVolume) {
      const resource = this.getOrCreateVolumeResource(primaryVolume);
      this.renderer.addActor(createBoundsActor(primaryVolume.bounds, [0.84, 0.9, 0.94], 0.3));
      if (model.cropBox) {
        this.renderer.addActor(createBoundsActor(model.cropBox, [1, 0.85, 0.45], 0.55));
      }
      this.renderer.addVolume(createVolumeActor(primaryVolume, resource));

      for (const plane of model.slicePlanes) {
        if (!plane.visible || plane.volumeId !== primaryVolume.id) {
          continue;
        }
        const slice = createSliceActor(plane, primaryVolume, resource);
        this.renderer.addActor(slice.actor);
        this.renderer.addActor(slice.borderActor);
        this.highlightables.push({
          itemId: plane.id,
          apply: (selected) => {
            slice.actor.getProperty().setOpacity(selected ? Math.min(1, plane.style.opacity + 0.08) : plane.style.opacity);
            slice.borderActor.getProperty().setColor(selected ? [1, 0.82, 0.38] : [0.95, 0.97, 0.99]);
            slice.borderActor.getProperty().setLineWidth(selected ? 3 : 1.4);
          }
        });
      }
    }

    for (const horizon of model.horizons) {
      if (!horizon.visible) {
        continue;
      }
      const surface = createHorizonActors(horizon);
      this.renderer.addActor(surface.fillActor);
      this.renderer.addActor(surface.wireActor);
      this.highlightables.push({
        itemId: horizon.id,
        apply: (selected) => {
          surface.wireActor.getProperty().setColor(selected ? [1, 0.8, 0.34] : colorToRgb(horizon.style.edgeColor ?? "#173042"));
          surface.wireActor.getProperty().setLineWidth(selected ? 3 : horizon.style.edgeWidth ?? 1.2);
          surface.fillActor.getProperty().setOpacity(selected ? Math.min(1, horizon.style.fillOpacity + 0.1) : horizon.style.fillOpacity);
        }
      });
    }

    for (const well of model.wells) {
      if (!well.visible) {
        continue;
      }
      const actor = createWellActor(well);
      this.renderer.addActor(actor);
      this.highlightables.push({
        itemId: well.id,
        apply: (selected) => {
          actor.getProperty().setColor(selected ? [1, 0.82, 0.4] : colorToRgb(well.style.color));
          actor.getProperty().setLineWidth(selected ? Math.max(4, well.style.width + 2) : Math.max(2, well.style.width));
        }
      });
    }

    for (const marker of model.markers) {
      if (!marker.visible) {
        continue;
      }
      const actor = createMarkerActor(marker);
      this.renderer.addActor(actor);
      this.highlightables.push({
        itemId: marker.id,
        apply: (selected) => {
          actor.getProperty().setColor(selected ? [1, 0.83, 0.4] : colorToRgb(marker.color));
          actor.setScale(selected ? 1.3 : 1, selected ? 1.3 : 1, selected ? 1.3 : 1);
        }
      });
    }

    for (const annotation of model.annotations ?? []) {
      if (!annotation.visible) {
        continue;
      }
      const actor = createAnnotationActor(annotation);
      this.renderer.addActor(actor);
      this.highlightables.push({
        itemId: annotation.id,
        apply: (selected) => {
          actor.getProperty().setColor(selected ? [1, 0.83, 0.4] : colorToRgb(annotation.color ?? "#ddeaf0"));
          actor.setScale(selected ? 1.3 : 1, selected ? 1.3 : 1, selected ? 1.3 : 1);
        }
      });
    }
  }

  private applySelection(selection: VolumeInterpretationSelection | null): void {
    for (const highlightable of this.highlightables) {
      highlightable.apply(selection?.itemId === highlightable.itemId);
    }
  }

  private getOrCreateVolumeResource(volume: VolumeInterpretationVolume): SyntheticVolumeResource {
    const signature = `${volume.id}:${volume.dimensions.inline}:${volume.dimensions.xline}:${volume.dimensions.sample}`;
    const cached = this.volumeResources.get(signature);
    if (cached) {
      return cached;
    }

    const values = synthesizeVolumeValues(volume);
    const imageData = vtkImageData.newInstance();
    imageData.setDimensions(volume.dimensions.inline, volume.dimensions.xline, volume.dimensions.sample);
    imageData.setOrigin([volume.bounds.minX, volume.bounds.minY, volume.bounds.minZ]);
    imageData.setSpacing([
      span(volume.bounds.minX, volume.bounds.maxX, volume.dimensions.inline),
      span(volume.bounds.minY, volume.bounds.maxY, volume.dimensions.xline),
      span(volume.bounds.minZ, volume.bounds.maxZ, volume.dimensions.sample)
    ]);
    imageData.getPointData().setScalars(
      vtkDataArray.newInstance({
        name: `${volume.id}-amplitude`,
        values,
        numberOfComponents: 1
      })
    );

    const resource: SyntheticVolumeResource = {
      imageData,
      range: minMax(values)
    };
    this.volumeResources.set(signature, resource);
    return resource;
  }

  private buildProjectedTargets(model: VolumeInterpretationModel): ProjectedTarget[] {
    if (!this.renderer || !this.host) {
      return [];
    }

    const width = Math.max(1, this.host.clientWidth);
    const height = Math.max(1, this.host.clientHeight);
    const project = (x: number, y: number, z: number): Point2D => projectPoint(this.renderer!, width, height, x, y, z);
    const targets: ProjectedTarget[] = [];

    for (const plane of model.slicePlanes) {
      if (!plane.visible) {
        continue;
      }
      const volume = model.volumes.find((candidate) => candidate.id === plane.volumeId);
      if (!volume) {
        continue;
      }
      const corners = slicePlaneCorners(plane, volume).map((point) => project(point.x, point.y, point.z));
      const center = polygonCenter(corners);
      targets.push({
        type: "polygon",
        points: corners,
        pick: {
          kind: "slice-plane",
          itemId: plane.id,
          itemName: plane.name,
          worldX: plane.axis === "inline" ? plane.position : (volume.bounds.minX + volume.bounds.maxX) / 2,
          worldY: plane.axis === "xline" ? plane.position : (volume.bounds.minY + volume.bounds.maxY) / 2,
          worldZ: plane.axis === "sample" ? plane.position : (volume.bounds.minZ + volume.bounds.maxZ) / 2,
          screenX: center.x,
          screenY: center.y
        }
      });
    }

    for (const horizon of model.horizons) {
      if (!horizon.visible || horizon.points.length < 12) {
        continue;
      }
      const perimeter = horizonPerimeter(horizon).map((point) => project(point.x, point.y, point.z));
      const centerIndex = Math.floor((horizon.rows * horizon.columns) / 2);
      const center = project(
        horizon.points[centerIndex * 3]!,
        horizon.points[centerIndex * 3 + 1]!,
        horizon.points[centerIndex * 3 + 2]!
      );
      targets.push({
        type: "polygon",
        points: perimeter,
        pick: {
          kind: "horizon-surface",
          itemId: horizon.id,
          itemName: horizon.name,
          worldX: horizon.points[centerIndex * 3]!,
          worldY: horizon.points[centerIndex * 3 + 1]!,
          worldZ: horizon.points[centerIndex * 3 + 2]!,
          screenX: center.x,
          screenY: center.y
        }
      });
    }

    for (const well of model.wells) {
      if (!well.visible || well.points.length < 6) {
        continue;
      }
      const points: Point2D[] = [];
      for (let index = 0; index < well.points.length; index += 3) {
        points.push(project(well.points[index]!, well.points[index + 1]!, well.points[index + 2]!));
      }
      targets.push({
        type: "polyline",
        points,
        strokeWidth: Math.max(3, well.style.width),
        pick: {
          kind: "well-trajectory",
          itemId: well.id,
          itemName: well.name,
          worldX: well.points[0]!,
          worldY: well.points[1]!,
          worldZ: well.points[2]!,
          screenX: points[0]!.x,
          screenY: points[0]!.y
        }
      });
    }

    for (const marker of model.markers) {
      if (!marker.visible) {
        continue;
      }
      const point = project(marker.x, marker.y, marker.z);
      targets.push({
        type: "point",
        point,
        radius: Math.max(10, marker.size + 6),
        pick: {
          kind: "well-marker",
          itemId: marker.id,
          itemName: marker.name,
          worldX: marker.x,
          worldY: marker.y,
          worldZ: marker.z,
          screenX: point.x,
          screenY: point.y
        }
      });
    }

    for (const annotation of model.annotations ?? []) {
      if (!annotation.visible) {
        continue;
      }
      const point = project(annotation.x, annotation.y, annotation.z);
      targets.push({
        type: "point",
        point,
        radius: 14,
        pick: {
          kind: "annotation",
          itemId: annotation.id,
          itemName: annotation.text,
          worldX: annotation.x,
          worldY: annotation.y,
          worldZ: annotation.z,
          screenX: point.x,
          screenY: point.y
        }
      });
    }

    return targets;
  }
}

function createVolumeActor(
  volume: VolumeInterpretationVolume,
  resource: SyntheticVolumeResource
): vtkVolume {
  const mapper = vtkVolumeMapper.newInstance();
  mapper.setInputData(resource.imageData);
  mapper.setSampleDistance(
    Math.max(
      span(volume.bounds.minX, volume.bounds.maxX, volume.dimensions.inline),
      span(volume.bounds.minY, volume.bounds.maxY, volume.dimensions.xline),
      span(volume.bounds.minZ, volume.bounds.maxZ, volume.dimensions.sample)
    ) * 0.75
  );

  const actor = vtkVolume.newInstance();
  actor.setMapper(mapper);

  const colorTransfer = createColorTransferFunction(
    volume.displayDefaults?.colormap ?? "red-white-blue",
    resource.range[0],
    resource.range[1]
  );
  const opacityTransfer = vtkPiecewiseFunction.newInstance();
  opacityTransfer.addPoint(resource.range[0], 0);
  opacityTransfer.addPoint(resource.range[0] * 0.35, volume.displayDefaults?.opacity ? volume.displayDefaults.opacity * 0.025 : 0.02);
  opacityTransfer.addPoint(0, 0);
  opacityTransfer.addPoint(resource.range[1] * 0.35, volume.displayDefaults?.opacity ? volume.displayDefaults.opacity * 0.025 : 0.02);
  opacityTransfer.addPoint(resource.range[1], 0);

  actor.getProperty().setRGBTransferFunction(0, colorTransfer);
  actor.getProperty().setScalarOpacity(0, opacityTransfer);
  actor.getProperty().setInterpolationTypeToFastLinear();
  actor.getProperty().setScalarOpacityUnitDistance(0, 2);
  return actor;
}

function createSliceActor(
  plane: VolumeInterpretationSlicePlane,
  volume: VolumeInterpretationVolume,
  resource: SyntheticVolumeResource
): { actor: vtkImageSlice; borderActor: vtkActor } {
  const mapper = vtkImageMapper.newInstance();
  mapper.setInputData(resource.imageData);
  const sliceIndex = worldToIndex(plane.axis, plane.position, volume);
  if (plane.axis === "inline") {
    mapper.setISlice(sliceIndex);
  } else if (plane.axis === "xline") {
    mapper.setJSlice(sliceIndex);
  } else {
    mapper.setKSlice(sliceIndex);
  }

  const actor = vtkImageSlice.newInstance();
  actor.setMapper(mapper);
  actor.getProperty().setInterpolationTypeToLinear();
  actor.getProperty().setOpacity(plane.style.opacity);
  actor.getProperty().setUseLookupTableScalarRange(true);
  actor.getProperty().setRGBTransferFunction(
    0,
    createColorTransferFunction(plane.style.colormap, resource.range[0], resource.range[1])
  );

  const borderActor = createPlaneBorderActor(plane, volume);
  return { actor, borderActor };
}

function createPlaneBorderActor(
  plane: VolumeInterpretationSlicePlane,
  volume: VolumeInterpretationVolume
): vtkActor {
  const corners = slicePlaneCorners(plane, volume);
  const points = vtkPoints.newInstance();
  points.setData(
    new Float32Array(corners.flatMap((point) => [point.x, point.y, point.z])),
    3
  );

  const lines = vtkCellArray.newInstance();
  lines.insertNextCell([0, 1, 2, 3, 0]);

  const polyData = vtkPolyData.newInstance();
  polyData.setPoints(points);
  polyData.setLines(lines);

  const mapper = vtkMapper.newInstance();
  mapper.setInputData(polyData);

  const actor = vtkActor.newInstance();
  actor.setMapper(mapper);
  actor.getProperty().setColor([0.95, 0.97, 0.99]);
  actor.getProperty().setOpacity(plane.style.showBorder === false ? 0 : 0.9);
  actor.getProperty().setLineWidth(1.4);
  actor.getProperty().setLighting(false);
  actor.setPickable(false);
  return actor;
}

function createHorizonActors(
  horizon: VolumeInterpretationHorizonSurface
): { fillActor: vtkActor; wireActor: vtkActor } {
  const points = vtkPoints.newInstance();
  points.setData(horizon.points, 3);

  const polys = vtkCellArray.newInstance();
  for (let row = 0; row < horizon.rows - 1; row += 1) {
    for (let column = 0; column < horizon.columns - 1; column += 1) {
      const a = row * horizon.columns + column;
      const b = a + 1;
      const c = (row + 1) * horizon.columns + column + 1;
      const d = c - 1;
      polys.insertNextCell([a, b, c]);
      polys.insertNextCell([a, c, d]);
    }
  }

  const polyData = vtkPolyData.newInstance();
  polyData.setPoints(points);
  polyData.setPolys(polys);

  if (horizon.colorValues) {
    polyData.getPointData().setScalars(
      vtkDataArray.newInstance({
        name: `${horizon.id}-scalar`,
        values: horizon.colorValues,
        numberOfComponents: 1
      })
    );
  }

  const fillMapper = vtkMapper.newInstance();
  fillMapper.setInputData(polyData);
  if (horizon.colorValues) {
    const range = minMax(horizon.colorValues);
    fillMapper.setScalarModeToUsePointData();
    fillMapper.setScalarVisibility(true);
    fillMapper.setScalarRange(range[0], range[1]);
    fillMapper.setLookupTable(createSurfaceLookupTable(range[0], range[1]));
  } else {
    fillMapper.setScalarVisibility(false);
  }

  const fillActor = vtkActor.newInstance();
  fillActor.setMapper(fillMapper);
  fillActor.getProperty().setColor(colorToRgb(horizon.style.fillColor ?? "#4cc9f0"));
  fillActor.getProperty().setOpacity(horizon.style.fillOpacity);
  fillActor.getProperty().setAmbient(0.35);
  fillActor.getProperty().setDiffuse(0.75);

  const wireMapper = vtkMapper.newInstance();
  wireMapper.setInputData(polyData);
  wireMapper.setScalarVisibility(false);

  const wireActor = vtkActor.newInstance();
  wireActor.setMapper(wireMapper);
  wireActor.getProperty().setRepresentationToWireframe();
  wireActor.getProperty().setColor(colorToRgb(horizon.style.edgeColor ?? "#173042"));
  wireActor.getProperty().setOpacity(horizon.style.showContours ? 0.82 : 0.48);
  wireActor.getProperty().setLineWidth(horizon.style.edgeWidth ?? 1.2);
  wireActor.getProperty().setLighting(false);

  return { fillActor, wireActor };
}

function createWellActor(well: VolumeInterpretationWellTrajectory): vtkActor {
  const points = vtkPoints.newInstance();
  points.setData(well.points, 3);

  const lineIds = Array.from({ length: well.points.length / 3 }, (_, index) => index);
  const lines = vtkCellArray.newInstance();
  lines.insertNextCell(lineIds);

  const polyData = vtkPolyData.newInstance();
  polyData.setPoints(points);
  polyData.setLines(lines);

  const mapper = vtkMapper.newInstance();
  const actor = vtkActor.newInstance();

  if (well.style.mode === "tube") {
    const tube = vtkTubeFilter.newInstance({
      radius: Math.max(4, well.style.width * 2.2),
      numberOfSides: 16,
      capping: true
    });
    tube.setInputData(polyData);
    mapper.setInputConnection(tube.getOutputPort());
  } else {
    mapper.setInputData(polyData);
  }

  actor.setMapper(mapper);
  actor.getProperty().setColor(colorToRgb(well.style.color));
  actor.getProperty().setLineWidth(Math.max(2, well.style.width));
  actor.getProperty().setAmbient(0.4);
  actor.getProperty().setDiffuse(0.7);
  return actor;
}

function createMarkerActor(marker: VolumeInterpretationMarker): vtkActor {
  const sphere = vtkSphereSource.newInstance({
    center: [marker.x, marker.y, marker.z],
    radius: Math.max(10, marker.size * 1.8),
    thetaResolution: 18,
    phiResolution: 18
  });
  const mapper = vtkMapper.newInstance();
  mapper.setInputConnection(sphere.getOutputPort());

  const actor = vtkActor.newInstance();
  actor.setMapper(mapper);
  actor.getProperty().setColor(colorToRgb(marker.color));
  actor.getProperty().setAmbient(0.45);
  actor.getProperty().setDiffuse(0.75);
  return actor;
}

function createAnnotationActor(annotation: VolumeInterpretationAnnotation): vtkActor {
  const sphere = vtkSphereSource.newInstance({
    center: [annotation.x, annotation.y, annotation.z],
    radius: 12,
    thetaResolution: 14,
    phiResolution: 14
  });
  const mapper = vtkMapper.newInstance();
  mapper.setInputConnection(sphere.getOutputPort());

  const actor = vtkActor.newInstance();
  actor.setMapper(mapper);
  actor.getProperty().setColor(colorToRgb(annotation.color ?? "#ddeaf0"));
  actor.getProperty().setAmbient(0.5);
  actor.getProperty().setDiffuse(0.6);
  return actor;
}

function createBoundsActor(
  bounds: VolumeInterpretationBounds,
  color: [number, number, number],
  opacity: number
): vtkActor {
  const cube = vtkCubeSource.newInstance({
    center: [
      (bounds.minX + bounds.maxX) / 2,
      (bounds.minY + bounds.maxY) / 2,
      (bounds.minZ + bounds.maxZ) / 2
    ],
    xLength: bounds.maxX - bounds.minX,
    yLength: bounds.maxY - bounds.minY,
    zLength: bounds.maxZ - bounds.minZ
  });

  const mapper = vtkMapper.newInstance();
  mapper.setInputConnection(cube.getOutputPort());

  const actor = vtkActor.newInstance();
  actor.setMapper(mapper);
  actor.getProperty().setRepresentationToWireframe();
  actor.getProperty().setColor(color);
  actor.getProperty().setOpacity(opacity);
  actor.getProperty().setLineWidth(1.2);
  actor.getProperty().setLighting(false);
  actor.setPickable(false);
  return actor;
}

function updateCamera(renderer: vtkRenderer, bounds: VolumeInterpretationBounds, view: VolumeInterpretationView): void {
  const camera = renderer.getActiveCamera();
  const yaw = (view.yawDeg * Math.PI) / 180;
  const pitch = (view.pitchDeg * Math.PI) / 180;
  const radius =
    Math.max(bounds.maxX - bounds.minX, bounds.maxY - bounds.minY, bounds.maxZ - bounds.minZ, 1) * (2.15 / Math.max(0.35, view.zoom));

  const offsetX = Math.cos(yaw) * Math.cos(pitch) * radius;
  const offsetY = Math.sin(yaw) * Math.cos(pitch) * radius;
  const offsetZ = Math.sin(pitch) * radius;

  camera.setPosition(view.focusX + offsetX, view.focusY + offsetY, view.focusZ + offsetZ);
  camera.setFocalPoint(view.focusX, view.focusY, view.focusZ);
  camera.setViewUp(0, 0, 1);
}

function projectPoint(
  renderer: vtkRenderer,
  width: number,
  height: number,
  x: number,
  y: number,
  z: number
): Point2D {
  const [displayX, displayY] = renderer.worldToNormalizedDisplay(x, y, z, width / Math.max(1, height));
  return {
    x: displayX * width,
    y: (1 - displayY) * height
  };
}

function synthesizeVolumeValues(volume: VolumeInterpretationVolume): Float32Array {
  const { inline, xline, sample } = volume.dimensions;
  const values = new Float32Array(inline * xline * sample);
  let offset = 0;

  for (let k = 0; k < sample; k += 1) {
    const zn = sample > 1 ? k / (sample - 1) - 0.5 : 0;
    for (let j = 0; j < xline; j += 1) {
      const yn = xline > 1 ? j / (xline - 1) - 0.5 : 0;
      for (let i = 0; i < inline; i += 1) {
        const xn = inline > 1 ? i / (inline - 1) - 0.5 : 0;
        const folded = Math.sin(zn * 28 + xn * 6 + Math.sin(yn * 7) * 1.8);
        const stratigraphy = Math.sin(zn * 44 + xn * 9 - yn * 5);
        const channel = Math.exp(-((xn - yn * 0.18) * (xn - yn * 0.18) * 22 + (zn + 0.12) * (zn + 0.12) * 85));
        const diapir = Math.exp(-((xn + 0.05) * (xn + 0.05) * 36 + (yn - 0.08) * (yn - 0.08) * 28)) * Math.cos(zn * 18);
        values[offset] = folded * 0.55 + stratigraphy * 0.32 - channel * 0.65 + diapir * 0.42;
        offset += 1;
      }
    }
  }

  return values;
}

function createColorTransferFunction(
  colorMap: VolumeInterpretationColorMap,
  minValue: number,
  maxValue: number
): vtkColorTransferFunction {
  const transfer = vtkColorTransferFunction.newInstance();
  if (colorMap === "grayscale") {
    transfer.addRGBPoint(minValue, 0.06, 0.08, 0.1);
    transfer.addRGBPoint(maxValue, 0.96, 0.97, 0.98);
    return transfer;
  }

  transfer.addRGBPoint(minValue, 0.16, 0.32, 0.84);
  transfer.addRGBPoint(0, 0.98, 0.98, 0.98);
  transfer.addRGBPoint(maxValue, 0.84, 0.18, 0.18);
  return transfer;
}

function createSurfaceLookupTable(minValue: number, maxValue: number): vtkColorTransferFunction {
  const transfer = vtkColorTransferFunction.newInstance();
  transfer.addRGBPoint(minValue, 0.82, 0.05, 0.12);
  transfer.addRGBPoint(minValue + (maxValue - minValue) * 0.33, 0.92, 0.72, 0.12);
  transfer.addRGBPoint(minValue + (maxValue - minValue) * 0.66, 0.18, 0.78, 0.46);
  transfer.addRGBPoint(maxValue, 0.08, 0.34, 0.88);
  return transfer;
}

function colorToRgb(color: string): [number, number, number] {
  if (color.startsWith("#")) {
    const normalized =
      color.length === 4
        ? `#${color[1]}${color[1]}${color[2]}${color[2]}${color[3]}${color[3]}`
        : color;
    return [
      parseInt(normalized.slice(1, 3), 16) / 255,
      parseInt(normalized.slice(3, 5), 16) / 255,
      parseInt(normalized.slice(5, 7), 16) / 255
    ];
  }
  return [0.8, 0.86, 0.9];
}

function span(min: number, max: number, count: number): number {
  return count > 1 ? (max - min) / (count - 1) : 1;
}

function worldToIndex(
  axis: VolumeInterpretationSlicePlane["axis"],
  worldPosition: number,
  volume: VolumeInterpretationVolume
): number {
  const bounds =
    axis === "inline"
      ? [volume.bounds.minX, volume.bounds.maxX, volume.dimensions.inline]
      : axis === "xline"
        ? [volume.bounds.minY, volume.bounds.maxY, volume.dimensions.xline]
        : [volume.bounds.minZ, volume.bounds.maxZ, volume.dimensions.sample];
  const [min, max, count] = bounds;
  const ratio = (worldPosition - min) / Math.max(1e-6, max - min);
  return Math.max(0, Math.min(count - 1, Math.round(ratio * (count - 1))));
}

function slicePlaneCorners(
  plane: VolumeInterpretationSlicePlane,
  volume: VolumeInterpretationVolume
): Array<{ x: number; y: number; z: number }> {
  switch (plane.axis) {
    case "inline":
      return [
        { x: plane.position, y: volume.bounds.minY, z: volume.bounds.minZ },
        { x: plane.position, y: volume.bounds.maxY, z: volume.bounds.minZ },
        { x: plane.position, y: volume.bounds.maxY, z: volume.bounds.maxZ },
        { x: plane.position, y: volume.bounds.minY, z: volume.bounds.maxZ }
      ];
    case "xline":
      return [
        { x: volume.bounds.minX, y: plane.position, z: volume.bounds.minZ },
        { x: volume.bounds.maxX, y: plane.position, z: volume.bounds.minZ },
        { x: volume.bounds.maxX, y: plane.position, z: volume.bounds.maxZ },
        { x: volume.bounds.minX, y: plane.position, z: volume.bounds.maxZ }
      ];
    default:
      return [
        { x: volume.bounds.minX, y: volume.bounds.minY, z: plane.position },
        { x: volume.bounds.maxX, y: volume.bounds.minY, z: plane.position },
        { x: volume.bounds.maxX, y: volume.bounds.maxY, z: plane.position },
        { x: volume.bounds.minX, y: volume.bounds.maxY, z: plane.position }
      ];
  }
}

function horizonPerimeter(
  horizon: VolumeInterpretationHorizonSurface
): Array<{ x: number; y: number; z: number }> {
  const points: Array<{ x: number; y: number; z: number }> = [];
  for (let column = 0; column < horizon.columns; column += 1) {
    points.push(horizonPoint(horizon, 0, column));
  }
  for (let row = 1; row < horizon.rows; row += 1) {
    points.push(horizonPoint(horizon, row, horizon.columns - 1));
  }
  for (let column = horizon.columns - 2; column >= 0; column -= 1) {
    points.push(horizonPoint(horizon, horizon.rows - 1, column));
  }
  for (let row = horizon.rows - 2; row > 0; row -= 1) {
    points.push(horizonPoint(horizon, row, 0));
  }
  return points;
}

function horizonPoint(
  horizon: VolumeInterpretationHorizonSurface,
  row: number,
  column: number
): { x: number; y: number; z: number } {
  const index = row * horizon.columns + column;
  return {
    x: horizon.points[index * 3]!,
    y: horizon.points[index * 3 + 1]!,
    z: horizon.points[index * 3 + 2]!
  };
}

function polygonCenter(points: Point2D[]): Point2D {
  const total = points.reduce(
    (accumulator, point) => ({
      x: accumulator.x + point.x,
      y: accumulator.y + point.y
    }),
    { x: 0, y: 0 }
  );
  return {
    x: total.x / Math.max(1, points.length),
    y: total.y / Math.max(1, points.length)
  };
}

function minMax(values: Float32Array): [number, number] {
  let min = Number.POSITIVE_INFINITY;
  let max = Number.NEGATIVE_INFINITY;
  for (const value of values) {
    min = Math.min(min, value);
    max = Math.max(max, value);
  }
  return [min, max];
}

function pickScore(target: ProjectedTarget, screenX: number, screenY: number): number | null {
  if (target.type === "point") {
    const distance = Math.hypot(target.point.x - screenX, target.point.y - screenY);
    return distance <= target.radius ? distance : null;
  }
  if (target.type === "polyline") {
    const distance = polylineDistance(target.points, screenX, screenY);
    return distance <= Math.max(8, target.strokeWidth + 3) ? distance : null;
  }
  if (pointInPolygon(target.points, screenX, screenY)) {
    return 0;
  }
  const distance = polygonDistance(target.points, screenX, screenY);
  return distance <= 8 ? distance : null;
}

function polylineDistance(points: Point2D[], x: number, y: number): number {
  let best = Number.POSITIVE_INFINITY;
  for (let index = 0; index < points.length - 1; index += 1) {
    best = Math.min(best, segmentDistance(points[index]!, points[index + 1]!, x, y));
  }
  return best;
}

function polygonDistance(points: Point2D[], x: number, y: number): number {
  let best = Number.POSITIVE_INFINITY;
  for (let index = 0; index < points.length; index += 1) {
    const nextIndex = (index + 1) % points.length;
    best = Math.min(best, segmentDistance(points[index]!, points[nextIndex]!, x, y));
  }
  return best;
}

function segmentDistance(a: Point2D, b: Point2D, x: number, y: number): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const lengthSquared = dx * dx + dy * dy;
  if (lengthSquared <= 1e-6) {
    return Math.hypot(x - a.x, y - a.y);
  }
  const ratio = Math.max(0, Math.min(1, ((x - a.x) * dx + (y - a.y) * dy) / lengthSquared));
  const projectedX = a.x + ratio * dx;
  const projectedY = a.y + ratio * dy;
  return Math.hypot(x - projectedX, y - projectedY);
}

function pointInPolygon(points: Point2D[], x: number, y: number): boolean {
  let inside = false;
  for (let left = 0, right = points.length - 1; left < points.length; right = left++) {
    const pointLeft = points[left]!;
    const pointRight = points[right]!;
    const intersects =
      pointLeft.y > y !== pointRight.y > y &&
      x < ((pointRight.x - pointLeft.x) * (y - pointLeft.y)) / Math.max(1e-6, pointRight.y - pointLeft.y) + pointLeft.x;
    if (intersects) {
      inside = !inside;
    }
  }
  return inside;
}
