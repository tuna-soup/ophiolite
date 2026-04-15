export type RockPhysicsTemplateId =
  | "vp-vs-vs-ai"
  | "ai-vs-si"
  | "vp-vs-vs"
  | "porosity-vs-vp"
  | "lambda-rho-vs-mu-rho"
  | "neutron-porosity-vs-bulk-density"
  | "phi-vs-ai"
  | "pr-vs-ai"
  | "vp-vs-density";

export type RockPhysicsCurveSemantic =
  | "p-velocity"
  | "s-velocity"
  | "vp-vs-ratio"
  | "acoustic-impedance"
  | "elastic-impedance"
  | "extended-elastic-impedance"
  | "shear-impedance"
  | "lambda-rho"
  | "mu-rho"
  | "bulk-density"
  | "resistivity"
  | "sonic"
  | "shear-sonic"
  | "poissons-ratio"
  | "neutron-porosity"
  | "effective-porosity"
  | "water-saturation"
  | "v-shale"
  | "gamma-ray";

export type RockPhysicsCategoricalSemantic = "well" | "wellbore" | "facies";

export type RockPhysicsColorSemantic = RockPhysicsCurveSemantic | RockPhysicsCategoricalSemantic;
export type RockPhysicsPointSymbol = "circle" | "square" | "diamond" | "triangle";

export interface RockPhysicsAxisRange {
  min: number;
  max: number;
}

export interface RockPhysicsCrossplotViewport {
  xMin: number;
  xMax: number;
  yMin: number;
  yMax: number;
}

export interface RockPhysicsAxisDefinition {
  label: string;
  unit?: string;
  semantic: RockPhysicsCurveSemantic;
  range: RockPhysicsAxisRange;
}

export interface RockPhysicsCategoryDefinition {
  id: number;
  label: string;
  color: string;
  symbol?: RockPhysicsPointSymbol;
}

export interface RockPhysicsCategoricalColorBinding {
  kind: "categorical";
  label: string;
  semantic: RockPhysicsCategoricalSemantic;
  categories: RockPhysicsCategoryDefinition[];
}

export interface RockPhysicsContinuousColorBinding {
  kind: "continuous";
  label: string;
  semantic: RockPhysicsCurveSemantic;
  range: RockPhysicsAxisRange;
  palette: string[];
}

export type RockPhysicsColorBinding = RockPhysicsCategoricalColorBinding | RockPhysicsContinuousColorBinding;

export interface RockPhysicsWellDescriptor {
  id: string;
  wellboreId: string;
  name: string;
  color: string;
}

export interface RockPhysicsSourceBinding {
  wellId: string;
  wellboreId: string;
  xCurveId: string;
  yCurveId: string;
  colorCurveId?: string;
  derivedChannels?: string[];
}

export interface RockPhysicsTemplateLine {
  id: string;
  label: string;
  color: string;
  points: Array<{
    x: number;
    y: number;
  }>;
}

export interface RockPhysicsTemplatePolylineOverlay {
  kind: "polyline";
  id: string;
  label?: string;
  color: string;
  width?: number;
  dashed?: boolean;
  points: Array<{
    x: number;
    y: number;
  }>;
}

export interface RockPhysicsTemplatePolygonOverlay {
  kind: "polygon";
  id: string;
  label?: string;
  strokeColor?: string;
  fillColor: string;
  points: Array<{
    x: number;
    y: number;
  }>;
  labelPosition?: {
    x: number;
    y: number;
  };
}

export interface RockPhysicsTemplateTextOverlay {
  kind: "text";
  id: string;
  text: string;
  color: string;
  x: number;
  y: number;
  rotationDeg?: number;
  align?: "left" | "center" | "right";
  baseline?: "top" | "middle" | "bottom";
}

export type RockPhysicsTemplateOverlay =
  | RockPhysicsTemplatePolylineOverlay
  | RockPhysicsTemplatePolygonOverlay
  | RockPhysicsTemplateTextOverlay;

export interface RockPhysicsPointColumns {
  x: Float32Array;
  y: Float32Array;
  colorScalars?: Float32Array;
  colorCategoryIds?: Uint16Array;
  symbolCategoryIds?: Uint16Array;
  wellIndices: Uint16Array;
  sourceBindingIndices?: Uint16Array;
  sampleDepthsM: Float32Array;
}

export interface RockPhysicsInteractionThresholds {
  exactPointLimit: number;
  progressivePointLimit: number;
}

export interface RockPhysicsCrossplotProbe {
  pointIndex: number;
  wellId: string;
  wellName: string;
  xValue: number;
  yValue: number;
  colorValue?: number;
  colorCategoryLabel?: string;
  sampleDepthM: number;
  screenX: number;
  screenY: number;
}

export type RockPhysicsInteractionQuality = "auto" | "exact" | "progressive";
export type RockPhysicsRenderQuality = "auto" | "quality" | "performance";

export interface RockPhysicsCrossplotModel {
  id: string;
  name: string;
  templateId: RockPhysicsTemplateId;
  title: string;
  subtitle?: string;
  pointCount: number;
  xAxis: RockPhysicsAxisDefinition;
  yAxis: RockPhysicsAxisDefinition;
  colorBinding: RockPhysicsColorBinding;
  columns: RockPhysicsPointColumns;
  wells: RockPhysicsWellDescriptor[];
  sourceBindings: RockPhysicsSourceBinding[];
  templateLines?: RockPhysicsTemplateLine[];
  templateOverlays?: RockPhysicsTemplateOverlay[];
  interactionThresholds?: RockPhysicsInteractionThresholds;
}
