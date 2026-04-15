import { AVO_ANALYSIS_CONTRACT_VERSION } from "@ophiolite/contracts";
import {
  adaptOphioliteAvoChiProjectionToChart,
  adaptOphioliteAvoCrossplotToChart,
  adaptOphioliteAvoResponseToChart,
  type OphioliteResolvedAvoChiProjectionSource,
  type OphioliteResolvedAvoCrossplotSource,
  type OphioliteResolvedAvoResponseSource
} from "./ophiolite-avo-adapter";
import type { AvoChiProjectionModel, AvoCrossplotModel, AvoResponseModel } from "./avo";

export interface AvoMockOptions {
  sampleCountPerInterface?: number;
  chiAngleDeg?: number;
}

interface MockInterface {
  id: string;
  label: string;
  reservoirLabel: string;
  color: string;
  interceptMean: number;
  gradientMean: number;
  spreadX: number;
  spreadY: number;
}

const MOCK_INTERFACES: readonly MockInterface[] = [
  {
    id: "soft-shale-brine",
    label: "SOFT SHALE on BRINE RESERVOIR",
    reservoirLabel: "Brine",
    color: "#8fb2ff",
    interceptMean: 0.085,
    gradientMean: -0.16,
    spreadX: 0.03,
    spreadY: 0.08
  },
  {
    id: "soft-shale-oil",
    label: "SOFT SHALE on OIL RESERVOIR",
    reservoirLabel: "Oil",
    color: "#1dd75f",
    interceptMean: 0.03,
    gradientMean: -0.2,
    spreadX: 0.035,
    spreadY: 0.085
  },
  {
    id: "soft-shale-gas",
    label: "SOFT SHALE on GAS RESERVOIR",
    reservoirLabel: "Gas",
    color: "#ff9863",
    interceptMean: -0.02,
    gradientMean: -0.24,
    spreadX: 0.04,
    spreadY: 0.09
  }
] as const;

const CHI_DEFAULT_DEG = 35;
const RESPONSE_ANGLES_DEG = Array.from({ length: 21 }, (_, index) => index * 2);

export function createMockAvoResponseModel(options: AvoMockOptions = {}): AvoResponseModel {
  return adaptOphioliteAvoResponseToChart(createMockOphioliteAvoResponseSource(options));
}

export function createMockAvoCrossplotModel(options: AvoMockOptions = {}): AvoCrossplotModel {
  return adaptOphioliteAvoCrossplotToChart(createMockOphioliteAvoCrossplotSource(options));
}

export function createMockAvoChiProjectionModel(options: AvoMockOptions = {}): AvoChiProjectionModel {
  return adaptOphioliteAvoChiProjectionToChart(createMockOphioliteAvoChiProjectionSource(options));
}

export function createMockOphioliteAvoResponseSource(options: AvoMockOptions = {}): OphioliteResolvedAvoResponseSource {
  const interfaces = createInterfaceDtos();

  return {
    schema_version: AVO_ANALYSIS_CONTRACT_VERSION,
    id: "mock-avo-response",
    name: "Mock AVO Response",
    title: "AVO Response",
    subtitle: "Modeled isotropic and anisotropic interface reflectivity",
    x_axis: {
      label: "Incident Angle",
      unit: "deg",
      min_value: 0,
      max_value: RESPONSE_ANGLES_DEG[RESPONSE_ANGLES_DEG.length - 1] ?? 40
    },
    y_axis: {
      label: "Real",
      unit: null,
      min_value: -0.35,
      max_value: 0.2
    },
    interfaces,
    series: MOCK_INTERFACES.flatMap((entry, index) => {
      const phaseShift = index * 0.11;
      return [
        {
          id: `${entry.id}:anisotropic`,
          interface_id: entry.id,
          label: `${entry.label}, Anisotropic`,
          color: entry.color,
          style: "solid",
          reflectivity_model: "ruger",
          anisotropy_mode: "vti",
          incidence_angles_deg: [...RESPONSE_ANGLES_DEG],
          values: RESPONSE_ANGLES_DEG.map((angleDeg) => responseValue(entry, angleDeg, phaseShift, true))
        },
        {
          id: `${entry.id}:isotropic`,
          interface_id: entry.id,
          label: `${entry.label}, Isotropic`,
          color: entry.color,
          style: "dashed",
          reflectivity_model: "shuey_three_term",
          anisotropy_mode: "isotropic",
          incidence_angles_deg: [...RESPONSE_ANGLES_DEG],
          values: RESPONSE_ANGLES_DEG.map((angleDeg) => responseValue(entry, angleDeg, phaseShift, false))
        }
      ];
    })
  };
}

