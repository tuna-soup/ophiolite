import type { OphioliteResolvedSurveyMapSource } from "./ophiolite-survey-map-adapter";
import { adaptOphioliteSurveyMapToChart } from "./ophiolite-survey-map-adapter";
import type { SurveyMapModel } from "./survey-map";

const GRID_COLUMNS = 56;
const GRID_ROWS = 56;
const X_MIN = 2_238_400;
const X_MAX = 2_242_800;
const Y_MIN = 6_681_200;
const Y_MAX = 6_685_600;

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
      const regionalDip = (0.64 - nx) * 34 + (ny - 0.38) * 20;
      const saddle = Math.exp(-((nx - 0.46) ** 2) / 0.032 - ((ny - 0.56) ** 2) / 0.048) * 16;
      const closure = Math.exp(-((nx - 0.22) ** 2) / 0.02 - ((ny - 0.16) ** 2) / 0.03) * -24;
      const undulation = Math.sin(nx * 7.4 - ny * 3.1) * 3.8 + Math.cos(nx * 2.6 + ny * 8.2) * 2.6;
      values[row * GRID_COLUMNS + column] = 3268 + regionalDip + saddle + closure + undulation;
    }
  }

  return {
    id: "mock-survey-map",
    name: "Horizon Map of Reservoir Top",
    x_label: "Easting",
    y_label: "Northing",
    coordinate_unit: "m",
    background: "#f4f2ee",
    surveys: [
      {
        id: "stybarrow-outline",
        name: "Stybarrow 3D",
        outline: [
          { x: 2_238_780, y: 6_681_460 },
          { x: 2_242_420, y: 6_681_520 },
          { x: 2_242_120, y: 6_685_120 },
          { x: 2_239_060, y: 6_685_060 }
        ],
        stroke: "rgba(41, 55, 68, 0.9)",
        fill: "rgba(255, 255, 255, 0.1)"
      }
    ],
    wells: [
      createWell("stybarrow-1", "Stybarrow-1", 2_239_980, 6_682_360, "#f2f5f8", -180, -520),
      createWell("stybarrow-2", "Stybarrow-2", 2_240_920, 6_683_120, "#e9eef2", 220, -440),
      createWell("stybarrow-3", "Stybarrow-3", 2_239_520, 6_684_020, "#f4f6f7", 260, 340),
      createWell("pyrenees-4", "Pyrenees-4", 2_241_080, 6_684_780, "#f4f6f7", -180, 420),
      createWell("pyrenees-5", "Pyrenees-5", 2_241_760, 6_683_880, "#f4f6f7", 260, 300),
      createWell("whale-1", "Whale-1", 2_240_760, 6_682_020, "#f4f6f7", -220, 560)
    ],
    scalar_field: {
      id: "mock-reservoir-top-grid",
      name: "Reservoir Top",
      columns: GRID_COLUMNS,
      rows: GRID_ROWS,
      values,
      origin: { x: X_MIN, y: Y_MIN },
      step: { x: xStep, y: yStep },
      unit: "m",
      min_value: 3236,
      max_value: 3308
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
