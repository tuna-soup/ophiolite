import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { HashRouter } from "react-router-dom";
import type { Mock } from "vitest";
import App from "./App";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(() => Promise.resolve(() => {})) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

const mockedInvoke = invoke as unknown as Mock;
const mockedOpen = open as unknown as Mock;
const mockedListen = listen as unknown as Mock;

function projectSummary(root = "C:\\projects\\alpha") {
  return {
    root,
    catalog_path: `${root}\\catalog.sqlite`,
    manifest_path: `${root}\\ophiolite-project.json`,
    well_count: 1,
    asset_count: 2
  };
}

function installCommandMap() {
  mockedInvoke.mockImplementation((command: string, payload: { request: unknown }) => {
    const request = payload?.request as Record<string, unknown>;
    switch (command) {
      case "create_project":
      case "open_project":
        return Promise.resolve({ Ok: projectSummary() });
      case "list_project_wells":
        return Promise.resolve({
          Ok: [{ id: "well-1", name: "Well Alpha", identifiers: { uwi: "UWI-1", api: null, operator_aliases: [] } }]
        });
      case "list_project_wellbores":
        return Promise.resolve({
          Ok: [{ id: "wellbore-1", well_id: "well-1", name: "Main Bore", identifiers: { uwi: "UWI-1", api: null, operator_aliases: [] } }]
        });
      case "list_project_asset_collections":
        return Promise.resolve({
          Ok: [{ id: "collection-1", wellbore_id: "wellbore-1", asset_kind: "Log", name: "Main Log", logical_asset_id: "logical-1", status: "Bound" }]
        });
      case "list_project_assets":
        return Promise.resolve({
          Ok: [
            {
              id: "asset-log-1",
              logical_asset_id: "logical-1",
              collection_id: "collection-1",
              well_id: "well-1",
              wellbore_id: "wellbore-1",
              asset_kind: "Log",
              status: "Bound",
              package_path: "C:\\projects\\alpha\\assets\\logs\\asset-log-1.laspkg",
              manifest: { extents: { start: 1000, stop: 1001.5, row_count: 4 } }
            },
            {
              id: "asset-traj-1",
              logical_asset_id: "logical-2",
              collection_id: "collection-2",
              well_id: "well-1",
              wellbore_id: "wellbore-1",
              asset_kind: "Trajectory",
              status: "Bound",
              package_path: "C:\\projects\\alpha\\assets\\trajectory\\asset-traj-1",
              manifest: { extents: { start: 1000, stop: 2000, row_count: 2 } }
            }
          ]
        });
      case "open_package_session":
        return Promise.resolve({
          Ok: {
            session_id: "session-1",
            root: "C:\\projects\\alpha\\assets\\logs\\asset-log-1.laspkg",
            revision: "rev-1",
            dirty: { has_unsaved_changes: false },
            summary: { summary: { curve_count: 2, row_count: 4 } }
          }
        });
      case "session_metadata":
        return Promise.resolve({
          Ok: {
            session: { session_id: "session-1", root: "C:\\projects\\alpha\\assets\\logs\\asset-log-1.laspkg", revision: "rev-1" },
            metadata: { metadata: { well: { company: "Harness Co", start: 1000, stop: 1001.5, step: 0.5 }, other: "" } }
          }
        });
      case "session_curve_catalog":
        return Promise.resolve({
          Ok: {
            session: { session_id: "session-1", root: "C:\\projects\\alpha\\assets\\logs\\asset-log-1.laspkg", revision: "rev-1" },
            curves: [{ name: "DEPT", is_index: true }, { name: "GR", is_index: false }]
          }
        });
      case "read_depth_window":
      case "read_curve_window":
        return Promise.resolve({
          Ok: {
            session: { session_id: "session-1", root: "C:\\projects\\alpha\\assets\\logs\\asset-log-1.laspkg", revision: "rev-1" },
            window: {
              start_row: 0,
              row_count: 2,
              columns: [
                { name: "DEPT", values: [{ Number: 1000 }, { Number: 1000.5 }] },
                { name: "GR", values: [{ Number: 80.1 }, { Number: 81.2 }] }
              ]
            }
          }
        });
      case "read_package_files":
        return Promise.resolve({
          Ok: {
            root: "C:\\projects\\alpha\\assets\\logs\\asset-log-1.laspkg",
            metadata_path: "metadata.json",
            parquet_path: "curves.parquet",
            row_count: 4,
            curve_count: 2,
            columns: [{ name: "DEPT", storage_kind: "Numeric", unit: "m", is_index: true }],
            metadata_json: "{}"
          }
        });
      case "list_project_compute_catalog":
        return Promise.resolve({
          Ok: {
            asset_family: request.asset_id === "asset-traj-1" ? "Trajectory" : "Log",
            functions:
              request.asset_id === "asset-traj-1"
                ? [
                    {
                      metadata: {
                        id: "trajectory:normalize_azimuth",
                        provider: "trajectory",
                        name: "Normalize Azimuth",
                        category: "Trajectory",
                        description: "Wrap azimuth values into the 0-360 degree interval.",
                        default_output_mnemonic: "trajectory_normalize_azimuth",
                        output_curve_type: "Computed",
                        tags: ["trajectory"]
                      },
                      input_specs: [{ Trajectory: {} }],
                      parameters: [],
                      binding_candidates: [],
                      availability: { Available: {} }
                    }
                  ]
                : [
                    {
                      metadata: {
                        id: "petro:vshale_linear",
                        provider: "petro",
                        name: "VShale (Linear)",
                        category: "Petrophysics",
                        description: "Calculate VShale (Linear) from gamma ray.",
                        default_output_mnemonic: "VSH_LIN",
                        output_curve_type: "VShale",
                        tags: ["petro"]
                      },
                      input_specs: [{ SingleCurve: { parameter_name: "gr_curve", allowed_types: ["GammaRay"] } }],
                      parameters: [{ Number: { name: "gr_min", label: "GR Clean", description: "Gamma ray in clean sand.", default: 30 } }],
                      binding_candidates: [
                        {
                          parameter_name: "gr_curve",
                          allowed_types: ["GammaRay"],
                          matches: [{ curve_name: "GR", original_mnemonic: "GR", semantic_type: "GammaRay", unit: "gAPI" }]
                        }
                      ],
                      availability: { Available: {} }
                    }
                  ]
          }
        });
      case "run_project_compute":
        return Promise.resolve({
          Ok: {
            collection: { id: "derived-collection-1", wellbore_id: "wellbore-1", asset_kind: request.source_asset_id === "asset-traj-1" ? "Trajectory" : "Log", name: "Derived", logical_asset_id: "logical-derived", status: "Bound" },
            asset: {
              id: request.source_asset_id === "asset-traj-1" ? "asset-traj-2" : "asset-log-2",
              logical_asset_id: "logical-derived",
              collection_id: "derived-collection-1",
              well_id: "well-1",
              wellbore_id: "wellbore-1",
              asset_kind: request.source_asset_id === "asset-traj-1" ? "Trajectory" : "Log",
              status: "Bound",
              package_path: "C:\\projects\\alpha\\assets\\derived",
              manifest: { extents: { start: 1000, stop: 2000, row_count: 2 } }
            },
            execution: {
              function_id: request.function_id,
              function_name: "Derived Compute",
              provider: "test",
              output_curve_name: "DERIVED",
              output_curve_type: "Computed"
            }
          }
        });
      case "read_project_trajectory_rows":
        return Promise.resolve({ Ok: [{ measured_depth: 1000, true_vertical_depth: 950 }, { measured_depth: 1100, true_vertical_depth: 1030 }] });
      case "project_assets_covering_depth_range":
        return Promise.resolve({
          Ok: [
            { id: "asset-log-1", asset_kind: "Log", manifest: { extents: { start: 1000, stop: 1001.5 } } },
            { id: "asset-traj-1", asset_kind: "Trajectory", manifest: { extents: { start: 1000, stop: 2000 } } }
          ]
        });
      case "import_project_las":
        return Promise.resolve({
          Ok: {
            resolution: { status: "Bound", well_id: "well-1", wellbore_id: "wellbore-1", created_well: false, created_wellbore: false },
            collection: { id: "collection-3", wellbore_id: "wellbore-1", asset_kind: "Log", name: "Imported Log", logical_asset_id: "logical-3", status: "Bound" },
            asset: {
              id: "asset-log-2",
              logical_asset_id: "logical-3",
              collection_id: "collection-3",
              well_id: "well-1",
              wellbore_id: "wellbore-1",
              asset_kind: "Log",
              status: "Bound",
              package_path: "C:\\projects\\alpha\\assets\\logs\\asset-log-2.laspkg",
              manifest: { extents: { start: 1200, stop: 1300, row_count: 10 } }
            }
          }
        });
      case "save_session":
      case "save_session_as":
        return Promise.resolve({
          Ok: { session_id: "session-1", root: "C:\\projects\\alpha\\assets\\logs\\asset-log-1.laspkg", revision: "rev-2", dirty_cleared: true, overwritten: true }
        });
      default:
        return Promise.resolve({ Ok: {} });
    }
  });
}

