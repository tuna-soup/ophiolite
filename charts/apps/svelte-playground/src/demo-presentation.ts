import { CARTESIAN_FONT_FAMILY, resolveProbePanelPresentation } from "@ophiolite/charts-core";

const lightPanel = resolveProbePanelPresentation("light", "standard");
const darkPanel = resolveProbePanelPresentation("dark", "standard");

const DEMO_CSS_VARS: Record<string, string> = {
  "--demo-font-family": CARTESIAN_FONT_FAMILY,
  "--demo-radius-sm": `${lightPanel.frame.borderRadiusPx}px`,
  "--demo-radius-md": "8px",
  "--demo-shell-bg": "#0b1720",
  "--demo-shell-text": darkPanel.colors.text,
  "--demo-text-muted": darkPanel.colors.label,
  "--demo-sidebar-bg": "rgba(4, 19, 29, 0.72)",
  "--demo-sidebar-border": darkPanel.colors.border,
  "--demo-group-title": "#c3d3de",
  "--demo-readout-bg": "rgba(7, 19, 29, 0.54)",
  "--demo-readout-border": darkPanel.colors.border,
  "--demo-button-bg": "rgba(250, 252, 253, 0.96)",
  "--demo-button-bg-active": "rgba(176, 220, 241, 0.98)",
  "--demo-button-bg-disabled": "rgba(250, 252, 253, 0.54)",
  "--demo-button-border": "rgba(132, 151, 168, 0.24)",
  "--demo-button-text": lightPanel.colors.text,
  "--demo-button-shadow": "0 8px 18px rgba(0, 0, 0, 0.12)",
  "--demo-viewer-border": "rgba(255, 255, 255, 0.06)",
  "--demo-selection-bg": "rgba(4, 19, 29, 0.86)",
  "--demo-selection-border": darkPanel.colors.border,
  "--demo-selection-text": darkPanel.colors.text,
  "--demo-modal-backdrop": "rgba(7, 12, 18, 0.14)",
  "--demo-modal-bg": lightPanel.colors.background,
  "--demo-modal-border": lightPanel.colors.border,
  "--demo-modal-shadow": lightPanel.colors.shadow,
  "--demo-modal-text": lightPanel.colors.text,
  "--demo-modal-muted": lightPanel.colors.label,
  "--demo-input-bg": "#f7fafc",
  "--demo-input-border": "rgba(123, 142, 161, 0.3)",
  "--demo-input-border-focus": "#86b2cb",
  "--demo-input-text": lightPanel.colors.text,
  "--demo-input-placeholder": "#7f93a7",
  "--demo-secondary-bg": "rgba(112, 131, 150, 0.12)",
  "--demo-secondary-border": "rgba(123, 142, 161, 0.28)",
  "--demo-secondary-text": lightPanel.colors.text,
  "--demo-primary-bg": "#233445",
  "--demo-primary-border": "#233445",
  "--demo-primary-text": "#f5f8fa"
};

export const demoCssVars = Object.entries(DEMO_CSS_VARS)
  .map(([name, value]) => `${name}: ${value}`)
  .join("; ");
