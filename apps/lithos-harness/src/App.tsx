import { type Dispatch, type SetStateAction, useEffect, useMemo, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { Navigate, Route, Routes, useNavigate } from "react-router-dom";
import type {
  CommandErrorDto,
  CurveCatalogDto,
  CurveCatalogEntryDto,
  CurveWindowDto,
  PackageFilesViewDto,
  SessionMetadataDto,
  SessionSummaryDto,
  ValidationReportDto
} from "./api";
import { api } from "./api";
import {
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Input,
  Label,
  Menubar,
  TableShell,
  Textarea
} from "./components/ui";

type NodeId = "overview" | "metadata" | "curves" | "imports" | "diagnostics" | "files";
type Notice = { tone: "error" | "info"; title: string; detail: string } | null;
type RawState = {
  path: string;
  summary: unknown | null;
  metadata: unknown | null;
  catalog: CurveCatalogEntryDto[];
  validation: ValidationReportDto | null;
  window: CurveWindowDto | null;
};
type DraftWorkspace = { kind: "draft"; root: string; raw: RawState };
type SessionWorkspace = {
  kind: "session";
  root: string;
  session: SessionSummaryDto;
  validation: ValidationReportDto | null;
  raw: RawState;
};
type Workspace = DraftWorkspace | SessionWorkspace | null;

const RECENTS_KEY = "lithos-harness-recents";
const DEFAULT_LAS = "test_data\\logs\\6038187_v1.2_short.las";
const DEFAULT_CURVE = "DT";
const NAV: Array<{ id: NodeId; label: string; subtitle: string }> = [
  { id: "overview", label: "Overview", subtitle: "Session, root, revision, dirty state" },
  { id: "metadata", label: "Metadata", subtitle: "Canonical metadata inspector" },
  { id: "curves", label: "Curves", subtitle: "Catalog and editable sample table" },
  { id: "imports", label: "Imports", subtitle: "Preview and import raw LAS files" },
  { id: "diagnostics", label: "Diagnostics", subtitle: "Validation and command failures" },
  { id: "files", label: "Package Files", subtitle: "Read-only metadata.json and parquet schema" }
];

function emptyRaw(path = DEFAULT_LAS): RawState {
  return { path, summary: null, metadata: null, catalog: [], validation: null, window: null };
}

function loadRecents(): string[] {
  try {
    const raw = window.localStorage.getItem(RECENTS_KEY);
    const parsed = raw ? (JSON.parse(raw) as unknown) : [];
    return Array.isArray(parsed) ? parsed.filter((value): value is string => typeof value === "string") : [];
  } catch {
    return [];
  }
}

function pretty(value: unknown) {
  return JSON.stringify(value, null, 2);
}

function asText(value: unknown) {
  if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") return String(value);
  if (value && typeof value === "object" && "Text" in (value as Record<string, unknown>)) return String((value as { Text: unknown }).Text);
  if (value && typeof value === "object" && "Number" in (value as Record<string, unknown>)) return String((value as { Number: unknown }).Number);
  return "";
}

function parseValue(raw: string, storageKind?: string) {
  if (!raw.trim()) return "Empty";
  if (storageKind === "Numeric") {
    const number = Number(raw);
    if (Number.isFinite(number)) return { Number: number };
  }
  return { Text: raw };
}

function inferDepthRange(metadata: SessionMetadataDto | null, rowCount?: number) {
  const well = metadata?.metadata.metadata.well as
    | { start?: unknown; stop?: unknown; step?: unknown }
    | undefined;
  const start = typeof well?.start === "number" ? well.start : null;
  const stop = typeof well?.stop === "number" ? well.stop : null;
  const step = typeof well?.step === "number" ? well.step : null;
  const rows = typeof rowCount === "number" ? rowCount : null;

  let effectiveStop = stop;
  if (effectiveStop == null && start != null && step != null && rows != null && rows > 1) {
    effectiveStop = start + step * (rows - 1);
  }

  if (start == null || effectiveStop == null) {
    return null;
  }

  return {
    min: Math.min(start, effectiveStop),
    max: Math.max(start, effectiveStop)
  };
}

async function chooseFolder(title: string): Promise<string | null> {
  const result = await open({ directory: true, multiple: false, title });
  return typeof result === "string" ? result : null;
}

async function chooseLasFile(title: string): Promise<string | null> {
  const result = await open({
    multiple: false,
    title,
    filters: [{ name: "LAS", extensions: ["las"] }]
  });
  return typeof result === "string" ? result : null;
}

function AppShell() {
  const [recents, setRecents] = useState<string[]>(() => loadRecents());
  const [workspace, setWorkspace] = useState<Workspace>(null);
  const [node, setNode] = useState<NodeId>("overview");
  const [notice, setNotice] = useState<Notice>(null);
  const [menuAction, setMenuAction] = useState<string | null>(null);

  useEffect(() => {
    window.localStorage.setItem(RECENTS_KEY, JSON.stringify(recents.slice(0, 8)));
  }, [recents]);

  useEffect(() => {
    let dispose: (() => void) | undefined;
    listen<string>("menu-action", (event) => setMenuAction(event.payload)).then((unlisten) => {
      dispose = unlisten;
    });
    return () => dispose?.();
  }, []);

  function pushRecent(path: string) {
    setRecents((current) => [path, ...current.filter((value) => value !== path)].slice(0, 8));
  }

  return (
    <Routes>
      <Route
        path="/"
        element={
          <HomePage
            recents={recents}
            notice={notice}
            setWorkspace={setWorkspace}
            setNode={setNode}
            pushRecent={pushRecent}
            setNotice={setNotice}
            menuAction={menuAction}
            setMenuAction={setMenuAction}
          />
        }
      />
      <Route
        path="/workspace"
        element={
          workspace ? (
            <WorkspacePage
              workspace={workspace}
              setWorkspace={setWorkspace}
              node={node}
              setNode={setNode}
              notice={notice}
              setNotice={setNotice}
              pushRecent={pushRecent}
              menuAction={menuAction}
              setMenuAction={setMenuAction}
            />
          ) : (
            <Navigate to="/" replace />
          )
        }
      />
    </Routes>
  );
}

function HomePage({
  recents,
  notice,
  setWorkspace,
  setNode,
  pushRecent,
  setNotice,
  menuAction,
  setMenuAction
}: {
  recents: string[];
  notice: Notice;
  setWorkspace: Dispatch<SetStateAction<Workspace>>;
  setNode: (node: NodeId) => void;
  pushRecent: (path: string) => void;
  setNotice: (notice: Notice) => void;
  menuAction: string | null;
  setMenuAction: Dispatch<SetStateAction<string | null>>;
}) {
  const navigate = useNavigate();
  const [draftRoot, setDraftRoot] = useState("");
  const [packageRoot, setPackageRoot] = useState("");

  async function openPackage(path: string) {
    try {
      const session = await api.openPackageSession(path);
      const validation = await api.validatePackage(path);
      setWorkspace({ kind: "session", root: path, session, validation, raw: emptyRaw() });
      pushRecent(path);
      setNode("overview");
      setNotice(null);
      navigate("/workspace");
    } catch (error) {
      const commandError = error as CommandErrorDto;
      setNotice({
        tone: "error",
        title: commandError.kind ?? "Open failed",
        detail: commandError.message ?? "Unknown package open failure"
      });
    }
  }

  async function createWorkspaceFromRoot(folder: string) {
    if (!folder.trim()) return;
    setDraftRoot(folder);
    const lasPath = await chooseLasFile("Choose a LAS file to import into the new package");
    if (lasPath) {
      const session = await api.importLasIntoWorkspace(folder, lasPath, null);
      const validation = await api.validatePackage(session.root);
      setWorkspace({
        kind: "session",
        root: session.root,
        session,
        validation,
        raw: { ...emptyRaw(), path: lasPath }
      });
      pushRecent(session.root);
      setNode("overview");
      setNotice({
        tone: "info",
        title: "Package created",
        detail: `Imported ${lasPath} and opened a live session at ${session.root}`
      });
      navigate("/workspace");
      return;
    }

    setWorkspace({ kind: "draft", root: folder, raw: emptyRaw() });
    setNode("imports");
    setNotice({
      tone: "info",
      title: "Draft workspace",
      detail: "No LAS selected yet. Use Imports to create the package and start a live session."
    });
    navigate("/workspace");
  }

  async function createDraftFromDialog() {
    const folder = await chooseFolder("Choose a folder for the new package");
    if (!folder) return;
    await createWorkspaceFromRoot(folder);
  }

  async function openPackageFromDialog() {
    const folder = await chooseFolder("Open an existing package");
    if (!folder) return;
    setPackageRoot(folder);
    await openPackage(folder);
  }

  useEffect(() => {
    if (menuAction === "file.new-package") {
      void createDraftFromDialog().finally(() => setMenuAction(null));
    } else if (menuAction === "file.open-package") {
      void openPackageFromDialog().finally(() => setMenuAction(null));
    }
  }, [menuAction, setMenuAction]);

  return (
    <main className="min-h-screen bg-[radial-gradient(circle_at_top_left,rgba(209,165,88,0.18),transparent_28%),linear-gradient(180deg,#fbf7ef_0%,#efe4d0_100%)] px-6 py-8 text-stone-900 md:px-8">
      <div className="mx-auto flex max-w-7xl flex-col gap-6">
        <header className="grid gap-6 md:grid-cols-[minmax(0,1fr)_320px]">
          <div className="space-y-4">
            <Badge tone="accent">Package-first desktop shell</Badge>
            <h1 className="max-w-4xl text-4xl font-semibold tracking-tight md:text-6xl">
              Create or open a package, then work inside a live session-backed workspace.
            </h1>
            <p className="max-w-3xl text-base text-stone-600 md:text-lg">
              The home screen is package-focused. The workspace screen is the inspector: metadata,
              curves, diagnostics, imports, and package files.
            </p>
          </div>
          <Card>
            <CardHeader>
              <CardTitle>Terms</CardTitle>
              <CardDescription>Package, session, workspace each mean something different.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-3 text-sm text-stone-600">
              <p><strong className="text-stone-900">Package:</strong> the saved folder on disk.</p>
              <p><strong className="text-stone-900">Session:</strong> the live editable SDK state for one open package.</p>
              <p><strong className="text-stone-900">Workspace:</strong> the app shell around a draft or session.</p>
              <p><strong className="text-stone-900">Window query:</strong> a row window for chosen curves, instead of loading everything by default.</p>
            </CardContent>
          </Card>
        </header>
        {notice ? <NoticeCard notice={notice} /> : null}
        <section className="grid gap-6 lg:grid-cols-[1.1fr_1fr]">
          <Card>
            <CardHeader>
              <CardTitle>Create a package</CardTitle>
              <CardDescription>Choose a folder and optionally import a LAS file immediately to start a live session.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <Label>
                Draft package root
                <Input aria-label="Draft package root" value={draftRoot} onChange={(event) => setDraftRoot(event.target.value)} />
              </Label>
              <div className="flex gap-3">
                <Button data-testid="create-draft-button" onClick={() => void createDraftFromDialog()}>Choose Folder</Button>
                <Button variant="outline" onClick={() => void createWorkspaceFromRoot(draftRoot)}>Use Typed Path</Button>
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>Open an existing package</CardTitle>
              <CardDescription>Open a saved package into a live SDK session.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <Label>
                Package path
                <Input aria-label="Package path" value={packageRoot} onChange={(event) => setPackageRoot(event.target.value)} />
              </Label>
              <div className="flex gap-3">
                <Button data-testid="open-package-button" onClick={() => void openPackageFromDialog()}>Choose Folder</Button>
                <Button variant="outline" onClick={() => void openPackage(packageRoot)}>Use Typed Path</Button>
              </div>
            </CardContent>
          </Card>
        </section>
        <section className="grid gap-6 lg:grid-cols-[1.05fr_1fr]">
          <Card>
            <CardHeader>
              <CardTitle>Recent packages</CardTitle>
              <CardDescription>Stored locally for quick reopening.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-3">
              {recents.length === 0 ? (
                <p className="text-sm text-stone-600">No recent packages yet.</p>
              ) : (
                recents.map((path) => (
                  <div key={path} className="flex items-center justify-between gap-3 rounded-2xl border border-stone-300 bg-white/70 p-3">
                    <div className="min-w-0">
                      <p className="truncate font-medium">{path}</p>
                      <p className="text-sm text-stone-600">Open directly into the workspace.</p>
                    </div>
                    <Button variant="secondary" onClick={() => void openPackage(path)}>Open</Button>
                  </div>
                ))
              )}
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>Workflow</CardTitle>
              <CardDescription>Closest mental model: Figma file chooser plus a domain inspector.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-3 text-sm text-stone-600">
              <p>1. Select a folder for a new package or open an existing one.</p>
              <p>2. If it is a draft, import a LAS file to create package files and start a session.</p>
              <p>3. Inspect metadata, curves, diagnostics, and raw package files.</p>
              <p>4. Save or Save As from the native File menu or the visible toolbar.</p>
            </CardContent>
          </Card>
        </section>
      </div>
    </main>
  );
}

function WorkspacePage({
  workspace,
  setWorkspace,
  node,
  setNode,
  notice,
  setNotice,
  pushRecent,
  menuAction,
  setMenuAction
}: {
  workspace: Workspace;
  setWorkspace: Dispatch<SetStateAction<Workspace>>;
  node: NodeId;
  setNode: (node: NodeId) => void;
  notice: Notice;
  setNotice: (notice: Notice) => void;
  pushRecent: (path: string) => void;
  menuAction: string | null;
  setMenuAction: Dispatch<SetStateAction<string | null>>;
}) {
  const navigate = useNavigate();
  const session = workspace?.kind === "session" ? workspace.session : null;
  const [company, setCompany] = useState("HARNESS EDIT");
  const [otherText, setOtherText] = useState("");
  const [catalog, setCatalog] = useState<CurveCatalogDto | null>(null);
  const [sessionMetadata, setSessionMetadata] = useState<SessionMetadataDto | null>(null);
  const [selectedCurve, setSelectedCurve] = useState(DEFAULT_CURVE);
  const [windowData, setWindowData] = useState<CurveWindowDto | null>(null);
  const [editedValues, setEditedValues] = useState<string[]>([]);
  const [depthMin, setDepthMin] = useState("");
  const [depthMax, setDepthMax] = useState("");

  async function run(action: () => Promise<void>) {
    try {
      await action();
      setNotice(null);
    } catch (error) {
      const commandError = error as CommandErrorDto;
      setNotice({
        tone: "error",
        title: commandError.kind ?? "Command failed",
        detail: commandError.message ?? "Unknown command failure"
      });
    }
  }

  async function refresh() {
    if (!session || workspace?.kind !== "session") return;
    const [sessionSummary, nextMetadata, validation, nextCatalog] = await Promise.all([
      api.sessionSummary(session.session_id),
      api.sessionMetadata(session.session_id),
      api.validatePackage(session.root),
      api.sessionCurveCatalog(session.session_id)
    ]);
    setWorkspace((current) =>
      current && current.kind === "session"
        ? { ...current, session: sessionSummary, validation, root: sessionSummary.root }
        : current
    );
    setSessionMetadata(nextMetadata);
    setCompany(String(nextMetadata.metadata.metadata.well.company ?? "HARNESS EDIT"));
    setOtherText(String((nextMetadata.metadata.metadata as { other?: unknown }).other ?? ""));
    const range = inferDepthRange(nextMetadata, sessionSummary.summary?.summary?.row_count);
    if (range) {
      setDepthMin(String(range.min));
      setDepthMax(String(range.max));
    }
    setCatalog(nextCatalog);
    pushRecent(sessionSummary.root);
  }

  async function importLasIntoWorkspace() {
    if (!workspace) return;
    const lasPath =
      workspace.raw.path.trim() || (await chooseLasFile("Choose a LAS file to import")) || "";
    if (!lasPath) return;
    const imported = await api.importLasIntoWorkspace(
      workspace.root,
      lasPath,
      session?.session_id ?? null
    );
    const validation = await api.validatePackage(imported.root);
    setWorkspace({ kind: "session", root: imported.root, session: imported, validation, raw: { ...workspace.raw, path: lasPath } });
    setNode("overview");
    pushRecent(imported.root);
    setNotice({ tone: "info", title: "LAS imported", detail: `Imported ${lasPath} into ${imported.root}` });
  }

  async function startNewWorkspaceFromFolder(folder: string) {
    if (!folder.trim()) return;
    const lasPath = await chooseLasFile("Choose a LAS file to import into the new package");
    if (lasPath) {
      const imported = await api.importLasIntoWorkspace(folder, lasPath, null);
      const validation = await api.validatePackage(imported.root);
      setWorkspace({
        kind: "session",
        root: imported.root,
        session: imported,
        validation,
        raw: { ...emptyRaw(), path: lasPath }
      });
      setNode("overview");
      pushRecent(imported.root);
      setNotice({
        tone: "info",
        title: "Package created",
        detail: `Imported ${lasPath} and opened a live session at ${imported.root}`
      });
      return;
    }

    setWorkspace({ kind: "draft", root: folder, raw: emptyRaw() });
    setNode("imports");
    setNotice({
      tone: "info",
      title: "Draft workspace",
      detail: "No LAS selected yet. Use Imports to create the package and start a live session."
    });
  }

  async function saveSession() {
    if (!session) throw { kind: "Draft", message: "Save requires an open package session." };
    await api.saveSession(session.session_id);
    await refresh();
    setNotice({ tone: "info", title: "Session saved", detail: `Saved changes back to ${session.root}` });
  }

  async function saveSessionAs() {
    if (!session) throw { kind: "Draft", message: "Save As requires an open package session." };
    const outputDir = await chooseFolder("Choose a destination folder for Save As");
    if (!outputDir) return;
    const result = await api.saveSessionAs(session.session_id, outputDir);
    const reopened = await api.openPackageSession(result.root);
    const validation = await api.validatePackage(result.root);
    setWorkspace((current) => current ? { kind: "session", root: result.root, session: reopened, validation, raw: current.raw } : current);
    pushRecent(result.root);
    setNotice({ tone: "info", title: "Session rebound", detail: `Save As kept the session and rebound it to ${result.root}` });
  }

  async function inspectRaw(kind: "summary" | "metadata" | "catalog" | "validation" | "window") {
    const path = workspace?.raw.path ?? DEFAULT_LAS;
    if (kind === "summary") {
      const summary = await api.inspectLasSummary(path);
      setWorkspace((current) => current ? { ...current, raw: { ...current.raw, summary } } : current);
      return;
    }
    if (kind === "metadata") {
      const metadata = await api.inspectLasMetadata(path);
      setWorkspace((current) => current ? { ...current, raw: { ...current.raw, metadata } } : current);
      return;
    }
    if (kind === "catalog") {
      const nextCatalog = await api.inspectLasCurveCatalog(path);
      setWorkspace((current) => current ? { ...current, raw: { ...current.raw, catalog: nextCatalog } } : current);
      return;
    }
    if (kind === "validation") {
      const validation = await api.validateLas(path);
      setWorkspace((current) => current ? { ...current, raw: { ...current.raw, validation } } : current);
      return;
    }
    const nextWindow = await api.inspectLasWindow(path, { curve_names: [selectedCurve], start_row: 0, row_count: 16 });
    setWorkspace((current) => current ? { ...current, raw: { ...current.raw, window: nextWindow } } : current);
  }

  useEffect(() => {
    if (!session) return;
    Promise.all([
      api.sessionMetadata(session.session_id),
      api.sessionCurveCatalog(session.session_id)
    ])
      .then(([nextMetadata, nextCatalog]) => {
        setSessionMetadata(nextMetadata);
        setCompany(String(nextMetadata.metadata.metadata.well.company ?? "HARNESS EDIT"));
        setOtherText(String((nextMetadata.metadata.metadata as { other?: unknown }).other ?? ""));
        const nextSelectedCurve =
          nextCatalog.curves.find((curve) => !curve.is_index)?.name ??
          nextCatalog.curves[0]?.name ??
          DEFAULT_CURVE;
        setCatalog(nextCatalog);
        setSelectedCurve(nextSelectedCurve);
        const range = inferDepthRange(nextMetadata, session.summary?.summary?.row_count);
        if (range) {
          setDepthMin(String(range.min));
          setDepthMax(String(range.max));
        }
      })
      .catch(() => {
        setSessionMetadata(null);
        setCatalog(null);
      });
  }, [session?.session_id]);

  useEffect(() => {
    if (!session || !selectedCurve) return;
    const curveNames = catalog?.curves.find((curve) => curve.is_index)?.name
      ? [catalog?.curves.find((curve) => curve.is_index)?.name ?? "", selectedCurve].filter(Boolean)
      : [selectedCurve];
    const parsedMin = Number(depthMin);
    const parsedMax = Number(depthMax);
    const request = Number.isFinite(parsedMin) && Number.isFinite(parsedMax)
      ? api.readDepthWindow(session.session_id, { curve_names: curveNames, depth_min: parsedMin, depth_max: parsedMax })
      : api.readCurveWindow(session.session_id, { curve_names: curveNames, start_row: 0, row_count: 64 });
    request.then((result) => {
      setWindowData(result.window);
      const selectedColumn = result.window.columns.find((column) => column.name === selectedCurve);
      setEditedValues((selectedColumn?.values ?? []).map(asText));
    }).catch(() => {
      setWindowData(null);
      setEditedValues([]);
    });
  }, [catalog, depthMax, depthMin, selectedCurve, session?.session_id]);

  useEffect(() => {
    if (menuAction === "file.import-las") {
      void run(importLasIntoWorkspace).finally(() => setMenuAction(null));
    } else if (menuAction === "file.save") {
      void run(saveSession).finally(() => setMenuAction(null));
    } else if (menuAction === "file.save-as") {
      void run(saveSessionAs).finally(() => setMenuAction(null));
    } else if (menuAction === "file.close-workspace") {
      void run(async () => {
        if (session) await api.closeSession(session.session_id);
        setWorkspace(null);
        navigate("/");
      }).finally(() => setMenuAction(null));
    } else if (menuAction === "file.new-package") {
      void (async () => {
        const folder = await chooseFolder("Choose a folder for the new package");
        if (folder) {
          await startNewWorkspaceFromFolder(folder);
        }
      })().finally(() => setMenuAction(null));
    } else if (menuAction === "file.open-package") {
      navigate("/");
      setMenuAction(null);
    }
  }, [menuAction, navigate, session?.session_id, setMenuAction]);

  const metadataRows = useMemo<Array<[string, unknown]>>(
    () =>
      workspace?.kind === "session"
        ? [
            ["Company", company],
            ["Root", workspace.root],
            ["Session", workspace.session.session_id],
            ["Revision", workspace.session.revision]
          ]
        : [
            ["Draft root", workspace?.root ?? ""],
            ["Status", "No package session yet"]
          ],
    [company, workspace]
  );

  async function applyMetadataEdit() {
    if (!session) throw { kind: "Draft", message: "Metadata editing requires an open package session." };
    await api.applyMetadataEdit(session.session_id, {
      items: [{ section: "Well", mnemonic: "COMP", unit: "", value: { Text: company }, description: "COMPANY" }],
      other: otherText.trim() ? otherText : null
    });
    await refresh();
  }

  async function applyCurveEdit() {
    if (!session || !windowData) throw { kind: "Draft", message: "Curve editing requires an open package session." };
    const descriptor = catalog?.curves.find((curve) => curve.name === selectedCurve);
    if (!descriptor) return;
    await api.applyCurveEdit(session.session_id, {
      Upsert: {
        mnemonic: descriptor.name ?? selectedCurve,
        original_mnemonic: descriptor.original_mnemonic ?? descriptor.name ?? selectedCurve,
        unit: descriptor.unit ?? "",
        header_value: "Empty",
        description: descriptor.description ?? "",
        data: editedValues.map((value) => parseValue(value, descriptor.storage_kind))
      }
    });
    await refresh();
  }

  return (
    <main className="min-h-screen bg-[radial-gradient(circle_at_top_left,rgba(209,165,88,0.18),transparent_28%),linear-gradient(180deg,#fbf7ef_0%,#efe4d0_100%)] px-4 py-4 text-stone-900 md:px-6">
      <div className="mx-auto flex max-w-[1600px] flex-col gap-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="space-y-1">
            <Badge tone={workspace?.kind === "session" ? "accent" : "default"}>
              {workspace?.kind === "session" ? "Live package session" : "Draft workspace"}
            </Badge>
            <h1 className="text-3xl font-semibold tracking-tight">{workspace?.root}</h1>
            <p className="text-sm text-stone-600">
              {workspace?.kind === "session"
                ? "Shared SDK session with save, save-as, metadata edits, and curve-table reads."
                : "Draft workspace. Use Imports to preview a LAS source and create the package."}
            </p>
          </div>
          <div className="rounded-2xl border border-stone-300 bg-white/80 px-4 py-3 text-sm shadow-sm">
            <div className="font-medium">{session ? session.session_id : "no session"}</div>
            <div className="text-stone-600">
              {session?.dirty.has_unsaved_changes ? "Unsaved changes present" : "Clean or draft"}
            </div>
          </div>
        </div>
        <Menubar aria-label="Workspace menubar">
          <Button variant="ghost" onClick={() => navigate("/")}>Packages</Button>
          <Button variant="ghost" onClick={() => void run(saveSession)}>Save</Button>
          <Button variant="ghost" onClick={() => void run(saveSessionAs)}>Save As</Button>
          <Button variant="ghost" onClick={() => void run(importLasIntoWorkspace)}>Import LAS</Button>
          <Button variant="ghost" onClick={() => void run(async () => { if (session) await api.closeSession(session.session_id); setWorkspace(null); navigate("/"); })}>Close Workspace</Button>
        </Menubar>
        {notice ? <NoticeCard notice={notice} /> : null}
        <div className="grid gap-4 xl:grid-cols-[280px_minmax(0,1fr)]">
          <Card>
            <CardHeader className="pb-3">
              <CardTitle>Package Browser</CardTitle>
              <CardDescription>Logical package tree for the current workspace.</CardDescription>
            </CardHeader>
            <CardContent className="grid gap-2">
              {NAV.map((item) => (
                <button key={item.id} className={`rounded-2xl border px-4 py-3 text-left transition ${node === item.id ? "border-amber-700 bg-amber-50" : "border-transparent bg-white/60 hover:border-stone-300 hover:bg-white"}`} onClick={() => setNode(item.id)}>
                  <div className="font-medium">{item.label}</div>
                  <div className="text-sm text-stone-600">{item.subtitle}</div>
                </button>
              ))}
            </CardContent>
          </Card>
          <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_340px]">
            <Card className="overflow-hidden">
              <CardHeader className="border-b border-stone-200 bg-white/70">
                <CardTitle>{NAV.find((item) => item.id === node)?.label}</CardTitle>
                <CardDescription>{NAV.find((item) => item.id === node)?.subtitle}</CardDescription>
              </CardHeader>
              <CardContent className="p-0">
                {node === "overview" ? <OverviewPanel workspace={workspace} /> : null}
                {node === "metadata" ? <MetadataPanel rows={metadataRows} company={company} setCompany={setCompany} otherText={otherText} setOtherText={setOtherText} onApply={() => void run(applyMetadataEdit)} /> : null}
                {node === "curves" ? <CurvesPanel catalog={catalog?.curves ?? []} window={windowData} selectedCurve={selectedCurve} setSelectedCurve={setSelectedCurve} editedValues={editedValues} setEditedValues={setEditedValues} depthMin={depthMin} setDepthMin={setDepthMin} depthMax={depthMax} setDepthMax={setDepthMax} onApply={() => void run(applyCurveEdit)} /> : null}
                {node === "imports" ? <ImportsPanel raw={workspace?.raw ?? emptyRaw()} onPathChange={(path) => setWorkspace((current) => current ? { ...current, raw: { ...current.raw, path } } : current)} onChoosePath={async () => { const lasPath = await chooseLasFile("Choose a LAS file to preview"); if (lasPath) { setWorkspace((current) => current ? { ...current, raw: { ...current.raw, path: lasPath } } : current); } }} onInspect={(kind) => void run(() => inspectRaw(kind))} onImport={() => void run(importLasIntoWorkspace)} /> : null}
                {node === "diagnostics" ? <DiagnosticsPanel validation={workspace?.kind === "session" ? workspace.validation : workspace?.raw.validation ?? null} /> : null}
                {node === "files" ? <FilesPanel path={workspace?.root ?? ""} /> : null}
              </CardContent>
            </Card>
            <Card>
              <CardHeader>
                <CardTitle>Inspector</CardTitle>
                <CardDescription>Always-visible session, package, and workspace state.</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="grid gap-3 rounded-2xl border border-stone-300 bg-white/70 p-4 text-sm">
                  <div><div className="font-medium">Workspace root</div><div className="break-all text-stone-600">{workspace?.root ?? "Not selected"}</div></div>
                  {session ? <><div><div className="font-medium">Session id</div><div className="break-all text-stone-600">{session.session_id}</div></div><div><div className="font-medium">Revision</div><div className="break-all text-stone-600">{session.revision}</div></div></> : <div className="text-stone-600">No SDK session yet.</div>}
                </div>
                <div className="grid gap-3">
                  {session ? <Button variant="outline" onClick={() => void run(refresh)}>Refresh Session</Button> : null}
                  <Button variant="secondary" onClick={() => setNode("imports")}>Go to Imports</Button>
                </div>
              </CardContent>
            </Card>
          </div>
        </div>
      </div>
    </main>
  );
}

