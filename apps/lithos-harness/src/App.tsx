import { useMemo, useState } from "react";
import {
  type CommandErrorDto,
  type CommandResponse,
  type SessionMetadataDto,
  type SessionSummaryDto,
  invokeCommand
} from "./api";

const defaultCurveEdit = JSON.stringify(
  {
    Upsert: {
      mnemonic: "DT",
      original_mnemonic: "DT",
      unit: "US/M",
      header_value: "Empty",
      description: "edit curve values here",
      data: [{ Number: 123.45 }, { Number: 234.56 }, { Number: 345.67 }]
    }
  },
  null,
  2
);

function prettyJson(value: unknown): string {
  return JSON.stringify(value, null, 2);
}

export default function App() {
  const [packagePath, setPackagePath] = useState("");
  const [saveAsPath, setSaveAsPath] = useState("");
  const [curveNames, setCurveNames] = useState("DT,RHOB");
  const [startRow, setStartRow] = useState("0");
  const [rowCount, setRowCount] = useState("3");
  const [curveEditJson, setCurveEditJson] = useState(defaultCurveEdit);
  const [metadataSection, setMetadataSection] = useState("Well");
  const [metadataMnemonic, setMetadataMnemonic] = useState("COMP");
  const [metadataUnit, setMetadataUnit] = useState("");
  const [metadataValueType, setMetadataValueType] = useState("Text");
  const [metadataValue, setMetadataValue] = useState("HARNESS EDIT");
  const [metadataDescription, setMetadataDescription] = useState("COMPANY");
  const [metadataOther, setMetadataOther] = useState("");

  const [sessionSummary, setSessionSummary] = useState<SessionSummaryDto | null>(null);
  const [sessionMetadata, setSessionMetadata] = useState<SessionMetadataDto | null>(null);
  const [catalog, setCatalog] = useState<unknown>(null);
  const [windowResult, setWindowResult] = useState<unknown>(null);
  const [validation, setValidation] = useState<unknown>(null);
  const [dirtyState, setDirtyState] = useState<unknown>(null);
  const [lastCommand, setLastCommand] = useState<string>("");
  const [lastResponse, setLastResponse] = useState<unknown>(null);
  const [lastError, setLastError] = useState<CommandErrorDto | null>(null);

  const currentSessionId = sessionSummary?.session_id ?? "";
  const currentRoot = sessionSummary?.root ?? "";

  const metadataValuePayload = useMemo(() => {
    if (metadataValueType === "Number") {
      return { Number: Number(metadataValue) };
    }
    if (metadataValueType === "Empty") {
      return "Empty";
    }
    return { Text: metadataValue };
  }, [metadataValue, metadataValueType]);

  async function run<T>(
    command: string,
    request: unknown,
    onOk?: (value: T) => void
  ) {
    setLastCommand(command);
    const response = await invokeCommand<T>(command, request);
    setLastResponse(response);
    if ("Err" in response) {
      setLastError(response.Err);
      return;
    }
    setLastError(null);
    onOk?.(response.Ok);
  }

  function requireSession(): string {
    if (!currentSessionId) {
      throw new Error("Open a package session first.");
    }
    return currentSessionId;
  }

  return (
    <main className="app-shell">
      <header className="hero">
        <div>
          <p className="eyebrow">Lithos Internal Harness</p>
          <h1>Tauri capability surface for the LAS SDK</h1>
          <p className="lede">
            This app is intentionally utilitarian. It is here to exercise the
            SDK end to end: inspect packages, open sessions, query windows, edit
            metadata and curves, save, save-as, and render structured
            diagnostics.
          </p>
        </div>
        <div className="session-pill">
          <span>Session</span>
          <strong>{currentSessionId || "none"}</strong>
          <small>{currentRoot || "no package open"}</small>
        </div>
      </header>

      <section className="grid">
        <article className="panel">
          <h2>Package</h2>
          <label>
            Package path
            <input
              value={packagePath}
              onChange={(event) => setPackagePath(event.target.value)}
              placeholder="C:\\path\\to\\well.laspkg"
            />
          </label>
          <div className="button-row">
            <button
              onClick={() =>
                run("inspect_package_summary", { path: packagePath }, setLastResponse)
              }
            >
              Inspect Summary
            </button>
            <button
              onClick={() =>
                run("inspect_package_metadata", { path: packagePath }, setLastResponse)
              }
            >
              Inspect Metadata
            </button>
            <button
              onClick={() => run("validate_package", { path: packagePath }, setValidation)}
            >
              Validate Package
            </button>
            <button
              className="primary"
              onClick={() =>
                run<SessionSummaryDto>(
                  "open_package_session",
                  { path: packagePath },
                  (value) => {
                    setSessionSummary(value);
                    setSessionMetadata(null);
                    setCatalog(null);
                    setWindowResult(null);
                    setDirtyState(null);
                  }
                )
              }
            >
              Open Session
            </button>
          </div>
        </article>

        <article className="panel">
          <h2>Session</h2>
          <div className="button-row">
            <button
              onClick={() =>
                run<SessionSummaryDto>(
                  "session_summary",
                  { session_id: requireSession() },
                  setSessionSummary
                )
              }
            >
              Session Summary
            </button>
            <button
              onClick={() =>
                run<SessionMetadataDto>(
                  "session_metadata",
                  { session_id: requireSession() },
                  setSessionMetadata
                )
              }
            >
              Session Metadata
            </button>
            <button
              onClick={() =>
                run("session_curve_catalog", { session_id: requireSession() }, setCatalog)
              }
            >
              Curve Catalog
            </button>
            <button
              onClick={() =>
                run("dirty_state", { session_id: requireSession() }, setDirtyState)
              }
            >
              Dirty State
            </button>
            <button
              onClick={() =>
                run("close_session", { session_id: requireSession() }, () => {
                  setSessionSummary(null);
                  setSessionMetadata(null);
                  setCatalog(null);
                  setWindowResult(null);
                  setDirtyState(null);
                })
              }
            >
              Close Session
            </button>
          </div>
        </article>

        <article className="panel">
          <h2>Window Query</h2>
          <label>
            Curve names
            <input
              value={curveNames}
              onChange={(event) => setCurveNames(event.target.value)}
              placeholder="DT,RHOB"
            />
          </label>
          <div className="inline-fields">
            <label>
              Start row
              <input
                value={startRow}
                onChange={(event) => setStartRow(event.target.value)}
              />
            </label>
            <label>
              Row count
              <input
                value={rowCount}
                onChange={(event) => setRowCount(event.target.value)}
              />
            </label>
          </div>
          <button
            className="primary"
            onClick={() =>
              run(
                "read_curve_window",
                {
                  session_id: requireSession(),
                  window: {
                    curve_names: curveNames
                      .split(",")
                      .map((value) => value.trim())
                      .filter(Boolean),
                    start_row: Number(startRow),
                    row_count: Number(rowCount)
                  }
                },
                setWindowResult
              )
            }
          >
            Read Curve Window
          </button>
        </article>

        <article className="panel">
          <h2>Metadata Edit</h2>
          <div className="inline-fields">
            <label>
              Section
              <select
                value={metadataSection}
                onChange={(event) => setMetadataSection(event.target.value)}
              >
                <option value="Version">Version</option>
                <option value="Well">Well</option>
                <option value="Parameters">Parameters</option>
              </select>
            </label>
            <label>
              Mnemonic
              <input
                value={metadataMnemonic}
                onChange={(event) => setMetadataMnemonic(event.target.value)}
              />
            </label>
          </div>
          <div className="inline-fields">
            <label>
              Unit
              <input
                value={metadataUnit}
                onChange={(event) => setMetadataUnit(event.target.value)}
              />
            </label>
            <label>
              Value type
              <select
                value={metadataValueType}
                onChange={(event) => setMetadataValueType(event.target.value)}
              >
                <option value="Text">Text</option>
                <option value="Number">Number</option>
                <option value="Empty">Empty</option>
              </select>
            </label>
          </div>
          <label>
            Value
            <input
              value={metadataValue}
              onChange={(event) => setMetadataValue(event.target.value)}
              disabled={metadataValueType === "Empty"}
            />
          </label>
          <label>
            Description
            <input
              value={metadataDescription}
              onChange={(event) => setMetadataDescription(event.target.value)}
            />
          </label>
          <label>
            Replace OTHER text
            <textarea
              value={metadataOther}
              onChange={(event) => setMetadataOther(event.target.value)}
              placeholder="Leave blank to keep OTHER unchanged."
            />
          </label>
          <button
            className="primary"
            onClick={() =>
              run(
                "apply_metadata_edit",
                {
                  session_id: requireSession(),
                  update: {
                    items: [
                      {
                        section: metadataSection,
                        mnemonic: metadataMnemonic,
                        unit: metadataUnit,
                        value: metadataValuePayload,
                        description: metadataDescription
                      }
                    ],
                    other: metadataOther.trim() ? metadataOther : null
                  }
                },
                setSessionSummary
              )
            }
          >
            Apply Metadata Edit
          </button>
        </article>

        <article className="panel">
          <h2>Curve Edit</h2>
          <p className="hint">
            Use the SDK request shape directly. Example:{" "}
            <code>{'{"Remove":{"mnemonic":"DT"}}'}</code> or the default upsert
            template below.
          </p>
          <textarea
            aria-label="Curve edit payload"
            className="code-area"
            value={curveEditJson}
            onChange={(event) => setCurveEditJson(event.target.value)}
          />
          <button
            className="primary"
            onClick={() =>
              run(
                "apply_curve_edit",
                {
                  session_id: requireSession(),
                  edit: JSON.parse(curveEditJson)
                },
                setSessionSummary
              )
            }
          >
            Apply Curve Edit
          </button>
        </article>

        <article className="panel">
          <h2>Save</h2>
          <label>
            Save-as output directory
            <input
              value={saveAsPath}
              onChange={(event) => setSaveAsPath(event.target.value)}
              placeholder="C:\\path\\to\\copy.laspkg"
            />
          </label>
          <div className="button-row">
            <button
              className="primary"
              onClick={() =>
                run("save_session", { session_id: requireSession() }, setLastResponse)
              }
            >
              Save
            </button>
            <button
              onClick={() =>
                run(
                  "save_session_as",
                  { session_id: requireSession(), output_dir: saveAsPath },
                  setLastResponse
                )
              }
            >
              Save As
            </button>
          </div>
        </article>
      </section>

      <section className="grid output-grid">
        <article className="panel output-panel">
          <h2>Session Summary</h2>
          <pre data-testid="session-summary-output">{prettyJson(sessionSummary)}</pre>
        </article>
        <article className="panel output-panel">
          <h2>Session Metadata</h2>
          <pre data-testid="session-metadata-output">{prettyJson(sessionMetadata)}</pre>
        </article>
        <article className="panel output-panel">
          <h2>Curve Catalog / Dirty State / Validation</h2>
          <pre data-testid="diagnostics-output">
            {prettyJson({ catalog, dirtyState, validation })}
          </pre>
        </article>
        <article className="panel output-panel">
          <h2>Window Result</h2>
          <pre data-testid="window-output">{prettyJson(windowResult)}</pre>
        </article>
        <article className="panel output-panel">
          <h2>Last Command</h2>
          <pre data-testid="last-command-output">
            {prettyJson({ lastCommand, lastResponse, lastError })}
          </pre>
        </article>
      </section>
    </main>
  );
}
