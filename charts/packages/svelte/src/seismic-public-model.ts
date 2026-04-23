import {
  OphioliteSeismicValidationError,
  adaptOphioliteGatherViewToPayload,
  adaptOphioliteSectionViewToPayload,
  validateGatherPayload,
  validateSectionPayload,
  type GatherPayload,
  type OphioliteEncodedGatherView,
  type OphioliteEncodedSectionView,
  type SectionPayload
} from "@ophiolite/charts-data-models";

export function adaptOphioliteSectionViewToSeismicSectionData(
  contract: OphioliteEncodedSectionView
): SectionPayload {
  return adaptOphioliteSectionViewToPayload(contract);
}

export function adaptOphioliteGatherViewToSeismicGatherData(
  contract: OphioliteEncodedGatherView
): GatherPayload {
  return adaptOphioliteGatherViewToPayload(contract);
}

export function adaptSeismicSectionDataToPayload(section: SectionPayload): SectionPayload {
  const issues = validateSectionPayload(section);
  if (issues.length > 0) {
    throw new OphioliteSeismicValidationError(issues);
  }
  return section;
}

export function adaptSeismicGatherDataToPayload(gather: GatherPayload): GatherPayload {
  const issues = validateGatherPayload(gather);
  if (issues.length > 0) {
    throw new OphioliteSeismicValidationError(issues);
  }
  return gather;
}

export function isOphioliteSectionView(
  value: SectionPayload | OphioliteEncodedSectionView
): value is OphioliteEncodedSectionView {
  return "horizontal_axis_f64le" in value;
}

export function isOphioliteGatherView(
  value: GatherPayload | OphioliteEncodedGatherView
): value is OphioliteEncodedGatherView {
  return "horizontal_axis_f64le" in value;
}

export function adaptSeismicSectionInputToPayload(
  input: SectionPayload | OphioliteEncodedSectionView
): SectionPayload {
  return isOphioliteSectionView(input)
    ? adaptOphioliteSectionViewToSeismicSectionData(input)
    : adaptSeismicSectionDataToPayload(input);
}

export function adaptSeismicGatherInputToPayload(
  input: GatherPayload | OphioliteEncodedGatherView
): GatherPayload {
  return isOphioliteGatherView(input)
    ? adaptOphioliteGatherViewToSeismicGatherData(input)
    : adaptSeismicGatherDataToPayload(input);
}
