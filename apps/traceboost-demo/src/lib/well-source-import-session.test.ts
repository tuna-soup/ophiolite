import assert from "node:assert/strict";
import test from "node:test";
import type {
  ProjectWellFolderImportPreview,
  ProjectWellSourceImportCanonicalDraft
} from "./bridge";
import {
  draftFromSuggestedDraft,
  editableTrajectoryRows,
  selectedTrajectoryDraftRows,
  summarizeTrajectoryDraftRows
} from "./well-source-import-session";

test("draftFromSuggestedDraft keeps detected CRS as a suggestion but leaves geometry unresolved until the user confirms it", () => {
  const preview = {
    topsMarkers: {
      commitEnabled: true,
      preferredDepthReference: "md"
    },
    trajectory: {
      commitEnabled: true
    }
  } as unknown as ProjectWellFolderImportPreview;

  const suggestedDraft = {
    binding: {
      well_name: "F02-A-02",
      wellbore_name: "F02-A-02",
      uwi: "8514",
      api: null,
      operator_aliases: []
    },
    sourceCoordinateReference: {
      mode: "detected",
      candidateId: "EPSG:23031",
      coordinateReference: null
    },
    wellMetadata: null,
    wellboreMetadata: null,
    importPlan: {
      selectedLogSourcePaths: [],
      asciiLogImports: null,
      topsMarkers: null,
      trajectory: { enabled: true }
    }
  } as ProjectWellSourceImportCanonicalDraft;

  const draft = draftFromSuggestedDraft(preview, suggestedDraft);

  assert.equal(draft.sourceCrsMode, "unresolved");
  assert.equal(draft.detectedCandidateId, "EPSG:23031");
});

test("trajectory draft helpers treat supplemented md-inc-azi stations as committable", () => {
  const rows = selectedTrajectoryDraftRows(
    [
      {
        measuredDepth: "100",
        inclinationDeg: "0.5",
        azimuthDeg: "45",
        trueVerticalDepth: "",
        xOffset: "",
        yOffset: ""
      },
      {
        measuredDepth: "200",
        inclinationDeg: "1.2",
        azimuthDeg: "47",
        trueVerticalDepth: "",
        xOffset: "",
        yOffset: ""
      }
    ],
    (value) => {
      const parsed = Number(value.trim());
      return Number.isFinite(parsed) ? parsed : null;
    }
  );

  const summary = summarizeTrajectoryDraftRows(rows);

  assert.equal(summary.measuredDepthCount, 2);
  assert.equal(summary.inclinationDegCount, 2);
  assert.equal(summary.azimuthDegCount, 2);
  assert.equal(summary.commitEnabled, true);
});

test("editableTrajectoryRows seeds the confirmation draft from preview draft rows", () => {
  const preview = {
    trajectory: {
      sourcePath: "/tmp/deviatie.txt",
      draftRows: [
        {
          measuredDepth: 100,
          inclinationDeg: 0.5,
          azimuthDeg: 45,
          trueVerticalDepth: null,
          xOffset: null,
          yOffset: null
        },
        {
          measuredDepth: 200,
          inclinationDeg: 1.2,
          azimuthDeg: 47,
          trueVerticalDepth: null,
          xOffset: null,
          yOffset: null
        }
      ],
      sampleRows: []
    }
  } as unknown as ProjectWellFolderImportPreview;

  const suggestedDraft = {
    importPlan: {
      trajectory: null
    }
  } as ProjectWellSourceImportCanonicalDraft;

  const rows = editableTrajectoryRows(preview, suggestedDraft);

  assert.equal(rows.length, 2);
  assert.equal(rows[0].measuredDepth, "100");
  assert.equal(rows[1].azimuthDeg, "47");
});
