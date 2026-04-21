import { CARTESIAN_FONT_FAMILY, formatCartesianCssFont, type CartesianTextStyle } from "./cartesian-presentation";

export type ProbePanelThemeId = "light" | "dark";
export type ProbePanelSizeId = "compact" | "standard";

export interface ProbePanelFrame {
  widthPx: number;
  labelWidthPx: number;
  paddingXPx: number;
  paddingYPx: number;
  rowGapPx: number;
  columnGapPx: number;
  borderRadiusPx: number;
  insetPx: number;
}

export interface ProbePanelColors {
  border: string;
  background: string;
  shadow: string;
  text: string;
  label: string;
}

export interface ProbePanelTypography {
  row: CartesianTextStyle;
}

export interface ProbePanelPresentation {
  theme: ProbePanelThemeId;
  size: ProbePanelSizeId;
  frame: ProbePanelFrame;
  colors: ProbePanelColors;
  typography: ProbePanelTypography;
}

const SHARED_PROBE_PANEL_TYPOGRAPHY: ProbePanelTypography = {
  row: {
    family: CARTESIAN_FONT_FAMILY,
    sizePx: 12,
    weight: 500,
    lineHeight: 1.25
  }
};

const PROBE_PANEL_SIZES: Record<ProbePanelSizeId, ProbePanelFrame> = {
  compact: {
    widthPx: 188,
    labelWidthPx: 72,
    paddingXPx: 8,
    paddingYPx: 6,
    rowGapPx: 2,
    columnGapPx: 8,
    borderRadiusPx: 6,
    insetPx: 8
  },
  standard: {
    widthPx: 224,
    labelWidthPx: 80,
    paddingXPx: 10,
    paddingYPx: 8,
    rowGapPx: 2,
    columnGapPx: 8,
    borderRadiusPx: 6,
    insetPx: 10
  }
};

const PROBE_PANEL_THEMES: Record<ProbePanelThemeId, ProbePanelColors> = {
  light: {
    border: "rgba(123, 142, 161, 0.24)",
    background: "rgba(255, 255, 255, 0.96)",
    shadow: "0 10px 24px rgba(27, 39, 54, 0.12)",
    text: "#233445",
    label: "#708396"
  },
  dark: {
    border: "rgba(176, 212, 238, 0.28)",
    background: "rgba(4, 19, 29, 0.9)",
    shadow: "0 8px 18px rgba(0, 0, 0, 0.24)",
    text: "#f2f6f8",
    label: "#93aab8"
  }
};

export function resolveProbePanelPresentation(
  theme: ProbePanelThemeId,
  size: ProbePanelSizeId
): ProbePanelPresentation {
  return {
    theme,
    size,
    frame: PROBE_PANEL_SIZES[size],
    colors: PROBE_PANEL_THEMES[theme],
    typography: SHARED_PROBE_PANEL_TYPOGRAPHY
  };
}

export function formatProbePanelCssFont(style: CartesianTextStyle): string {
  return formatCartesianCssFont(style);
}
