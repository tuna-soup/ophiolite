import { ROCK_PHYSICS_CROSSPLOT_CONTRACT_VERSION } from "@ophiolite/contracts";
import type {
  OphioliteResolvedRockPhysicsCrossplotSource,
  OphioliteResolvedRockPhysicsWellDto
} from "./ophiolite-rock-physics-adapter";
import { adaptOphioliteRockPhysicsCrossplotToChart } from "./ophiolite-rock-physics-adapter";
import { getRockPhysicsTemplateSpec } from "./rock-physics-template-catalog";
import type {
  RockPhysicsCategoricalSemantic,
  RockPhysicsCrossplotModel,
  RockPhysicsCurveSemantic,
  RockPhysicsPointSymbol,
  RockPhysicsTemplateId
} from "./rock-physics-crossplot";

export type RockPhysicsMockColorMode =
  | RockPhysicsCategoricalSemantic
  | "water-saturation"
  | "v-shale"
  | "gamma-ray"
  | "bulk-density"
  | "neutron-porosity";

export interface RockPhysicsMockOptions {
  pointCount?: number;
  wellCount?: number;
  templateId?: RockPhysicsTemplateId;
  colorMode?: RockPhysicsMockColorMode;
  porositySemantic?: "neutron-porosity" | "effective-porosity";
}

interface GeneratedRockPhysicsSample {
  sampleDepthM: number;
  faciesId: number;
  waterSaturation: number;
  vShale: number;
  gammaRay: number;
  neutronPorosity: number;
  effectivePorosity: number;
  bulkDensity: number;
  pVelocity: number;
  sVelocity: number;
  vpVsRatio: number;
  acousticImpedance: number;
  shearImpedance: number;
  lambdaRho: number;
  muRho: number;
  poissonsRatio: number;
}

interface FaciesDefinition {
  id: number;
  label: string;
  color: string;
  symbol: RockPhysicsPointSymbol;
  porosity: number;
  waterSaturation: number;
  vShale: number;
  density: number;
  vp: number;
  vs: number;
  gammaRay: number;
}

const WELL_PALETTE = ["#d94841", "#2563eb", "#eab308", "#0f9d58", "#8b5cf6", "#0ea5e9", "#ef6c00", "#7c3aed"];
const SATURATION_PALETTE = ["#1d4ed8", "#0ea5e9", "#67e8f9", "#facc15", "#f97316", "#dc2626"];
const VSHALE_PALETTE = ["#34d399", "#84cc16", "#facc15", "#fb923c", "#ef4444"];
const GAMMARAY_PALETTE = ["#1d4ed8", "#06b6d4", "#84cc16", "#facc15", "#ef4444"];
const DENSITY_PALETTE = ["#4338ca", "#2563eb", "#06b6d4", "#f59e0b", "#dc2626"];
const POROSITY_PALETTE = ["#7c3aed", "#2563eb", "#06b6d4", "#22c55e", "#facc15"];

const FACIES_CATEGORIES: readonly FaciesDefinition[] = [
  {
    id: 0,
    label: "Gas Sand",
    color: "#f97316",
    symbol: "triangle",
    porosity: 27,
    waterSaturation: 0.18,
    vShale: 0.1,
    density: 2.18,
    vp: 3_050,
    vs: 1_760,
    gammaRay: 42
  },
  {
    id: 1,
    label: "Brine Sand",
    color: "#38bdf8",
    symbol: "circle",
    porosity: 23,
    waterSaturation: 0.92,
    vShale: 0.12,
    density: 2.28,
    vp: 3_450,
    vs: 1_980,
    gammaRay: 52
  },
  {
    id: 2,
    label: "Shale",
    color: "#84cc16",
    symbol: "square",
    porosity: 31,
    waterSaturation: 0.76,
    vShale: 0.58,
    density: 2.42,
    vp: 3_000,
    vs: 1_420,
    gammaRay: 108
  },
  {
    id: 3,
    label: "Carbonate",
    color: "#a855f7",
    symbol: "diamond",
    porosity: 11,
    waterSaturation: 0.86,
    vShale: 0.05,
    density: 2.64,
    vp: 4_900,
    vs: 2_780,
    gammaRay: 22
  }
] as const;