function NoticeCard({ notice }: { notice: Exclude<Notice, null> }) {
  return <Card className={notice.tone === "error" ? "border-amber-300" : "border-sky-300"}><CardContent className="flex items-start gap-4 px-6 py-4"><Badge tone={notice.tone === "error" ? "warn" : "accent"}>{notice.tone === "error" ? "Problem" : "Status"}</Badge><div className="space-y-1"><div className="font-medium">{notice.title}</div><p className="text-sm text-stone-600">{notice.detail}</p></div></CardContent></Card>;
}

function OverviewPanel({ workspace }: { workspace: Workspace }) {
  const rows = workspace?.kind === "session" ? [["Package root", workspace.root], ["Session id", workspace.session.session_id], ["Revision", workspace.session.revision], ["Dirty", workspace.session.dirty.has_unsaved_changes ? "true" : "false"], ["Curve count", String(workspace.session.summary?.summary?.curve_count ?? "")], ["Row count", String(workspace.session.summary?.summary?.row_count ?? "")]] : [["Draft root", workspace?.root ?? ""], ["Status", "No package session yet"], ["Next step", "Import a LAS file to create the package"]];
  return <TableShell><table className="w-full border-collapse text-sm"><tbody>{rows.map(([label, value]) => <tr key={label} className="border-b border-stone-200 last:border-b-0"><th className="w-56 bg-stone-100/70 px-4 py-3 text-left font-medium">{label}</th><td className="px-4 py-3 text-stone-600">{value}</td></tr>)}</tbody></table></TableShell>;
}

