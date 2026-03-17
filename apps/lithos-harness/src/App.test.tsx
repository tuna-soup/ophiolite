import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { HashRouter } from "react-router-dom";
import type { Mock } from "vitest";
import App from "./App";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn()
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(() => Promise.resolve(() => {}))
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn()
}));

const mockedInvoke = invoke as unknown as Mock;
const mockedListen = listen as unknown as Mock;
const mockedOpen = open as unknown as Mock;

const session = {
  Ok: {
    session_id: "session-1",
    root: "C:\\packages\\well-a",
    revision: "rev-1",
    dirty: { has_unsaved_changes: false },
    summary: {
      summary: { curve_count: 2, row_count: 4 }
    }
  }
};

function installCommandMap() {
  mockedInvoke.mockImplementation((command: string) => {
    switch (command) {
      case "open_package_session":
        return Promise.resolve(session);
      case "validate_package":
        return Promise.resolve({ Ok: { valid: true, issues: [], errors: [], kind: "Package" } });
      case "session_summary":
        return Promise.resolve(session);
      case "session_metadata":
        return Promise.resolve({
          Ok: {
            session: { session_id: "session-1", root: "C:\\packages\\well-a", revision: "rev-1" },
            metadata: {
              metadata: {
                well: { company: "Harness Co", start: 1000, stop: 1001.5, step: 0.5 },
                other: ""
              }
            }
          }
        });
      case "session_curve_catalog":
        return Promise.resolve({
          Ok: {
            session: { session_id: "session-1", root: "C:\\packages\\well-a", revision: "rev-1" },
            curves: [
              { curve_id: "curve-depth", name: "DEPT", is_index: true, storage_kind: "Numeric" },
              { curve_id: "curve-dt", name: "DT", original_mnemonic: "DT", storage_kind: "Numeric" }
            ]
          }
        });
      case "read_curve_window":
        return Promise.resolve({
          Ok: {
            session: { session_id: "session-1", root: "C:\\packages\\well-a", revision: "rev-1" },
            window: {
              start_row: 0,
              row_count: 2,
              columns: [
                { name: "DEPT", values: [{ Number: 1000 }, { Number: 1000.5 }] },
                { name: "DT", values: [{ Number: 123.4 }, { Number: 124.1 }] }
              ]
            }
          }
        });
      case "read_depth_window":
        return Promise.resolve({
          Ok: {
            session: { session_id: "session-1", root: "C:\\packages\\well-a", revision: "rev-1" },
            window: {
              start_row: 0,
              row_count: 2,
              columns: [
                { name: "DEPT", values: [{ Number: 1000 }, { Number: 1000.5 }] },
                { name: "DT", values: [{ Number: 123.4 }, { Number: 124.1 }] }
              ]
            }
          }
        });
      case "apply_metadata_edit":
        return Promise.resolve({
          Ok: {
            session_id: "session-1",
            root: "C:\\packages\\well-a",
            revision: "rev-2",
            dirty: { has_unsaved_changes: true },
            summary: { summary: { curve_count: 2, row_count: 4 } }
          }
        });
      case "apply_curve_edit":
        return Promise.resolve({
          Ok: {
            session_id: "session-1",
            root: "C:\\packages\\well-a",
            revision: "rev-3",
            dirty: { has_unsaved_changes: true },
            summary: { summary: { curve_count: 2, row_count: 4 } }
          }
        });
      case "save_session":
        return Promise.resolve({
          Ok: {
            session_id: "session-1",
            root: "C:\\packages\\well-a",
            revision: "rev-4",
            overwritten: true,
            dirty_cleared: true
          }
        });
      case "save_session_as":
        return Promise.resolve({
          Ok: {
            session_id: "session-1",
            root: "C:\\packages\\well-b",
            revision: "rev-5",
            overwritten: false,
            dirty_cleared: true
          }
        });
      case "close_session":
        return Promise.resolve({ Ok: { closed: true } });
      case "inspect_las_summary":
        return Promise.resolve({ Ok: { summary: { curve_count: 2, row_count: 4 } } });
      case "inspect_las_metadata":
        return Promise.resolve({ Ok: { metadata: { well: { company: "Raw Co" } } } });
      case "inspect_las_curve_catalog":
        return Promise.resolve({ Ok: [{ name: "DT" }, { name: "RHOB" }] });
      case "inspect_las_window":
        return Promise.resolve({
          Ok: {
            start_row: 0,
            row_count: 2,
            columns: [{ name: "DT", values: [{ Number: 123.4 }, { Number: 124.1 }] }]
          }
        });
      case "validate_las":
        return Promise.resolve({ Ok: { valid: true, issues: [], errors: [], kind: "Package" } });
      case "import_las_into_workspace":
        return Promise.resolve(session);
      case "read_package_files":
        return Promise.resolve({
          Ok: {
            root: "C:\\packages\\well-a",
            metadata_path: "C:\\packages\\well-a\\metadata.json",
            parquet_path: "C:\\packages\\well-a\\curves.parquet",
            row_count: 4,
            curve_count: 2,
            columns: [{ name: "DEPT", storage_kind: "Numeric", unit: "m", is_index: true }],
            metadata_json: "{\n  \"canonical\": {}\n}"
          }
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

describe("lithos harness desktop shell", () => {
  beforeEach(() => {
    mockedInvoke.mockReset();
    mockedListen.mockClear();
    mockedOpen.mockReset();
    installCommandMap();
  });

  it("renders the package home", () => {
    renderApp();
    expect(screen.getByText(/create or open a package/i)).toBeInTheDocument();
    expect(screen.getByTestId("create-draft-button")).toBeInTheDocument();
    expect(screen.getByTestId("open-package-button")).toBeInTheDocument();
  });

  it("creates a draft workspace from a folder dialog", async () => {
    mockedOpen.mockResolvedValueOnce("C:\\packages\\draft-a");
    const user = userEvent.setup();
    renderApp();

    await user.click(screen.getByTestId("create-draft-button"));

    await waitFor(() =>
      expect(screen.getAllByText("C:\\packages\\draft-a").length).toBeGreaterThan(0)
    );
    expect(screen.getAllByText(/draft workspace/i).length).toBeGreaterThan(0);
  });

  it("opens an existing package into a live session workspace", async () => {
    mockedOpen.mockResolvedValueOnce("C:\\packages\\well-a");
    const user = userEvent.setup();
    renderApp();

    await user.click(screen.getByTestId("open-package-button"));

    await waitFor(() => expect(screen.getAllByText("C:\\packages\\well-a").length).toBeGreaterThan(0));
    await waitFor(() => expect(mockedInvoke).toHaveBeenCalledWith("read_depth_window", expect.anything()));
    expect(
      screen.getByRole("button", { name: /metadata canonical metadata inspector/i })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /curves catalog and editable sample table/i })
    ).toBeInTheDocument();
  });

  it("creates a package session immediately, edits metadata and curves, and saves", async () => {
    mockedOpen
      .mockResolvedValueOnce("C:\\packages\\draft-a")
      .mockResolvedValueOnce("C:\\logs\\sample.las")
      .mockResolvedValueOnce("C:\\packages\\well-b");

    const user = userEvent.setup();
    renderApp();

    await user.click(screen.getByTestId("create-draft-button"));

    await waitFor(() => expect(screen.getAllByText("C:\\packages\\well-a").length).toBeGreaterThan(0));
    expect(mockedInvoke).toHaveBeenCalledWith(
      "import_las_into_workspace",
      expect.objectContaining({
        request: expect.objectContaining({
          package_root: "C:\\packages\\draft-a",
          las_path: "C:\\logs\\sample.las"
        })
      })
    );

    await user.click(
      screen.getByRole("button", { name: /metadata canonical metadata inspector/i })
    );
    await user.clear(screen.getByLabelText(/company value/i));
    await user.type(screen.getByLabelText(/company value/i), "HARNESSED");
    await user.click(screen.getByTestId("metadata-apply-button"));

    await user.click(
      screen.getByRole("button", { name: /curves catalog and editable sample table/i })
    );
    await waitFor(() => expect(screen.getByDisplayValue("123.4")).toBeInTheDocument());
    const cellInput = screen.getByDisplayValue("123.4");
    await user.clear(cellInput);
    await user.type(cellInput, "111.1");
    await user.click(screen.getByTestId("curve-apply-button"));

    const menubar = screen.getByLabelText(/workspace menubar/i);
    await user.click(within(menubar).getByRole("button", { name: /^save$/i }));
    await user.click(within(menubar).getByRole("button", { name: /^save as$/i }));

    await waitFor(() => expect(mockedInvoke).toHaveBeenCalledWith("save_session_as", expect.anything()));
  });
});
