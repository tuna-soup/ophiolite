import assert from "node:assert/strict";
import test from "node:test";
import { describeDatasetCoordinateMaterializationAvailability } from "./dataset-coordinate-materialization";

test("describeDatasetCoordinateMaterializationAvailability requires an active dataset first", () => {
  assert.deepStrictEqual(
    describeDatasetCoordinateMaterializationAvailability({
      hasActiveDataset: false,
      displayCoordinateReferenceId: "EPSG:32632",
      activeEffectiveNativeCoordinateReferenceId: "EPSG:32632",
      activeSurveyMapTransformStatus: "display_equivalent"
    }),
    {
      reasonCode: "missing_dataset",
      title: "No active survey dataset",
      message: "Open a seismic volume before materializing a reprojected copy."
    }
  );
});

test("describeDatasetCoordinateMaterializationAvailability requires the survey CRS before a copy", () => {
  assert.deepStrictEqual(
    describeDatasetCoordinateMaterializationAvailability({
      hasActiveDataset: true,
      displayCoordinateReferenceId: "EPSG:32632",
      activeEffectiveNativeCoordinateReferenceId: null,
      activeSurveyMapTransformStatus: "native_only"
    }),
    {
      reasonCode: "missing_native_crs",
      title: "Survey CRS required",
      message:
        "Assign the survey CRS first. That updates how TraceBoost interprets the current store's raw X/Y values without rewriting the dataset."
    }
  );
});

test("describeDatasetCoordinateMaterializationAvailability reports when no copy is needed", () => {
  assert.deepStrictEqual(
    describeDatasetCoordinateMaterializationAvailability({
      hasActiveDataset: true,
      displayCoordinateReferenceId: "EPSG:32632",
      activeEffectiveNativeCoordinateReferenceId: "epsg:32632",
      activeSurveyMapTransformStatus: "display_equivalent"
    }),
    {
      reasonCode: "already_equivalent",
      title: "No copy needed",
      message:
        "The active survey CRS already matches display CRS EPSG:32632. Assigning the survey CRS is enough; a reprojected copy would not change the stored coordinates."
    }
  );
});

test("describeDatasetCoordinateMaterializationAvailability distinguishes missing transforms from missing backend support", () => {
  assert.deepStrictEqual(
    describeDatasetCoordinateMaterializationAvailability({
      hasActiveDataset: true,
      displayCoordinateReferenceId: "EPSG:3857",
      activeEffectiveNativeCoordinateReferenceId: "EPSG:4326",
      activeSurveyMapTransformStatus: "display_unavailable"
    }),
    {
      reasonCode: "transform_unavailable",
      title: "No display transform available",
      message:
        "No display transform is currently available from EPSG:4326 to EPSG:3857, so TraceBoost cannot stage a reprojected copy for this survey."
    }
  );

  assert.deepStrictEqual(
    describeDatasetCoordinateMaterializationAvailability({
      hasActiveDataset: true,
      displayCoordinateReferenceId: "EPSG:3857",
      activeEffectiveNativeCoordinateReferenceId: "EPSG:4326",
      activeSurveyMapTransformStatus: "display_transformed"
    }),
    {
      reasonCode: "backend_unavailable",
      title: "Reprojected copy not available yet",
      message:
        "Assigning the survey CRS updates interpretation only. Writing a new seismic volume store in display CRS EPSG:3857 is not available yet."
    }
  );
});
