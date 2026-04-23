import assert from "node:assert/strict";
import test from "node:test";
import { adaptRockPhysicsCrossplotInputToModel } from "../src/rock-physics-public-model";
import { adaptSurveyMapInputToModel } from "../src/survey-map-public-model";
import { adaptWellCorrelationPanelInputToModel } from "../src/well-correlation-public-model";
import type {
  RockPhysicsCrossplotData,
  SurveyMapData,
  WellCorrelationPanelData
} from "../src/types";

test("survey map simple data normalizes to the internal survey map model", () => {
  const map: SurveyMapData = {
    name: "North Survey",
    xLabel: "Easting",
    yLabel: "Northing",
    coordinateUnit: "m",
    areas: [
      {
        name: "North Outline",
        points: [
          { x: 120, y: 160 },
          { x: 2060, y: 180 },
          { x: 2120, y: 1540 },
          { x: 180, y: 1620 }
        ],
        stroke: "#274f44"
      }
    ],
    wells: [
      {
        name: "Well A",
        position: { x: 420, y: 480 },
        trajectory: [
          { x: 420, y: 480 },
          { x: 520, y: 620 }
        ],
        color: "#0e7490"
      }
    ]
  };

  const normalized = adaptSurveyMapInputToModel(map);

  assert.ok(normalized);
  assert.equal(normalized.name, "North Survey");
  assert.equal(normalized.surveys.length, 1);
  assert.equal(normalized.surveys[0]?.name, "North Outline");
  assert.deepEqual(normalized.surveys[0]?.outline[0], { x: 120, y: 160 });
  assert.equal(normalized.wells.length, 1);
  assert.equal(normalized.wells[0]?.name, "Well A");
  assert.deepEqual(normalized.wells[0]?.surface, { x: 420, y: 480 });
  assert.deepEqual(normalized.wells[0]?.trajectory?.[1], { x: 520, y: 620 });
});

test("rock physics simple data normalizes to the internal crossplot model", () => {
  const model: RockPhysicsCrossplotData = {
    templateId: "vp-vs-vs-ai",
    title: "Vp/Vs vs AI",
    groups: [
      { name: "Well A", color: "#0f766e" },
      { name: "Well B", color: "#b45309", symbol: "diamond" }
    ],
    points: [
      { x: 5850, y: 1.62, group: "Well A", depthM: 2410 },
      { x: 6120, y: 1.68, group: "Well A", depthM: 2422 },
      { x: 7180, y: 1.89, group: "Well B", depthM: 2462 },
      { x: 7560, y: 1.96, group: "Well B", depthM: 2474 }
    ]
  };

  const normalized = adaptRockPhysicsCrossplotInputToModel(model);

  assert.ok(normalized);
  assert.equal(normalized.templateId, "vp-vs-vs-ai");
  assert.equal(normalized.pointCount, 4);
  assert.equal(normalized.colorBinding.kind, "categorical");
  assert.equal(normalized.wells.length, 2);
  assert.equal(normalized.wells[0]?.name, "Well A");
  assert.equal(normalized.wells[1]?.name, "Well B");
  assert.deepEqual([...normalized.columns.x], [5850, 6120, 7180, 7560]);
  assert.deepEqual(
    [...normalized.columns.y].map((value) => Number(value.toFixed(2))),
    [1.62, 1.68, 1.89, 1.96]
  );
  assert.deepEqual([...normalized.columns.wellIndices], [0, 0, 1, 1]);
  assert.deepEqual([...normalized.columns.sampleDepthsM], [2410, 2422, 2462, 2474]);
});

test("well correlation simple data normalizes to the internal panel model", () => {
  const panel: WellCorrelationPanelData = {
    name: "Well Correlation",
    depthDomain: {
      start: 1500,
      end: 1620,
      unit: "m",
      label: "MD"
    },
    wells: [
      {
        name: "Well A",
        depthDatum: "md",
        curves: [
          {
            name: "GR",
            color: "#1f2937",
            values: Float32Array.from([72, 86, 102]),
            depths: Float32Array.from([1500, 1520, 1540]),
            unit: "API",
            axis: {
              min: 0,
              max: 180,
              label: "GR",
              unit: "API"
            }
          }
        ],
        tops: [{ name: "Reservoir Top", depth: 1540, color: "#b45309" }]
      }
    ]
  };

  const normalized = adaptWellCorrelationPanelInputToModel(panel);

  assert.ok(normalized);
  assert.equal(normalized.name, "Well Correlation");
  assert.equal(normalized.wells.length, 1);
  assert.equal(normalized.wells[0]?.panelDepthMapping.length, 3);
  assert.equal(normalized.wells[0]?.tracks.length, 3);
  assert.equal(normalized.wells[0]?.tracks[0]?.kind, "reference");
  assert.equal(normalized.wells[0]?.tracks[1]?.kind, "curve");
  assert.equal(normalized.wells[0]?.tracks[2]?.kind, "tops");
  assert.equal(normalized.wells[0]?.tops[0]?.nativeDepth, 1540);
});