function MetadataPanel({ rows, company, setCompany, otherText, setOtherText, onApply }: { rows: Array<[string, unknown]>; company: string; setCompany: (value: string) => void; otherText: string; setOtherText: (value: string) => void; onApply: () => void; }) {
  return <div className="grid gap-4 p-4 lg:grid-cols-[minmax(0,1fr)_320px]"><TableShell><table className="w-full border-collapse text-sm"><thead className="bg-stone-100/80"><tr><th className="px-4 py-3 text-left font-medium">Field</th><th className="px-4 py-3 text-left font-medium">Value</th></tr></thead><tbody>{rows.map(([key, value]) => <tr key={key} className="border-t border-stone-200"><td className="px-4 py-3 font-medium">{key}</td><td className="px-4 py-3 text-stone-600">{asText(value)}</td></tr>)}</tbody></table></TableShell><div className="space-y-4 rounded-[22px] border border-stone-300 bg-white/70 p-4"><div><h3 className="font-medium">Metadata inspector</h3><p className="text-sm text-stone-600">First pass editor for the canonical `COMP` field and optional OTHER text.</p></div><Label>Company value<Input aria-label="Company value" value={company} onChange={(event) => setCompany(event.target.value)} /></Label><Label>Replace OTHER text<Textarea aria-label="Replace OTHER text" value={otherText} onChange={(event) => setOtherText(event.target.value)} /></Label><Button data-testid="metadata-apply-button" onClick={onApply}>Apply Metadata Edit</Button></div></div>;
}

