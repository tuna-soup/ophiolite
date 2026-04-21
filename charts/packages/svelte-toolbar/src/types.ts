export type ToolbarIconName =
  | "pointer"
  | "crosshair"
  | "pan"
  | "orbit"
  | "sliceDrag"
  | "fitToData"
  | "topView"
  | "sideView"
  | "resetView"
  | "centerSelection"
  | "settings";

export interface ChartToolbarToolItem<TTool extends string = string> {
  id: TTool;
  label: string;
  icon: ToolbarIconName;
  active: boolean;
  disabled?: boolean;
}

export interface ChartToolbarActionItem<TAction extends string = string> {
  id: TAction;
  label: string;
  icon: ToolbarIconName;
  disabled?: boolean;
}
