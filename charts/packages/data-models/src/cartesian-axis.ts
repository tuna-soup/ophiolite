export type CartesianAxisId = "x" | "y";

export type CartesianAxisContextTrigger = "contextmenu";

export interface CartesianAxisOverride {
  label?: string;
  unit?: string;
  min?: number;
  max?: number;
  tickCount?: number;
  tickFormat?: string;
}

export interface CartesianAxisOverrides {
  x?: CartesianAxisOverride;
  y?: CartesianAxisOverride;
}
