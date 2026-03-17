import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { invoke } from "@tauri-apps/api/core";
import type { Mock } from "vitest";
import App from "./App";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn()
}));

const mockedInvoke = invoke as unknown as Mock;

function queueInvokeResponses(...responses: unknown[]) {
  mockedInvoke.mockReset();
  for (const response of responses) {
    mockedInvoke.mockResolvedValueOnce(response);
  }
}

describe("Lithos harness smoke flows", () => {
  it("supports inspect and session open flows", async () => {
    queueInvokeResponses(
      { Ok: { package: { path: "C:\\well.laspkg" } } },
      {
        Ok: {
          session_id: "session-1",
          root: "C:\\well.laspkg",
          revision: "rev-1",
          dirty: { has_unsaved_changes: false }
        }
      },
      {
        Ok: {
          session: {
            session_id: "session-1",
            root: "C:\\well.laspkg",
            revision: "rev-1"
          },
          metadata: {
            metadata: {
              well: {
                company: "Lithos Energy"
              }
            }
          }
        }
      },
      {
        Ok: {
          session: {
            session_id: "session-1",
            root: "C:\\well.laspkg",
            revision: "rev-1"
          },
          curves: [
            { mnemonic: "DT" },
            { mnemonic: "RHOB" }
          ]
        }
      },
      {
        Ok: {
          session: {
            session_id: "session-1",
            root: "C:\\well.laspkg",
            revision: "rev-1"
          },
          window: {
            start_row: 0,
            row_count: 3
          }
        }
      }
    );

    const user = userEvent.setup();
    render(<App />);

    await user.type(screen.getByLabelText(/package path/i), "C:\\well.laspkg");
    await user.click(screen.getByRole("button", { name: /inspect summary/i }));
    await waitFor(() =>
      expect(screen.getByTestId("last-command-output")).toHaveTextContent(
        "inspect_package_summary"
      )
    );

    await user.click(screen.getByRole("button", { name: /open session/i }));
    await waitFor(() =>
      expect(screen.getByText("session-1")).toBeInTheDocument()
    );

    await user.click(screen.getByRole("button", { name: /session metadata/i }));
    await waitFor(() =>
      expect(screen.getByTestId("session-metadata-output")).toHaveTextContent(
        "Lithos Energy"
      )
    );

    await user.click(screen.getByRole("button", { name: /curve catalog/i }));
    await waitFor(() =>
      expect(screen.getByTestId("diagnostics-output")).toHaveTextContent("RHOB")
    );

    await user.click(screen.getByRole("button", { name: /read curve window/i }));
    await waitFor(() =>
      expect(screen.getByTestId("window-output")).toHaveTextContent("row_count")
    );

    expect(mockedInvoke).toHaveBeenNthCalledWith(1, "inspect_package_summary", {
      request: { path: "C:\\well.laspkg" }
    });
    expect(mockedInvoke).toHaveBeenNthCalledWith(2, "open_package_session", {
      request: { path: "C:\\well.laspkg" }
    });
  });

  it("supports metadata edit and save-as flows", async () => {
    queueInvokeResponses(
      {
        Ok: {
          session_id: "session-2",
          root: "C:\\well.laspkg",
          revision: "rev-2",
          dirty: { has_unsaved_changes: false }
        }
      },
      {
        Ok: {
          session_id: "session-2",
          root: "C:\\well.laspkg",
          revision: "rev-2",
          dirty: { has_unsaved_changes: true }
        }
      },
      {
        Ok: {
          session: {
            session_id: "session-2",
            root: "C:\\copy.laspkg",
            revision: "rev-3"
          },
          saved: {
            path: "C:\\copy.laspkg"
          }
        }
      }
    );

    const user = userEvent.setup();
    render(<App />);

    await user.type(screen.getByLabelText(/package path/i), "C:\\well.laspkg");
    await user.click(screen.getByRole("button", { name: /open session/i }));
    await waitFor(() => expect(screen.getByText("session-2")).toBeInTheDocument());

    const valueField = screen.getByLabelText(/^value$/i);
    await user.clear(valueField);
    await user.type(valueField, "HARNESSED");
    await user.click(screen.getByRole("button", { name: /apply metadata edit/i }));
    await waitFor(() =>
      expect(screen.getByTestId("session-summary-output")).toHaveTextContent("true")
    );

    await user.type(
      screen.getByLabelText(/save-as output directory/i),
      "C:\\copy.laspkg"
    );
    await user.click(screen.getByRole("button", { name: /^save as$/i }));
    await waitFor(() =>
      expect(screen.getByTestId("last-command-output")).toHaveTextContent(
        "copy.laspkg"
      )
    );

    expect(mockedInvoke).toHaveBeenNthCalledWith(2, "apply_metadata_edit", {
      request: {
        session_id: "session-2",
        update: {
          items: [
            {
              section: "Well",
              mnemonic: "COMP",
              unit: "",
              value: { Text: "HARNESSED" },
              description: "COMPANY"
            }
          ],
          other: null
        }
      }
    });
  });

  it("supports curve edit and save flows", async () => {
    queueInvokeResponses(
      {
        Ok: {
          session_id: "session-3",
          root: "C:\\well.laspkg",
          revision: "rev-4",
          dirty: { has_unsaved_changes: false }
        }
      },
      {
        Ok: {
          session_id: "session-3",
          root: "C:\\well.laspkg",
          revision: "rev-5",
          dirty: { has_unsaved_changes: true }
        }
      },
      {
        Ok: {
          session: {
            session_id: "session-3",
            root: "C:\\well.laspkg",
            revision: "rev-6"
          },
          saved: {
            path: "C:\\well.laspkg"
          }
        }
      }
    );

    const user = userEvent.setup();
    render(<App />);

    await user.type(screen.getByLabelText(/package path/i), "C:\\well.laspkg");
    await user.click(screen.getByRole("button", { name: /open session/i }));
    await waitFor(() => expect(screen.getByText("session-3")).toBeInTheDocument());

    const curveEditor = screen.getByLabelText(/curve edit payload/i);
    fireEvent.change(curveEditor, {
      target: { value: '{"Remove":{"mnemonic":"DT"}}' }
    });
    await user.click(screen.getByRole("button", { name: /apply curve edit/i }));
    await waitFor(() =>
      expect(screen.getByTestId("session-summary-output")).toHaveTextContent("rev-5")
    );

    await user.click(screen.getByRole("button", { name: /^save$/i }));
    await waitFor(() =>
      expect(screen.getByTestId("last-command-output")).toHaveTextContent(
        "well.laspkg"
      )
    );

    expect(mockedInvoke).toHaveBeenNthCalledWith(2, "apply_curve_edit", {
      request: {
        session_id: "session-3",
        edit: {
          Remove: {
            mnemonic: "DT"
          }
        }
      }
    });
  });
});