const SUPPORTED_CONTINUOUS_COLOR_MODES = [
  "water-saturation",
  "v-shale",
  "gamma-ray",
  "bulk-density",
  "neutron-porosity"
] as const satisfies readonly RockPhysicsMockColorMode[];
type ContinuousMockColorMode = (typeof SUPPORTED_CONTINUOUS_COLOR_MODES)[number];

export function createMockRockPhysicsCrossplotModel(
  options: RockPhysicsMockOptions = {}
): RockPhysicsCrossplotModel {
  return adaptOphioliteRockPhysicsCrossplotToChart(createMockOphioliteRockPhysicsCrossplotSource(options));
}

export function createMockOphioliteRockPhysicsCrossplotSource(
  options: RockPhysicsMockOptions = {}
): OphioliteResolvedRockPhysicsCrossplotSource {
  const pointCount = options.pointCount ?? 7_200;
  const wellCount = Math.max(1, options.wellCount ?? 6);
  const templateId = options.templateId ?? "vp-vs-vs-ai";
  const template = getRockPhysicsTemplateSpec(templateId);
  const random = mulberry32(14_042_026);
  const wells = createWells(wellCount);
  const xSemantic = resolveTemplateAxisSemantic(templateId, "x", options.porositySemantic);
  const ySemantic = resolveTemplateAxisSemantic(templateId, "y", options.porositySemantic);
  const supportedColorModes = getRockPhysicsMockColorModes(templateId);
  const colorMode = supportedColorModes.includes(options.colorMode ?? "well")
    ? (options.colorMode ?? getDefaultRockPhysicsMockColorMode(templateId))
    : getDefaultRockPhysicsMockColorMode(templateId);

  const sourceBindings = wells.map((well) => ({
    id: `${well.well_id}:binding`,
    well_id: well.well_id,
    wellbore_id: well.wellbore_id,
    x_curve_id: `${well.well_id}:${curveTokenForSemantic(xSemantic)}`,
    y_curve_id: `${well.well_id}:${curveTokenForSemantic(ySemantic)}`,
    color_curve_id: isContinuousColorMode(colorMode) ? `${well.well_id}:${curveTokenForSemantic(colorMode)}` : null,
    derived_channels: Array.from(new Set([xSemantic, ySemantic]))
  }));

  const samples: OphioliteResolvedRockPhysicsCrossplotSource["samples"] = [];
  for (let index = 0; index < pointCount; index += 1) {
    const wellIndex = index % wellCount;
    const sample = generateSample(index, wellIndex, wellCount, random);
    const xValue = valueForSemantic(sample, xSemantic);
    const yValue = valueForSemantic(sample, ySemantic);

    samples.push({
      well_id: wells[wellIndex]!.well_id,
      wellbore_id: wells[wellIndex]!.wellbore_id,
      sample_depth_m: sample.sampleDepthM,
      x_value: xValue,
      y_value: yValue,
      color_value: isContinuousColorMode(colorMode) ? valueForSemantic(sample, colorMode) : null,
      color_category_id: resolveColorCategoryId(colorMode, sample, wellIndex),
      symbol_category_id: colorMode === "facies" ? sample.faciesId : null,
      source_binding_id: sourceBindings[wellIndex]!.id
    });
  }

  const axisRanges = mockAxisRanges(templateId);

  return {
    schema_version: ROCK_PHYSICS_CROSSPLOT_CONTRACT_VERSION,
    id: `mock-rock-physics-${templateId}`,
    name: `Mock Rock Physics ${template.title}`,
    template_id: templateId,
    title: template.title,
    subtitle: `${labelForColorMode(colorMode)} across ${wellCount} wells`,
    x_axis: {
      label: template.xLabel,
      unit: template.xUnit ?? null,
      semantic: xSemantic,
      min_value: axisRanges.xMin,
      max_value: axisRanges.xMax
    },
    y_axis: {
      label: template.yLabel,
      unit: template.yUnit ?? null,
      semantic: ySemantic,
      min_value: axisRanges.yMin,
      max_value: axisRanges.yMax
    },
    color_binding: buildColorBinding(colorMode),
    wells,
    samples,
    source_bindings: sourceBindings,
    template_lines: null,
    template_overlays: null,
    interaction_thresholds: {
      exact_point_limit: 100_000,
      progressive_point_limit: 1_000_000
    }
  };
}