function CurvesPanel({ catalog, window, selectedCurve, setSelectedCurve, editedValues, setEditedValues, depthMin, setDepthMin, depthMax, setDepthMax, onApply }: { catalog: CurveCatalogEntryDto[]; window: CurveWindowDto | null; selectedCurve: string; setSelectedCurve: (value: string) => void; editedValues: string[]; setEditedValues: Dispatch<SetStateAction<string[]>>; depthMin: string; setDepthMin: (value: string) => void; depthMax: string; setDepthMax: (value: string) => void; onApply: () => void; }) {
  const selectedColumn = window?.columns.find((column) => column.name === selectedCurve);
  return <div className="grid gap-4 p-4"><div className="grid gap-4 lg:grid-cols-[320px_minmax(0,1fr)]"><Card className="shadow-none"><CardHeader><CardTitle>Curve catalog</CardTitle><CardDescription>Select a curve and depth interval for the log-track table view.</CardDescription></CardHeader><CardContent className="space-y-3"><div className="grid gap-2">{catalog.length === 0 ? <p className="text-sm text-stone-600">Open a session to load the curve catalog.</p> : catalog.map((curve) => <button key={curve.curve_id ?? curve.name ?? curve.mnemonic} className={`rounded-2xl border px-3 py-2 text-left text-sm ${selectedCurve === (curve.name ?? curve.mnemonic ?? "") ? "border-amber-700 bg-amber-50" : "border-stone-300 bg-white/80 hover:bg-stone-100"}`} onClick={() => setSelectedCurve(curve.name ?? curve.mnemonic ?? "")}><div className="font-medium">{curve.name ?? curve.mnemonic}</div><div className="text-xs text-stone-600">{curve.canonical_name ?? curve.original_mnemonic ?? "curve"}</div></button>)}</div><div className="grid gap-3 rounded-2xl border border-stone-300 bg-stone-50/70 p-4 text-sm"><Label>Depth min<Input aria-label="Depth min" value={depthMin} onChange={(event) => setDepthMin(event.target.value)} /></Label><Label>Depth max<Input aria-label="Depth max" value={depthMax} onChange={(event) => setDepthMax(event.target.value)} /></Label><p className="text-stone-600">Depth-range reads are preferred. If no valid range is available, the app falls back to a row window.</p></div></CardContent></Card><TableShell><div className="overflow-auto"><table className="min-w-full border-collapse text-sm"><thead className="bg-stone-100/80"><tr>{(window?.columns ?? []).map((column) => <th key={column.name} className="border-l border-stone-200 px-4 py-3 text-left font-medium first:border-l-0">{column.name}</th>)}</tr></thead><tbody>{window && window.columns.length > 0 ? Array.from({ length: window.row_count }).map((_, index) => <tr key={index} className="border-t border-stone-200">{window.columns.map((column) => <td key={`${column.name}-${index}`} className="border-l border-stone-200 px-4 py-3 text-stone-600 first:border-l-0">{column.name === selectedCurve ? <Input value={editedValues[index] ?? ""} onChange={(event) => setEditedValues((current) => { const next = [...current]; next[index] = event.target.value; return next; })} /> : asText(column.values[index])}</td>)}</tr>) : <tr><td className="px-4 py-4 text-stone-600">Select a curve to populate the table.</td></tr>}</tbody></table></div></TableShell></div><Card className="shadow-none"><CardHeader><CardTitle>Curve edit inspector</CardTitle><CardDescription>Edits the selected curve using the current depth-filtered table state.</CardDescription></CardHeader><CardContent className="space-y-4"><div className="rounded-2xl border border-stone-300 bg-stone-50/70 p-4 text-sm text-stone-600">Selected curve: <strong className="text-stone-900">{selectedCurve || "none"}</strong><br />Editable rows loaded: {selectedColumn?.values.length ?? 0}<br />Depth range: {depthMin || "?"} to {depthMax || "?"}</div><Button data-testid="curve-apply-button" onClick={onApply}>Apply Curve Edit</Button></CardContent></Card></div>;
}