export function createMockOphioliteAvoCrossplotSource(options: AvoMockOptions = {}): OphioliteResolvedAvoCrossplotSource {
  const sampleCountPerInterface = Math.max(120, options.sampleCountPerInterface ?? 620);
  const random = mulberry32(20_260_415);
  const points: OphioliteResolvedAvoCrossplotSource["points"] = [];

  MOCK_INTERFACES.forEach((entry, interfaceIndex) => {
    for (let sampleIndex = 0; sampleIndex < sampleCountPerInterface; sampleIndex += 1) {
      const intercept = entry.interceptMean + gaussian(random) * entry.spreadX;
      const gradient =
        entry.gradientMean +
        gaussian(random) * entry.spreadY -
        (intercept - entry.interceptMean) * (0.8 + interfaceIndex * 0.15);

      points.push({
        interface_id: entry.id,
        intercept,
        gradient,
        chi_projection: projectChi(intercept, gradient, options.chiAngleDeg ?? CHI_DEFAULT_DEG),
        simulation_id: sampleIndex % 4 === 0 ? interfaceIndex * 10_000 + sampleIndex : null
      });
    }
  });

  return {
    schema_version: AVO_ANALYSIS_CONTRACT_VERSION,
    id: "mock-avo-crossplot",
    name: "Mock AVO Intercept-Gradient",
    title: "AVO Cross-plot",
    subtitle: "Synthetic interface populations for feasibility analysis",
    x_axis: {
      label: "Intercept",
      unit: null,
      min_value: -0.3,
      max_value: 0.2
    },
    y_axis: {
      label: "Gradient",
      unit: null,
      min_value: -0.75,
      max_value: 0.75
    },
    interfaces: createInterfaceDtos(),
    points,
    reference_lines: [
      {
        id: "avo-class-trend",
        label: "Class trend",
        color: "rgba(48, 61, 78, 0.58)",
        style: "solid",
        x1: -0.18,
        y1: 0.75,
        x2: 0.18,
        y2: -0.6
      }
    ],
    background_regions: [
      {
        id: "class-iv",
        label: "IV",
        fill_color: "rgba(246, 233, 214, 0.42)",
        x_min: -0.3,
        x_max: 0,
        y_min: 0,
        y_max: 0.75
      },
      {
        id: "class-iii",
        label: "III",
        fill_color: "rgba(210, 178, 227, 0.26)",
        x_min: -0.3,
        x_max: 0,
        y_min: -0.75,
        y_max: 0
      },
      {
        id: "class-iip",
        label: "IIp",
        fill_color: "rgba(204, 243, 206, 0.3)",
        x_min: 0,
        x_max: 0.2,
        y_min: -0.75,
        y_max: 0
      },
      {
        id: "class-i",
        label: "I",
        fill_color: "rgba(221, 238, 255, 0.3)",
        x_min: 0,
        x_max: 0.2,
        y_min: 0,
        y_max: 0.75
      }
    ]
  };
}

export function createMockOphioliteAvoChiProjectionSource(
  options: AvoMockOptions = {}
): OphioliteResolvedAvoChiProjectionSource {
  const chiAngleDeg = options.chiAngleDeg ?? CHI_DEFAULT_DEG;
  const crossplot = createMockOphioliteAvoCrossplotSource(options);

  const projectedByInterface = new Map<string, number[]>();
  crossplot.points.forEach((point) => {
    const projected = point.chi_projection ?? projectChi(point.intercept, point.gradient, chiAngleDeg);
    const values = projectedByInterface.get(point.interface_id) ?? [];
    values.push(projected);
    projectedByInterface.set(point.interface_id, values);
  });

  return {
    schema_version: AVO_ANALYSIS_CONTRACT_VERSION,
    id: "mock-avo-chi-projection",
    name: "Mock AVO Chi Projection",
    title: "AVO Weighted-Stack Plot",
    subtitle: "Synthetic chi-projection populations for histogram comparison",
    chi_angle_deg: chiAngleDeg,
    projection_label: `Chi Projection (${chiAngleDeg.toFixed(1)} deg)`,
    x_axis: {
      label: "Weighted Stack",
      unit: null,
      min_value: -0.5,
      max_value: 1.25
    },
    interfaces: createInterfaceDtos(),
    series: MOCK_INTERFACES.map((entry) => {
      const projectedValues = projectedByInterface.get(entry.id) ?? [];
      return {
        id: `${entry.id}:chi`,
        interface_id: entry.id,
        label: entry.label,
        color: entry.color,
        projected_values: projectedValues,
        mean_value: average(projectedValues)
      };
    }),
    preferred_bin_count: 24
  };
}

function createInterfaceDtos(): OphioliteResolvedAvoResponseSource["interfaces"] {
  return MOCK_INTERFACES.map((entry) => ({
    id: entry.id,
    label: entry.label,
    color: entry.color,
    reservoir_label: entry.reservoirLabel
  }));
}

function responseValue(entry: MockInterface, angleDeg: number, phaseShift: number, anisotropic: boolean): number {
  const angleRad = (angleDeg * Math.PI) / 180;
  const sin2 = Math.sin(angleRad) ** 2;
  const tan2 = Math.tan(angleRad) ** 2;
  const base = entry.interceptMean + entry.gradientMean * sin2 + 0.045 * tan2 * (0.7 + phaseShift);
  if (!anisotropic) {
    return base;
  }
  return base + 0.012 * Math.sin(angleRad * 1.7 + phaseShift) - 0.004 * angleDeg / 40;
}

function projectChi(intercept: number, gradient: number, chiAngleDeg: number): number {
  const chi = (chiAngleDeg * Math.PI) / 180;
  return intercept * Math.cos(chi) + gradient * Math.sin(chi);
}

function average(values: readonly number[]): number | null {
  if (values.length === 0) {
    return null;
  }
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

function mulberry32(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state += 0x6d2b79f5;
    let value = Math.imul(state ^ (state >>> 15), state | 1);
    value ^= value + Math.imul(value ^ (value >>> 7), value | 61);
    return ((value ^ (value >>> 14)) >>> 0) / 4294967296;
  };
}

function gaussian(random: () => number): number {
  let u = 0;
  let v = 0;
  while (u === 0) {
    u = random();
  }
  while (v === 0) {
    v = random();
  }
  return Math.sqrt(-2 * Math.log(u)) * Math.cos(2 * Math.PI * v);
}