export function getRockPhysicsMockColorModes(templateId: RockPhysicsTemplateId): RockPhysicsMockColorMode[] {
  const template = getRockPhysicsTemplateSpec(templateId);
  const categorical = [...template.allowedCategoricalColorSemantics];
  const continuous = template.allowedContinuousColorSemantics.filter(isSupportedContinuousMockColorMode);
  return [...categorical, ...continuous];
}

export function getDefaultRockPhysicsMockColorMode(templateId: RockPhysicsTemplateId): RockPhysicsMockColorMode {
  const recommended = getRockPhysicsTemplateSpec(templateId).recommendedColorSemantic;
  return getRockPhysicsMockColorModes(templateId).includes(recommended as RockPhysicsMockColorMode)
    ? (recommended as RockPhysicsMockColorMode)
    : "well";
}

function buildColorBinding(
  colorMode: RockPhysicsMockColorMode
): OphioliteResolvedRockPhysicsCrossplotSource["color_binding"] {
  if (colorMode === "well" || colorMode === "wellbore") {
    return {
      kind: "categorical",
      semantic: colorMode,
      label: colorMode === "well" ? "Well" : "Wellbore",
      categories: null
    };
  }

  if (colorMode === "facies") {
    return {
      kind: "categorical",
      semantic: "facies",
      label: "Facies",
      categories: FACIES_CATEGORIES.map(({ id, label, color, symbol }) => ({
        id,
        label,
        color,
        symbol
      }))
    };
  }

  switch (colorMode) {
    case "water-saturation":
      return {
        kind: "continuous",
        label: "Water Saturation",
        semantic: "water-saturation",
        min_value: 0,
        max_value: 1,
        palette: [...SATURATION_PALETTE]
      };
    case "v-shale":
      return {
        kind: "continuous",
        label: "V-Shale",
        semantic: "v-shale",
        min_value: 0,
        max_value: 0.8,
        palette: [...VSHALE_PALETTE]
      };
    case "gamma-ray":
      return {
        kind: "continuous",
        label: "Gamma Ray",
        semantic: "gamma-ray",
        min_value: 0,
        max_value: 200,
        palette: [...GAMMARAY_PALETTE]
      };
    case "bulk-density":
      return {
        kind: "continuous",
        label: "Bulk Density",
        semantic: "bulk-density",
        min_value: 1.95,
        max_value: 2.95,
        palette: [...DENSITY_PALETTE]
      };
    case "neutron-porosity":
      return {
        kind: "continuous",
        label: "Neutron Porosity",
        semantic: "neutron-porosity",
        min_value: 0,
        max_value: 45,
        palette: [...POROSITY_PALETTE]
      };
  }
}

function createWells(count: number): OphioliteResolvedRockPhysicsWellDto[] {
  return Array.from({ length: count }, (_, index) => ({
    well_id: `well-${index + 1}`,
    wellbore_id: `wellbore-${index + 1}`,
    name: `Well ${6500 + index * 17}/11-${String.fromCharCode(65 + index)}`,
    color: WELL_PALETTE[index % WELL_PALETTE.length]!
  }));
}

function resolveTemplateAxisSemantic(
  templateId: RockPhysicsTemplateId,
  axis: "x" | "y",
  porositySemantic: RockPhysicsMockOptions["porositySemantic"]
): RockPhysicsCurveSemantic {
  const template = getRockPhysicsTemplateSpec(templateId);
  const semantics = axis === "x" ? template.xSemantics : template.ySemantics;
  if (templateId === "porosity-vs-vp" && axis === "x" && porositySemantic && semantics.includes(porositySemantic)) {
    return porositySemantic;
  }
  return semantics[0]!;
}