function ImportsPanel({ raw, onPathChange, onChoosePath, onInspect, onImport }: { raw: RawState; onPathChange: (path: string) => void; onChoosePath: () => void; onInspect: (kind: "summary" | "metadata" | "catalog" | "validation" | "window") => void; onImport: () => void; }) {
  return <div className="grid gap-4 p-4 lg:grid-cols-[360px_minmax(0,1fr)]"><Card className="shadow-none"><CardHeader><CardTitle>Import LAS</CardTitle><CardDescription>Preview a raw LAS file, then import it into the current draft or open workspace.</CardDescription></CardHeader><CardContent className="space-y-4"><Label>LAS source path<Input aria-label="LAS source path" value={raw.path} onChange={(event) => onPathChange(event.target.value)} /></Label><div className="flex flex-wrap gap-3"><Button data-testid="choose-las-button" onClick={onChoosePath}>Choose LAS</Button><Button variant="outline" data-testid="import-las-button" onClick={onImport}>Import Into Workspace</Button></div><div className="flex flex-wrap gap-3"><Button onClick={() => onInspect("summary")}>LAS Summary</Button><Button variant="secondary" onClick={() => onInspect("metadata")}>LAS Metadata</Button><Button variant="secondary" onClick={() => onInspect("catalog")}>LAS Curves</Button><Button variant="outline" onClick={() => onInspect("validation")}>Validate LAS</Button><Button variant="outline" onClick={() => onInspect("window")}>LAS Window</Button></div></CardContent></Card><TableShell><div className="grid gap-4 p-4 md:grid-cols-2"><InfoBlock title="Summary" value={raw.summary} /><InfoBlock title="Metadata" value={raw.metadata} /><InfoBlock title="Curves" value={raw.catalog} /><InfoBlock title="Diagnostics" value={raw.validation} /><InfoBlock title="Window preview" value={raw.window} className="md:col-span-2" /></div></TableShell></div>;
}