function renderApp() {
  return render(
    <HashRouter>
      <App />
    </HashRouter>
  );
}

describe("ophiolite project harness", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedOpen.mockReset();
    mockedListen.mockClear();
    installCommandMap();
  });

  it("renders the project home", () => {
    renderApp();
    expect(screen.getByText(/open a ophiolite project/i)).toBeInTheDocument();
    expect(screen.getByTestId("create-project-button")).toBeInTheDocument();
    expect(screen.getByTestId("open-project-button")).toBeInTheDocument();
  });

  it("opens an existing project and shows wells and assets", async () => {
    mockedOpen.mockResolvedValueOnce("C:\\projects\\alpha");
    const user = userEvent.setup();
    renderApp();

    await user.click(screen.getByTestId("open-project-button"));

    await waitFor(() => expect(screen.getAllByText("C:\\projects\\alpha").length).toBeGreaterThan(0));
    expect(screen.getAllByText("Well Alpha").length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Main Bore/i).length).toBeGreaterThan(0);
  });

  it("imports a LAS asset into the selected project", async () => {
    mockedOpen.mockResolvedValueOnce("C:\\projects\\alpha");
    const user = userEvent.setup();
    renderApp();

    await user.click(screen.getByTestId("open-project-button"));
    await waitFor(() => expect(screen.getAllByText("Well Alpha").length).toBeGreaterThan(0));

    await user.click(screen.getByRole("button", { name: /import asset/i }));
    await user.click(screen.getByTestId("import-asset-button"));

    await waitFor(() =>
      expect(mockedInvoke).toHaveBeenCalledWith(
        "import_project_las",
        expect.objectContaining({
          request: expect.objectContaining({ project_root: "C:\\projects\\alpha" })
        })
      )
    );
  });

  it("runs a depth coverage query and opens a selected asset", async () => {
    mockedOpen.mockResolvedValueOnce("C:\\projects\\alpha");
    const user = userEvent.setup();
    renderApp();

    await user.click(screen.getByTestId("open-project-button"));
    await waitFor(() => expect(screen.getAllByText("Well Alpha").length).toBeGreaterThan(0));

    await user.click(screen.getByRole("button", { name: /depth coverage/i }));
    await user.type(screen.getByLabelText(/coverage depth min/i), "1000");
    await user.type(screen.getByLabelText(/coverage depth max/i), "1100");
    await user.click(screen.getByTestId("run-coverage-button"));

    await waitFor(() => expect(screen.getByText("asset-log-1")).toBeInTheDocument());
  });
});