function generateSample(
  sampleIndex: number,
  wellIndex: number,
  wellCount: number,
  random: () => number
): GeneratedRockPhysicsSample {
  const facies = FACIES_CATEGORIES[pickFaciesId(random, wellIndex, wellCount)]!;
  const wellBias = wellIndex / Math.max(1, wellCount - 1);
  const depthTrend = sampleIndex / 20_000;
  const waterSaturation = clamp(facies.waterSaturation + wellBias * 0.04 + gaussian(random) * 0.07, 0.03, 1);
  const vShale = clamp(facies.vShale + gaussian(random) * 0.06 + depthTrend * 0.2, 0, 0.8);
  const neutronPorosity = clamp(facies.porosity + gaussian(random) * 3.4 + (facies.id === 2 ? 4 : 0), 2, 48);
  const effectivePorosity = clamp(neutronPorosity - vShale * 8 - waterSaturation * 0.8 + gaussian(random) * 1.1, 1, 38);
  const bulkDensity = clamp(
    facies.density - effectivePorosity * 0.012 - (1 - waterSaturation) * 0.05 + vShale * 0.08 + gaussian(random) * 0.025,
    1.95,
    2.95
  );
  const pVelocity = clamp(
    facies.vp - effectivePorosity * 32 - (1 - waterSaturation) * 160 + (1 - vShale) * 70 + gaussian(random) * 85,
    1_500,
    5_950
  );
  const sVelocity = clamp(
    facies.vs - effectivePorosity * 19 - (1 - waterSaturation) * 65 + (1 - vShale) * 45 + gaussian(random) * 55,
    850,
    3_650
  );
  const pVelocityClamped = Math.max(pVelocity, sVelocity + 180);
  const vpVsRatio = pVelocityClamped / Math.max(sVelocity, 1);
  const acousticImpedance = pVelocityClamped * bulkDensity;
  const shearImpedance = sVelocity * bulkDensity;
  const vpKm = pVelocityClamped / 1_000;
  const vsKm = sVelocity / 1_000;
  const muRho = clamp(bulkDensity * vsKm * vsKm, 5, 130);
  const lambdaRho = clamp(bulkDensity * (vpKm * vpKm - 2 * vsKm * vsKm), -30, 180);
  const poissonsRatio = clamp(
    (vpVsRatio * vpVsRatio - 2) / Math.max(0.1, 2 * (vpVsRatio * vpVsRatio - 1)),
    0.08,
    0.42
  );
  const gammaRay = clamp(facies.gammaRay + vShale * 36 + gaussian(random) * 6, 5, 200);

  return {
    sampleDepthM: 1_800 + wellIndex * 115 + sampleIndex * 0.18 + gaussian(random) * 12,
    faciesId: facies.id,
    waterSaturation,
    vShale,
    gammaRay,
    neutronPorosity,
    effectivePorosity,
    bulkDensity,
    pVelocity: pVelocityClamped,
    sVelocity,
    vpVsRatio,
    acousticImpedance,
    shearImpedance,
    lambdaRho,
    muRho,
    poissonsRatio
  };
}

function pickFaciesId(random: () => number, wellIndex: number, wellCount: number): number {
  const r = random();
  const wellBias = wellIndex / Math.max(1, wellCount - 1);
  if (wellBias < 0.2) {
    return r < 0.42 ? 0 : r < 0.78 ? 1 : r < 0.94 ? 2 : 3;
  }
  if (wellBias < 0.55) {
    return r < 0.2 ? 0 : r < 0.52 ? 1 : r < 0.88 ? 2 : 3;
  }
  return r < 0.14 ? 0 : r < 0.36 ? 1 : r < 0.72 ? 2 : 3;
}

function valueForSemantic(sample: GeneratedRockPhysicsSample, semantic: RockPhysicsCurveSemantic): number {
  switch (semantic) {
    case "p-velocity":
      return sample.pVelocity;
    case "s-velocity":
      return sample.sVelocity;
    case "vp-vs-ratio":
      return sample.vpVsRatio;
    case "acoustic-impedance":
      return sample.acousticImpedance;
    case "elastic-impedance":
      return sample.acousticImpedance;
    case "extended-elastic-impedance":
      return sample.acousticImpedance;
    case "shear-impedance":
      return sample.shearImpedance;
    case "lambda-rho":
      return sample.lambdaRho;
    case "mu-rho":
      return sample.muRho;
    case "bulk-density":
      return sample.bulkDensity;
    case "poissons-ratio":
      return sample.poissonsRatio;
    case "neutron-porosity":
      return sample.neutronPorosity;
    case "effective-porosity":
      return sample.effectivePorosity;
    case "water-saturation":
      return sample.waterSaturation;
    case "v-shale":
      return sample.vShale;
    case "gamma-ray":
      return sample.gammaRay;
    case "resistivity":
      return 2 + (1 - sample.waterSaturation) * 18 + (1 - sample.vShale) * 4;
    case "sonic":
      return 1_000_000 / Math.max(sample.pVelocity, 1);
    case "shear-sonic":
      return 1_000_000 / Math.max(sample.sVelocity, 1);
  }
}