function DiagnosticsPanel({ validation }: { validation: ValidationReportDto | null }) {
  const issues = validation?.issues ?? [];
  return <div className="grid gap-4 p-4 lg:grid-cols-[minmax(0,1fr)_320px]"><TableShell><table className="w-full border-collapse text-sm"><thead className="bg-stone-100/80"><tr><th className="px-4 py-3 text-left font-medium">Code</th><th className="px-4 py-3 text-left font-medium">Severity</th><th className="px-4 py-3 text-left font-medium">Message</th></tr></thead><tbody>{issues.length === 0 ? <tr><td className="px-4 py-4 text-stone-600" colSpan={3}>{validation ? "No structured issues for this package." : "No validation results yet."}</td></tr> : issues.map((issue) => <tr key={`${issue.code}-${issue.message}`} className="border-t border-stone-200"><td className="px-4 py-3 font-mono text-xs">{issue.code}</td><td className="px-4 py-3">{issue.severity}</td><td className="px-4 py-3 text-stone-600">{issue.message}</td></tr>)}</tbody></table></TableShell><InfoBlock title="Validation payload" value={validation} /></div>;
}

function FilesPanel({ path }: { path: string }) {
  const [files, setFiles] = useState<PackageFilesViewDto | null>(null);

  useEffect(() => {
    if (!path) return;
    api.readPackageFiles(path).then(setFiles).catch(() => setFiles(null));
  }, [path]);

  return <div className="grid gap-4 p-4 lg:grid-cols-[320px_minmax(0,1fr)]"><Card className="shadow-none"><CardHeader><CardTitle>Package Files</CardTitle><CardDescription>Read-only package storage details for the current root.</CardDescription></CardHeader><CardContent className="space-y-3 text-sm"><div><div className="font-medium">metadata.json</div><div className="break-all text-stone-600">{files?.metadata_path ?? `${path}\\metadata.json`}</div></div><div><div className="font-medium">curves.parquet</div><div className="break-all text-stone-600">{files?.parquet_path ?? `${path}\\curves.parquet`}</div></div><div><div className="font-medium">Rows</div><div className="text-stone-600">{files?.row_count ?? "-"}</div></div><div><div className="font-medium">Columns</div><div className="text-stone-600">{files?.curve_count ?? 0}</div></div></CardContent></Card><TableShell><div className="grid h-full lg:grid-cols-[280px_minmax(0,1fr)]"><div className="border-r border-stone-200 bg-stone-50/70"><div className="border-b border-stone-200 px-4 py-3 font-medium">Schema Information</div><div className="max-h-[640px] overflow-auto">{files?.columns?.map((column) => <div key={column.name} className="border-b border-stone-200 px-4 py-3 text-sm"><div className="font-medium">{column.name}</div><div className="text-xs text-stone-600">{column.storage_kind} | {column.unit || "no unit"} | {column.is_index ? "index" : "curve"}</div></div>)}</div></div><div className="min-w-0"><div className="border-b border-stone-200 px-4 py-3 font-medium">metadata.json</div><pre className="max-h-[640px] overflow-auto p-4 text-xs">{files?.metadata_json ?? "No package files written yet."}</pre></div></div></TableShell></div>;
}

function InfoBlock({ title, value, className = "" }: { title: string; value: unknown; className?: string; }) {
  return <Card className={className}><CardHeader><CardTitle className="text-base">{title}</CardTitle></CardHeader><CardContent><pre className="max-h-[320px] overflow-auto rounded-2xl bg-stone-100/80 p-4 text-xs">{pretty(value)}</pre></CardContent></Card>;
}

export default function App() {
  return <AppShell />;
}
