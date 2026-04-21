import { mount, unmount } from "svelte";

import {
  validateOphioliteAvoCrossplotSource,
  validateOphioliteAvoResponseSource,
  type OphioliteResolvedAvoCrossplotSource,
  type OphioliteResolvedAvoResponseSource,
  OphioliteAvoValidationError
} from "@ophiolite/charts-data-models";

import AvoCrossplotJupyterHost from "./AvoCrossplotJupyterHost.svelte";
import AvoResponseJupyterHost from "./AvoResponseJupyterHost.svelte";

export interface AvoChartMountOptions {
  chartId?: string;
}

export interface MountedAvoChartHandle<TSource> {
  setSource(source: TSource): void;
  fitToData(): void;
  dispose(): Promise<void>;
}

interface AvoResponseHostComponent {
  setSource(source: OphioliteResolvedAvoResponseSource): void;
  fitToData(): void;
}

interface AvoCrossplotHostComponent {
  setSource(source: OphioliteResolvedAvoCrossplotSource): void;
  fitToData(): void;
}

export function mountAvoResponseChart(
  target: HTMLElement,
  source: OphioliteResolvedAvoResponseSource,
  options: AvoChartMountOptions = {}
): MountedAvoChartHandle<OphioliteResolvedAvoResponseSource> {
  assertValidAvoResponseSource(source);
  const component = mount(AvoResponseJupyterHost, {
    target,
    props: {
      source,
      chartId: options.chartId
    }
  }) as AvoResponseHostComponent;
  return {
    setSource(nextSource) {
      assertValidAvoResponseSource(nextSource);
      component.setSource(nextSource);
    },
    fitToData() {
      component.fitToData();
    },
    dispose() {
      return Promise.resolve(unmount(component as never));
    }
  };
}

export function mountAvoCrossplotChart(
  target: HTMLElement,
  source: OphioliteResolvedAvoCrossplotSource,
  options: AvoChartMountOptions = {}
): MountedAvoChartHandle<OphioliteResolvedAvoCrossplotSource> {
  assertValidAvoCrossplotSource(source);
  const component = mount(AvoCrossplotJupyterHost, {
    target,
    props: {
      source,
      chartId: options.chartId
    }
  }) as AvoCrossplotHostComponent;
  return {
    setSource(nextSource) {
      assertValidAvoCrossplotSource(nextSource);
      component.setSource(nextSource);
    },
    fitToData() {
      component.fitToData();
    },
    dispose() {
      return Promise.resolve(unmount(component as never));
    }
  };
}

export type { OphioliteResolvedAvoResponseSource, OphioliteResolvedAvoCrossplotSource };

function assertValidAvoResponseSource(source: OphioliteResolvedAvoResponseSource): void {
  const issues = validateOphioliteAvoResponseSource(source);
  if (issues.length > 0) {
    throw new OphioliteAvoValidationError(issues);
  }
}

function assertValidAvoCrossplotSource(source: OphioliteResolvedAvoCrossplotSource): void {
  const issues = validateOphioliteAvoCrossplotSource(source);
  if (issues.length > 0) {
    throw new OphioliteAvoValidationError(issues);
  }
}