function resolveColorCategoryId(
  colorMode: RockPhysicsMockColorMode,
  sample: GeneratedRockPhysicsSample,
  wellIndex: number
): number | null {
  switch (colorMode) {
    case "well":
    case "wellbore":
      return wellIndex;
    case "facies":
      return sample.faciesId;
    default:
      return null;
  }
}

function labelForColorMode(colorMode: RockPhysicsMockColorMode): string {
  switch (colorMode) {
    case "well":
      return "Well color";
    case "wellbore":
      return "Wellbore color";
    case "facies":
      return "Facies color";
    case "water-saturation":
      return "Water saturation color";
    case "v-shale":
      return "V-shale color";
    case "gamma-ray":
      return "Gamma ray color";
    case "bulk-density":
      return "Bulk density color";
    case "neutron-porosity":
      return "Neutron porosity color";
  }
}

function curveTokenForSemantic(semantic: RockPhysicsCurveSemantic | RockPhysicsMockColorMode): string {
  switch (semantic) {
    case "p-velocity":
      return "vp";
    case "s-velocity":
      return "vs";
    case "vp-vs-ratio":
      return "vpvs";
    case "acoustic-impedance":
      return "ai";
    case "elastic-impedance":
      return "ei";
    case "extended-elastic-impedance":
      return "eei";
    case "shear-impedance":
      return "si";
    case "lambda-rho":
      return "lambda-rho";
    case "mu-rho":
      return "mu-rho";
    case "bulk-density":
      return "rhob";
    case "resistivity":
      return "rt";
    case "sonic":
      return "dt";
    case "shear-sonic":
      return "dts";
    case "poissons-ratio":
      return "pr";
    case "neutron-porosity":
      return "nphi";
    case "effective-porosity":
      return "phie";
    case "water-saturation":
      return "sw";
    case "v-shale":
      return "vsh";
    case "gamma-ray":
      return "gr";
    case "well":
      return "well";
    case "wellbore":
      return "wellbore";
    case "facies":
      return "facies";
  }
}

function mockAxisRanges(templateId: RockPhysicsTemplateId): { xMin: number; xMax: number; yMin: number; yMax: number } {
  switch (templateId) {
    case "ai-vs-si":
      return { xMin: 5_000, xMax: 11_000, yMin: 1_800, yMax: 7_200 };
    case "vp-vs-vs":
      return { xMin: 800, xMax: 3_700, yMin: 1_500, yMax: 6_000 };
    case "porosity-vs-vp":
      return { xMin: 0, xMax: 100, yMin: 1_300, yMax: 6_000 };
    case "lambda-rho-vs-mu-rho":
      return { xMin: -30, xMax: 180, yMin: 10, yMax: 130 };
    case "neutron-porosity-vs-bulk-density":
      return { xMin: -5, xMax: 50, yMin: 1.9, yMax: 3.0 };
    case "phi-vs-ai":
      return { xMin: 5_500, xMax: 11_000, yMin: 0, yMax: 40 };
    case "pr-vs-ai":
      return { xMin: 5_500, xMax: 11_000, yMin: 0.1, yMax: 0.45 };
    case "vp-vs-density":
      return { xMin: 1.95, xMax: 2.95, yMin: 2_000, yMax: 5_500 };
    case "vp-vs-vs-ai":
    default:
      return { xMin: 5_500, xMax: 11_000, yMin: 1.3, yMax: 2.35 };
  }
}

function isContinuousColorMode(colorMode: RockPhysicsMockColorMode): colorMode is Exclude<RockPhysicsMockColorMode, RockPhysicsCategoricalSemantic> {
  return colorMode !== "well" && colorMode !== "wellbore" && colorMode !== "facies";
}

function isSupportedContinuousMockColorMode(semantic: RockPhysicsCurveSemantic): semantic is ContinuousMockColorMode {
  return SUPPORTED_CONTINUOUS_COLOR_MODES.includes(semantic as ContinuousMockColorMode);
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

function mulberry32(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let next = state;
    next = Math.imul(next ^ (next >>> 15), next | 1);
    next ^= next + Math.imul(next ^ (next >>> 7), next | 61);
    return ((next ^ (next >>> 14)) >>> 0) / 4_294_967_296;
  };
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), max);
}
