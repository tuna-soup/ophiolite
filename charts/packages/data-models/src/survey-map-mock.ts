import type { OphioliteResolvedSurveyMapSource } from "./ophiolite-survey-map-adapter";
import { adaptOphioliteSurveyMapToChart } from "./ophiolite-survey-map-adapter";
import type { SurveyMapModel } from "./survey-map";

const GRID_COLUMNS = 56;
const GRID_ROWS = 44;
const X_MIN = 920_000;
const X_MAX = 1_048_000;
const Y_MIN = 96_000;
const Y_MAX = 106_000;

export function createMockSurveyMap(): SurveyMapModel {
  return adaptOphioliteSurveyMapToChart(createMockOphioliteSurveyMapSource());
}

export function createMockOphioliteSurveyMapSource(): OphioliteResolvedSurveyMapSource {
  const xStep = (X_MAX - X_MIN) / (GRID_COLUMNS - 1);
  const yStep = (Y_MAX - Y_MIN) / (GRID_ROWS - 1);
  const values = new Float32Array(GRID_COLUMNS * GRID_ROWS);

  for (let row = 0; row < GRID_ROWS; row += 1) {
    for (let column = 0; column < GRID_COLUMNS; column += 1) {
      const x = X_MIN + column * xStep;
      const y = Y_MIN + row * yStep;
      const nx = (x - X_MIN) / (X_MAX - X_MIN);
      const ny = (y - Y_MIN) / (Y_MAX - Y_MIN);
      const basin = 0.68 - nx * 1.35 + ny * 0.24;
      const ridge = Math.exp(-((nx - 0.34) ** 2) / 0.03 - ((ny - 0.34) ** 2) / 0.12) * 0.55;
      const flank = Math.exp(-((nx - 0.58) ** 2) / 0.014 - ((ny - 0.63) ** 2) / 0.06) * 0.42;
      const trend = Math.sin(nx * 9.5 + ny * 5.2) * 0.05 + Math.cos(nx * 3.4 - ny * 7.1) * 0.04;
      values[row * GRID_COLUMNS + column] = 1820 + basin * 120 + ridge * 95 + flank * 55 + trend * 40;
    }
  }

  return {
    id: "mock-survey-map",
    name: "Stybarrow survey map",
    x_label: "Easting",
    y_label: "Northing",
    coordinate_unit: "m",
    background: "#f4f2ee",
    surveys: [
      {
        id: "stybarrow-outline",
        name: "Stybarrow 3D",
        outline: [
          { x: 926_000, y: 96_400 },
          { x: 1_043_000, y: 96_800 },
          { x: 1_038_000, y: 105_200 },
          { x: 931_500, y: 104_700 }
        ],
        stroke: "rgba(41, 55, 68, 0.9)",
        fill: "rgba(255, 255, 255, 0.1)"
      }
    ],
    wells: [
      createWell("stybarrow-1", "Stybarrow-1", 988_500, 103_900, "#f2f5f8", -2_000, -4_400),
      createWell("stybarrow-2", "Stybarrow-2", 993_700, 101_400, "#e9eef2", 1_900, -2_700),
      createWell("stybarrow-3", "Stybarrow-3", 976_100, 98_700, "#f4f6f7", 2_500, 1_600),
      createWell("pyrenees-4", "Pyrenees-4", 1_012_000, 101_100, "#f4f6f7", -1_100, 2_800),
      createWell("pyrenees-5", "Pyrenees-5", 1_020_500, 100_000, "#f4f6f7", 1_400, 3_300),
      createWell("whale-1", "Whale-1", 1_033_000, 103_300, "#f4f6f7", -2_600, -1_500)
    ],
    scalar_field: {
      id: "mock-twt-grid",
      name: "TWT",
      columns: GRID_COLUMNS,
      rows: GRID_ROWS,
      values,
      origin: { x: X_MIN, y: Y_MIN },
      step: { x: xStep, y: yStep },
      unit: "ms",
      min_value: 1600,
      max_value: 1960
    }
  };
}

function createWell(
  id: string,
  name: string,
  x: number,
  y: number,
  color: string,
  endDx: number,
  endDy: number
) {
  const trajectory = [
    { x, y },
    { x: x + endDx * 0.24, y: y + endDy * 0.21 },
    { x: x + endDx * 0.58 + Math.sin(x / 7000) * 420, y: y + endDy * 0.57 + Math.cos(y / 6000) * 280 },
    { x: x + endDx, y: y + endDy }
  ];

  return {
    well_id: id,
    wellbore_id: id,
    name,
    surface_position: { x, y },
    plan_trajectory: trajectory,
    color
  };
}

