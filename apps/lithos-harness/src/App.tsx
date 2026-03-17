import { type Dispatch, type ReactNode, type SetStateAction, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { Navigate, Route, Routes, useNavigate } from "react-router-dom";
import type {
  AssetBindingInput,
  AssetCollectionRecord,
  AssetRecord,
  CommandErrorDto,
  CurveCatalogDto,
  CurveWindowDto,
  DrillingObservationRow,
  PackageFilesViewDto,
  PressureObservationRow,
  ProjectAssetImportResult,
  ProjectSummaryDto,
  SessionMetadataDto,
  SessionSummaryDto,
  TopRow,
  TrajectoryRow,
  WellRecord,
  WellboreRecord
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

type Notice = { tone: "error" | "info"; title: string; detail: string } | null;
type ViewId = "overview" | "imports" | "coverage" | "asset";
type ImportKind = "log" | "trajectory" | "tops" | "pressure" | "drilling";

type LogDetail = {
  kind: "log";
  asset: AssetRecord;
  session: SessionSummaryDto;
  metadata: SessionMetadataDto;
  catalog: CurveCatalogDto;
  window: CurveWindowDto | null;
  files: PackageFilesViewDto | null;
};

type StructuredDetail =
  | { kind: "trajectory"; asset: AssetRecord; rows: TrajectoryRow[] }
  | { kind: "tops"; asset: AssetRecord; rows: TopRow[] }
  | { kind: "pressure"; asset: AssetRecord; rows: PressureObservationRow[] }
  | { kind: "drilling"; asset: AssetRecord; rows: DrillingObservationRow[] };

type AssetDetail = LogDetail | StructuredDetail | null;

type ProjectWorkspace = {
  project: ProjectSummaryDto;
  wells: WellRecord[];
  wellbores: WellboreRecord[];
  collections: AssetCollectionRecord[];
  assets: AssetRecord[];
  selectedWellId: string | null;
  selectedWellboreId: string | null;
  selectedAssetId: string | null;
  view: ViewId;
  detail: AssetDetail;
  coverage: AssetRecord[];
  coverageMin: string;
  coverageMax: string;
  importKind: ImportKind;
  importPath: string;
  importCollectionName: string;
  importBinding: AssetBindingInput;
};

const RECENTS_KEY = "lithos-harness-project-recents";
const DEFAULT_PATHS: Record<ImportKind, string> = {
  log: "test_data\\logs\\6038187_v1.2_short.las",
  trajectory: "test_data\\trajectory.csv",
  tops: "test_data\\tops.csv",
  pressure: "test_data\\pressure.csv",
  drilling: "test_data\\drilling.csv"
};

function defaultBinding(): AssetBindingInput {
  return {
    well_name: "Well A",
    wellbore_name: "Main Bore",
    uwi: "",
    api: "",
    operator_aliases: []
  };
}

function pretty(value: unknown) {
  return JSON.stringify(value, null, 2);
}

function asText(value: unknown) {
  if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") return String(value);
  if (value && typeof value === "object" && "Text" in (value as Record<string, unknown>)) return String((value as { Text: unknown }).Text);
  if (value && typeof value === "object" && "Number" in (value as Record<string, unknown>)) return String((value as { Number: unknown }).Number);
  return value == null ? "" : String(value);
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

function assetKindLabel(kind: string) {
  switch (kind) {
    case "Log":
      return "Log";
    case "Trajectory":
      return "Trajectory";
    case "TopSet":
      return "Tops";
    case "PressureObservation":
      return "Pressure";
    case "DrillingObservation":
      return "Drilling";
    default:
      return kind;
  }
}

function operatorAliases(raw: string) {
  return raw
    .split(",")
    .map((value) => value.trim())
    .filter(Boolean);
}

function inferDepthRange(metadata: SessionMetadataDto, rowCount?: number) {
  const well = metadata.metadata.metadata.well as { start?: unknown; stop?: unknown; step?: unknown };
  const start = typeof well?.start === "number" ? well.start : null;
  const stop = typeof well?.stop === "number" ? well.stop : null;
  const step = typeof well?.step === "number" ? well.step : null;
  if (start == null) return null;
  if (stop != null) return { min: Math.min(start, stop), max: Math.max(start, stop) };
  if (step != null && rowCount && rowCount > 1) {
    const computed = start + step * (rowCount - 1);
    return { min: Math.min(start, computed), max: Math.max(start, computed) };
  }
  return null;
}

async function chooseFolder(title: string) {
  const result = await open({ directory: true, multiple: false, title });
  return typeof result === "string" ? result : null;
}

async function chooseFile(title: string, extensions: string[]) {
  const result = await open({ multiple: false, title, filters: [{ name: extensions.join(", "), extensions }] });
  return typeof result === "string" ? result : null;
}

async function loadProjectWorkspace(project: ProjectSummaryDto, current?: Partial<ProjectWorkspace>): Promise<ProjectWorkspace> {
  const wells = await api.listProjectWells(project.root);
  const selectedWellId =
    current?.selectedWellId && wells.some((item) => item.id === current.selectedWellId)
      ? current.selectedWellId
      : (wells[0]?.id ?? null);
  const wellbores = selectedWellId ? await api.listProjectWellbores(project.root, selectedWellId) : [];
  const selectedWellboreId =
    current?.selectedWellboreId && wellbores.some((item) => item.id === current.selectedWellboreId)
      ? current.selectedWellboreId
      : (wellbores[0]?.id ?? null);
  const [collections, assets] = selectedWellboreId
    ? await Promise.all([
        api.listProjectAssetCollections(project.root, selectedWellboreId),
        api.listProjectAssets(project.root, selectedWellboreId)
      ])
    : [[], []];
  return {
    project,
    wells,
    wellbores,
    collections,
    assets,
    selectedWellId,
    selectedWellboreId,
    selectedAssetId: current?.selectedAssetId && assets.some((item) => item.id === current.selectedAssetId) ? current.selectedAssetId : (assets[0]?.id ?? null),
    view: current?.view ?? "overview",
    detail: null,
    coverage: current?.coverage ?? [],
    coverageMin: current?.coverageMin ?? "",
    coverageMax: current?.coverageMax ?? "",
    importKind: current?.importKind ?? "log",
    importPath: current?.importPath ?? DEFAULT_PATHS.log,
    importCollectionName: current?.importCollectionName ?? "",
    importBinding: current?.importBinding ?? defaultBinding()
  };
}

async function loadAssetDetail(workspace: ProjectWorkspace): Promise<AssetDetail> {
  const asset = workspace.assets.find((item) => item.id === workspace.selectedAssetId);
  if (!asset) return null;
  if (asset.asset_kind === "Log") {
    const session = await api.openPackageSession(asset.package_path);
    const [metadata, catalog, files] = await Promise.all([
      api.sessionMetadata(session.session_id),
      api.sessionCurveCatalog(session.session_id),
      api.readPackageFiles(asset.package_path)
    ]);
    const curveNames = catalog.curves
      .filter((curve) => curve.name)
      .map((curve) => curve.name as string);
    const range = inferDepthRange(metadata, session.summary?.summary?.row_count);
    const window =
      curveNames.length >= 2
        ? range
          ? (await api.readDepthWindow(session.session_id, { curve_names: curveNames.slice(0, 2), depth_min: range.min, depth_max: range.max })).window
          : (await api.readCurveWindow(session.session_id, { curve_names: curveNames.slice(0, 2), start_row: 0, row_count: 25 })).window
        : null;
    return { kind: "log", asset, session, metadata, catalog, window, files };
  }
  if (asset.asset_kind === "Trajectory") {
    return { kind: "trajectory", asset, rows: await api.readProjectTrajectoryRows(workspace.project.root, asset.id, null, null) };
  }
  if (asset.asset_kind === "TopSet") {
    return { kind: "tops", asset, rows: await api.readProjectTops(workspace.project.root, asset.id) };
  }
  if (asset.asset_kind === "PressureObservation") {
    return { kind: "pressure", asset, rows: await api.readProjectPressureObservations(workspace.project.root, asset.id, null, null) };
  }
  return { kind: "drilling", asset, rows: await api.readProjectDrillingObservations(workspace.project.root, asset.id, null, null) };
}

function AppShell() {
  const [recents, setRecents] = useState<string[]>(() => loadRecents());
  const [workspace, setWorkspace] = useState<ProjectWorkspace | null>(null);
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
      <Route path="/" element={<HomePage recents={recents} notice={notice} setWorkspace={setWorkspace} setNotice={setNotice} pushRecent={pushRecent} menuAction={menuAction} setMenuAction={setMenuAction} />} />
      <Route path="/project" element={workspace ? <ProjectPage workspace={workspace} setWorkspace={setWorkspace} notice={notice} setNotice={setNotice} pushRecent={pushRecent} menuAction={menuAction} setMenuAction={setMenuAction} /> : <Navigate to="/" replace />} />
    </Routes>
  );
}

export default function App() {
  return <AppShell />;
}

function HomePage({
  recents,
  notice,
  setWorkspace,
  setNotice,
  pushRecent,
  menuAction,
  setMenuAction
}: {
  recents: string[];
  notice: Notice;
  setWorkspace: Dispatch<SetStateAction<ProjectWorkspace | null>>;
  setNotice: (notice: Notice) => void;
  pushRecent: (path: string) => void;
  menuAction: string | null;
  setMenuAction: Dispatch<SetStateAction<string | null>>;
}) {
  const navigate = useNavigate();
  const [projectRoot, setProjectRoot] = useState("");

  async function openProject(path: string, create: boolean) {
    if (!path.trim()) return;
    const project = create ? await api.createProject(path) : await api.openProject(path);
    const next = await loadProjectWorkspace(project);
    setWorkspace(next);
    pushRecent(project.root);
    setNotice(create ? { tone: "info", title: "Project created", detail: `Created LithosProject at ${project.root}` } : null);
    navigate("/project");
  }

  useEffect(() => {
    if (menuAction === "file.new-project") {
      void chooseFolder("Choose a folder for the new Lithos project")
        .then((folder) => {
          if (folder) return openProject(folder, true);
        })
        .finally(() => setMenuAction(null));
    } else if (menuAction === "file.open-project") {
      void chooseFolder("Open an existing Lithos project")
        .then((folder) => {
          if (folder) return openProject(folder, false);
        })
        .finally(() => setMenuAction(null));
    }
  }, [menuAction]);

  return (
    <main className="min-h-screen bg-[radial-gradient(circle_at_top_left,rgba(209,165,88,0.18),transparent_28%),linear-gradient(180deg,#fbf7ef_0%,#efe4d0_100%)] px-6 py-8 text-stone-900 md:px-8">
      <div className="mx-auto flex max-w-7xl flex-col gap-6">
        <header className="grid gap-6 md:grid-cols-[minmax(0,1fr)_320px]">
          <div className="space-y-4">
            <Badge tone="accent">Project-first desktop shell</Badge>
            <h1 className="max-w-4xl text-4xl font-semibold tracking-tight md:text-6xl">
              Open a Lithos project and inspect multiple wellbore asset families in one workspace.
            </h1>
            <p className="max-w-3xl text-base text-stone-600 md:text-lg">
              The app now starts from `LithosProject`, not a single package. Logs, tops,
              trajectory, pressure, and drilling assets all live under one project browser.
            </p>
          </div>
          <Card>
            <CardHeader>
              <CardTitle>Terms</CardTitle>
              <CardDescription>Project, asset package, session, and workspace are different layers.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-3 text-sm text-stone-600">
              <p><strong className="text-stone-900">Project:</strong> `catalog.sqlite` plus `assets/`.</p>
              <p><strong className="text-stone-900">Asset package:</strong> one authoritative storage unit for one asset.</p>
              <p><strong className="text-stone-900">Session:</strong> live editable state for a selected log package.</p>
              <p><strong className="text-stone-900">Workspace:</strong> the app shell around one open project.</p>
            </CardContent>
          </Card>
        </header>
        {notice ? <NoticeCard notice={notice} /> : null}
        <section className="grid gap-6 lg:grid-cols-[1fr_1fr]">
          <Card>
            <CardHeader>
              <CardTitle>Create a project</CardTitle>
              <CardDescription>Create a `LithosProject` root and then import assets into it.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <Label>
                Project root
                <Input aria-label="Project root" value={projectRoot} onChange={(event) => setProjectRoot(event.target.value)} />
              </Label>
              <div className="flex gap-3">
                <Button data-testid="create-project-button" onClick={() => void chooseFolder("Choose a folder for the new Lithos project").then((folder) => { if (folder) return openProject(folder, true); })}>Choose Folder</Button>
                <Button variant="outline" onClick={() => void openProject(projectRoot, true)}>Use Typed Path</Button>
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>Open an existing project</CardTitle>
              <CardDescription>Resume browsing wells, wellbores, collections, and assets.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <Label>
                Existing project root
                <Input aria-label="Existing project root" value={projectRoot} onChange={(event) => setProjectRoot(event.target.value)} />
              </Label>
              <div className="flex gap-3">
                <Button data-testid="open-project-button" onClick={() => void chooseFolder("Open an existing Lithos project").then((folder) => { if (folder) return openProject(folder, false); })}>Choose Folder</Button>
                <Button variant="outline" onClick={() => void openProject(projectRoot, false)}>Use Typed Path</Button>
              </div>
            </CardContent>
          </Card>
        </section>
        <Card>
          <CardHeader>
            <CardTitle>Recent projects</CardTitle>
            <CardDescription>Stored locally for quick reopening.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {recents.length === 0 ? (
              <p className="text-sm text-stone-600">No recent projects yet.</p>
            ) : (
              recents.map((path) => (
                <div key={path} className="flex items-center justify-between gap-3 rounded-2xl border border-stone-300 bg-white/70 p-3">
                  <div className="min-w-0">
                    <p className="truncate font-medium">{path}</p>
                    <p className="text-sm text-stone-600">Open directly into the project browser.</p>
                  </div>
                  <Button variant="secondary" onClick={() => void openProject(path, false)}>Open</Button>
                </div>
              ))
            )}
          </CardContent>
        </Card>
      </div>
    </main>
  );
}

function NoticeCard({ notice }: { notice: Exclude<Notice, null> }) {
  return (
    <Card className={notice.tone === "error" ? "border-amber-300" : "border-sky-300"}>
      <CardContent className="flex items-start gap-4 px-6 py-4">
        <Badge tone={notice.tone === "error" ? "warn" : "accent"}>
          {notice.tone === "error" ? "Problem" : "Status"}
        </Badge>
        <div className="space-y-1">
          <div className="font-medium">{notice.title}</div>
          <p className="text-sm text-stone-600">{notice.detail}</p>
        </div>
      </CardContent>
    </Card>
  );
}

function ProjectPage({
  workspace,
  setWorkspace,
  notice,
  setNotice,
  pushRecent,
  menuAction,
  setMenuAction
}: {
  workspace: ProjectWorkspace;
  setWorkspace: Dispatch<SetStateAction<ProjectWorkspace | null>>;
  notice: Notice;
  setNotice: (notice: Notice) => void;
  pushRecent: (path: string) => void;
  menuAction: string | null;
  setMenuAction: Dispatch<SetStateAction<string | null>>;
}) {
  const navigate = useNavigate();

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

  async function reload(next?: Partial<ProjectWorkspace>) {
    const project = await api.openProject(workspace.project.root);
    const refreshed = await loadProjectWorkspace(project, { ...workspace, ...next });
    setWorkspace(refreshed);
    pushRecent(project.root);
  }

  async function selectWell(wellId: string) {
    const refreshed = await loadProjectWorkspace(workspace.project, { ...workspace, selectedWellId: wellId, selectedWellboreId: null, selectedAssetId: null });
    setWorkspace(refreshed);
  }

  async function selectWellbore(wellboreId: string) {
    const refreshed = await loadProjectWorkspace(workspace.project, { ...workspace, selectedWellboreId: wellboreId, selectedAssetId: null });
    setWorkspace(refreshed);
  }

  async function selectAsset(assetId: string) {
    const next = { ...workspace, selectedAssetId: assetId, view: "asset" as ViewId };
    setWorkspace(next);
    const detail = await loadAssetDetail(next);
    setWorkspace((current) => current ? { ...current, selectedAssetId: assetId, view: "asset", detail } : current);
  }

  async function importAsset() {
    const path =
      workspace.importPath.trim() ||
      (await chooseFile(workspace.importKind === "log" ? "Choose a LAS file" : "Choose a CSV file", workspace.importKind === "log" ? ["las"] : ["csv"])) ||
      "";
    if (!path) return;
    let result: ProjectAssetImportResult;
    if (workspace.importKind === "log") {
      result = await api.importProjectLas(workspace.project.root, path, workspace.importCollectionName || null);
    } else if (workspace.importKind === "trajectory") {
      result = await api.importProjectTrajectoryCsv(workspace.project.root, path, workspace.importBinding, workspace.importCollectionName || null);
    } else if (workspace.importKind === "tops") {
      result = await api.importProjectTopsCsv(workspace.project.root, path, workspace.importBinding, workspace.importCollectionName || null);
    } else if (workspace.importKind === "pressure") {
      result = await api.importProjectPressureCsv(workspace.project.root, path, workspace.importBinding, workspace.importCollectionName || null);
    } else {
      result = await api.importProjectDrillingCsv(workspace.project.root, path, workspace.importBinding, workspace.importCollectionName || null);
    }
    const refreshed = await loadProjectWorkspace(await api.openProject(workspace.project.root), {
      ...workspace,
      selectedWellId: result.resolution.well_id,
      selectedWellboreId: result.resolution.wellbore_id,
      selectedAssetId: result.asset.id,
      importPath: path
    });
    setWorkspace(refreshed);
    setNotice({ tone: "info", title: "Asset imported", detail: `Imported ${path} into ${assetKindLabel(result.asset.asset_kind)} collection ${result.collection.name}.` });
    await selectAsset(result.asset.id);
  }

  async function runCoverage() {
    if (!workspace.selectedWellboreId || !workspace.coverageMin || !workspace.coverageMax) return;
    const coverage = await api.projectAssetsCoveringDepthRange(workspace.project.root, workspace.selectedWellboreId, Number(workspace.coverageMin), Number(workspace.coverageMax));
    setWorkspace({ ...workspace, view: "coverage", coverage });
  }

  async function saveLog() {
    if (!workspace.detail || workspace.detail.kind !== "log") throw { kind: "NoSession", message: "Select a log asset first." };
    await api.saveSession(workspace.detail.session.session_id);
    await selectAsset(workspace.detail.asset.id);
  }

  async function saveLogAs() {
    if (!workspace.detail || workspace.detail.kind !== "log") throw { kind: "NoSession", message: "Select a log asset first." };
    const folder = await chooseFolder("Choose an output folder for Save As");
    if (!folder) return;
    await api.saveSessionAs(workspace.detail.session.session_id, folder);
    await selectAsset(workspace.detail.asset.id);
  }

  useEffect(() => {
    if (menuAction === "file.new-project") {
      void run(async () => {
        const folder = await chooseFolder("Choose a folder for the new Lithos project");
        if (!folder) return;
        const project = await api.createProject(folder);
        setWorkspace(await loadProjectWorkspace(project));
        pushRecent(project.root);
      }).finally(() => setMenuAction(null));
    } else if (menuAction === "file.open-project") {
      void run(async () => {
        const folder = await chooseFolder("Open an existing Lithos project");
        if (!folder) return;
        const project = await api.openProject(folder);
        setWorkspace(await loadProjectWorkspace(project));
        pushRecent(project.root);
      }).finally(() => setMenuAction(null));
    } else if (menuAction === "file.import-asset") {
      setWorkspace({ ...workspace, view: "imports" });
      setMenuAction(null);
    } else if (menuAction === "file.save") {
      void run(saveLog).finally(() => setMenuAction(null));
    } else if (menuAction === "file.save-as") {
      void run(saveLogAs).finally(() => setMenuAction(null));
    } else if (menuAction === "file.close-workspace") {
      setWorkspace(null);
      navigate("/");
      setMenuAction(null);
    }
  }, [menuAction]);

  return (
    <main className="min-h-screen bg-[radial-gradient(circle_at_top_left,rgba(209,165,88,0.18),transparent_28%),linear-gradient(180deg,#fbf7ef_0%,#efe4d0_100%)] px-4 py-4 text-stone-900 md:px-6">
      <div className="mx-auto flex max-w-[1700px] flex-col gap-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="space-y-1">
            <Badge tone="accent">LithosProject workspace</Badge>
            <h1 className="text-3xl font-semibold tracking-tight">{workspace.project.root}</h1>
            <p className="text-sm text-stone-600">Browse wells, wellbores, collections, and assets. Logs can still be opened into a live editable session.</p>
          </div>
          <div className="rounded-2xl border border-stone-300 bg-white/80 px-4 py-3 text-sm shadow-sm">
            <div className="font-medium">{workspace.project.well_count} wells</div>
            <div className="text-stone-600">{workspace.project.asset_count} assets</div>
          </div>
        </div>
        <Menubar aria-label="Project menubar">
          <Button variant="ghost" onClick={() => navigate("/")}>Projects</Button>
          <Button variant="ghost" onClick={() => setWorkspace({ ...workspace, view: "overview" })}>Overview</Button>
          <Button variant="ghost" onClick={() => setWorkspace({ ...workspace, view: "imports" })}>Import Asset</Button>
          <Button variant="ghost" onClick={() => setWorkspace({ ...workspace, view: "coverage" })}>Depth Coverage</Button>
          <Button variant="ghost" onClick={() => void run(saveLog)}>Save Log</Button>
          <Button variant="ghost" onClick={() => void run(saveLogAs)}>Save Log As</Button>
        </Menubar>
        {notice ? <NoticeCard notice={notice} /> : null}
        <div className="grid gap-4 xl:grid-cols-[300px_300px_minmax(0,1fr)_320px]">
          <SidebarCard title="Wells" description="Project discovery">
            {workspace.wells.map((well) => (
              <TreeButton key={well.id} selected={workspace.selectedWellId === well.id} onClick={() => void run(() => selectWell(well.id))} title={well.name} subtitle={well.identifiers.uwi || well.identifiers.api || well.id} />
            ))}
          </SidebarCard>
          <SidebarCard title="Wellbores & Assets" description="Select a wellbore, then an asset">
            {workspace.wellbores.map((wellbore) => (
              <TreeButton key={wellbore.id} selected={workspace.selectedWellboreId === wellbore.id} onClick={() => void run(() => selectWellbore(wellbore.id))} title={wellbore.name} subtitle={wellbore.id} />
            ))}
            <div className="rounded-2xl border border-stone-200 bg-stone-50/70 p-3">
              <div className="mb-2 text-sm font-medium">Collections</div>
              <div className="grid gap-2 text-sm text-stone-600">
                {workspace.collections.map((collection) => (
                  <div key={collection.id} className="rounded-xl border border-stone-200 bg-white/80 px-3 py-2">
                    <div className="font-medium text-stone-900">{collection.name}</div>
                    <div>{assetKindLabel(collection.asset_kind)} | {collection.status}</div>
                  </div>
                ))}
              </div>
            </div>
            {workspace.assets.map((asset) => (
              <TreeButton key={asset.id} selected={workspace.selectedAssetId === asset.id} onClick={() => void run(() => selectAsset(asset.id))} title={assetKindLabel(asset.asset_kind)} subtitle={`${asset.id} | ${asset.manifest.extents.start ?? "?"} - ${asset.manifest.extents.stop ?? "?"}`} />
            ))}
          </SidebarCard>
          <MainPanel workspace={workspace} setWorkspace={setWorkspace} onImport={() => void run(importAsset)} onRunCoverage={() => void run(runCoverage)} onSelectAsset={(assetId) => void run(() => selectAsset(assetId))} />
          <InspectorPanel workspace={workspace} onRefresh={() => void run(() => reload())} />
        </div>
      </div>
    </main>
  );
}

function SidebarCard({ title, description, children }: { title: string; description: string; children: ReactNode }) {
  return (
    <Card>
      <CardHeader className="pb-3">
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent className="grid gap-2">{children}</CardContent>
    </Card>
  );
}

function TreeButton({ selected, onClick, title, subtitle }: { selected: boolean; onClick: () => void; title: string; subtitle: string }) {
  return (
    <button className={`rounded-2xl border px-4 py-3 text-left transition ${selected ? "border-amber-700 bg-amber-50" : "border-transparent bg-white/60 hover:border-stone-300 hover:bg-white"}`} onClick={onClick}>
      <div className="font-medium">{title}</div>
      <div className="text-xs text-stone-600">{subtitle}</div>
    </button>
  );
}

function MainPanel({
  workspace,
  setWorkspace,
  onImport,
  onRunCoverage,
  onSelectAsset
}: {
  workspace: ProjectWorkspace;
  setWorkspace: Dispatch<SetStateAction<ProjectWorkspace | null>>;
  onImport: () => void;
  onRunCoverage: () => void;
  onSelectAsset: (assetId: string) => void;
}) {
  return (
    <Card className="overflow-hidden">
      <CardHeader className="border-b border-stone-200 bg-white/70">
        <CardTitle>{workspace.view === "overview" ? "Overview" : workspace.view === "imports" ? "Imports" : workspace.view === "coverage" ? "Depth Coverage" : "Asset Viewer"}</CardTitle>
        <CardDescription>
          {workspace.view === "overview" && "Project, wellbore, and selected asset context."}
          {workspace.view === "imports" && "Import LAS or structured CSV assets into the selected project."}
          {workspace.view === "coverage" && "Find all assets covering a requested depth interval."}
          {workspace.view === "asset" && "Inspect the selected asset; log assets open a package session."}
        </CardDescription>
      </CardHeader>
      <CardContent className="p-0">
        {workspace.view === "overview" ? <OverviewPanel workspace={workspace} /> : null}
        {workspace.view === "imports" ? <ImportsPanel workspace={workspace} setWorkspace={setWorkspace} onImport={onImport} /> : null}
        {workspace.view === "coverage" ? <CoveragePanel workspace={workspace} setWorkspace={setWorkspace} onRun={onRunCoverage} onSelectAsset={onSelectAsset} /> : null}
        {workspace.view === "asset" ? <AssetPanel detail={workspace.detail} /> : null}
      </CardContent>
    </Card>
  );
}

function OverviewPanel({ workspace }: { workspace: ProjectWorkspace }) {
  const rows = [
    ["Project root", workspace.project.root],
    ["Catalog", workspace.project.catalog_path],
    ["Selected well", workspace.wells.find((item) => item.id === workspace.selectedWellId)?.name ?? "None"],
    ["Selected wellbore", workspace.wellbores.find((item) => item.id === workspace.selectedWellboreId)?.name ?? "None"],
    ["Selected asset", workspace.assets.find((item) => item.id === workspace.selectedAssetId)?.id ?? "None"]
  ];
  return (
    <TableShell>
      <table className="w-full border-collapse text-sm">
        <tbody>
          {rows.map(([label, value]) => (
            <tr key={label} className="border-b border-stone-200 last:border-b-0">
              <th className="w-56 bg-stone-100/70 px-4 py-3 text-left font-medium">{label}</th>
              <td className="px-4 py-3 text-stone-600">{value}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </TableShell>
  );
}

function ImportsPanel({
  workspace,
  setWorkspace,
  onImport
}: {
  workspace: ProjectWorkspace;
  setWorkspace: Dispatch<SetStateAction<ProjectWorkspace | null>>;
  onImport: () => void;
}) {
  const binding = workspace.importBinding;
  return (
    <div className="grid gap-4 p-4 lg:grid-cols-[360px_minmax(0,1fr)]">
      <Card className="shadow-none">
        <CardHeader>
          <CardTitle>Import Asset</CardTitle>
          <CardDescription>LAS imports infer identity; structured CSV imports use explicit well/wellbore binding.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid grid-cols-2 gap-2">
            {(["log", "trajectory", "tops", "pressure", "drilling"] as const).map((kind) => (
              <Button key={kind} variant={workspace.importKind === kind ? "default" : "outline"} onClick={() => setWorkspace({ ...workspace, importKind: kind, importPath: DEFAULT_PATHS[kind] })}>
                {kind === "log" ? "LAS Log" : assetKindLabel(kind === "tops" ? "TopSet" : kind === "pressure" ? "PressureObservation" : kind === "drilling" ? "DrillingObservation" : "Trajectory")}
              </Button>
            ))}
          </div>
          <Label>
            Source path
            <Input aria-label="Import source path" value={workspace.importPath} onChange={(event) => setWorkspace({ ...workspace, importPath: event.target.value })} />
          </Label>
          <Label>
            Collection name
            <Input aria-label="Collection name" value={workspace.importCollectionName} onChange={(event) => setWorkspace({ ...workspace, importCollectionName: event.target.value })} />
          </Label>
          {workspace.importKind !== "log" ? (
            <div className="grid gap-3 rounded-2xl border border-stone-300 bg-stone-50/70 p-4">
              <Label>
                Well name
                <Input aria-label="Well name" value={binding.well_name} onChange={(event) => setWorkspace({ ...workspace, importBinding: { ...binding, well_name: event.target.value } })} />
              </Label>
              <Label>
                Wellbore name
                <Input aria-label="Wellbore name" value={binding.wellbore_name} onChange={(event) => setWorkspace({ ...workspace, importBinding: { ...binding, wellbore_name: event.target.value } })} />
              </Label>
              <Label>
                UWI
                <Input aria-label="UWI" value={binding.uwi ?? ""} onChange={(event) => setWorkspace({ ...workspace, importBinding: { ...binding, uwi: event.target.value } })} />
              </Label>
              <Label>
                API
                <Input aria-label="API" value={binding.api ?? ""} onChange={(event) => setWorkspace({ ...workspace, importBinding: { ...binding, api: event.target.value } })} />
              </Label>
              <Label>
                Operator aliases
                <Textarea aria-label="Operator aliases" value={binding.operator_aliases.join(", ")} onChange={(event) => setWorkspace({ ...workspace, importBinding: { ...binding, operator_aliases: operatorAliases(event.target.value) } })} />
              </Label>
            </div>
          ) : null}
          <div className="flex gap-3">
            <Button data-testid="choose-import-button" onClick={() => void chooseFile(workspace.importKind === "log" ? "Choose a LAS file" : "Choose a CSV file", workspace.importKind === "log" ? ["las"] : ["csv"]).then((path) => path && setWorkspace({ ...workspace, importPath: path }))}>Choose Source</Button>
            <Button data-testid="import-asset-button" variant="outline" onClick={onImport}>Import Into Project</Button>
          </div>
        </CardContent>
      </Card>
      <InfoBlock title="Import Preview" value={{ importKind: workspace.importKind, importPath: workspace.importPath, binding: workspace.importBinding, collectionName: workspace.importCollectionName || null }} />
    </div>
  );
}

function CoveragePanel({
  workspace,
  setWorkspace,
  onRun,
  onSelectAsset
}: {
  workspace: ProjectWorkspace;
  setWorkspace: Dispatch<SetStateAction<ProjectWorkspace | null>>;
  onRun: () => void;
  onSelectAsset: (assetId: string) => void;
}) {
  return (
    <div className="grid gap-4 p-4 lg:grid-cols-[340px_minmax(0,1fr)]">
      <Card className="shadow-none">
        <CardHeader>
          <CardTitle>Depth Coverage</CardTitle>
          <CardDescription>Find assets covering an interval in the selected wellbore.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <Label>
            Depth min
            <Input aria-label="Coverage depth min" value={workspace.coverageMin} onChange={(event) => setWorkspace({ ...workspace, coverageMin: event.target.value })} />
          </Label>
          <Label>
            Depth max
            <Input aria-label="Coverage depth max" value={workspace.coverageMax} onChange={(event) => setWorkspace({ ...workspace, coverageMax: event.target.value })} />
          </Label>
          <Button data-testid="run-coverage-button" onClick={onRun}>Run Coverage Query</Button>
        </CardContent>
      </Card>
      <TableShell>
        <table className="w-full border-collapse text-sm">
          <thead className="bg-stone-100/80">
            <tr>
              <th className="px-4 py-3 text-left font-medium">Kind</th>
              <th className="px-4 py-3 text-left font-medium">Asset</th>
              <th className="px-4 py-3 text-left font-medium">Extent</th>
              <th className="px-4 py-3 text-left font-medium">Action</th>
            </tr>
          </thead>
          <tbody>
            {workspace.coverage.length === 0 ? (
              <tr><td className="px-4 py-4 text-stone-600" colSpan={4}>No coverage results yet.</td></tr>
            ) : (
              workspace.coverage.map((asset) => (
                <tr key={asset.id} className="border-t border-stone-200">
                  <td className="px-4 py-3">{assetKindLabel(asset.asset_kind)}</td>
                  <td className="px-4 py-3 text-stone-600">{asset.id}</td>
                  <td className="px-4 py-3 text-stone-600">{asset.manifest.extents.start ?? "?"} - {asset.manifest.extents.stop ?? "?"}</td>
                  <td className="px-4 py-3"><Button variant="secondary" onClick={() => onSelectAsset(asset.id)}>Open</Button></td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </TableShell>
    </div>
  );
}

function AssetPanel({ detail }: { detail: AssetDetail }) {
  if (!detail) return <div className="p-6 text-sm text-stone-600">Select an asset to inspect it.</div>;
  if (detail.kind === "log") {
    const rows = [
      ["Package path", detail.asset.package_path],
      ["Session id", detail.session.session_id],
      ["Revision", detail.session.revision],
      ["Dirty", detail.session.dirty.has_unsaved_changes ? "true" : "false"],
      ["Curve count", String(detail.session.summary?.summary?.curve_count ?? "")],
      ["Row count", String(detail.session.summary?.summary?.row_count ?? "")]
    ];
    return (
      <div className="grid gap-4 p-4">
        <TableShell>
          <table className="w-full border-collapse text-sm">
            <tbody>
              {rows.map(([label, value]) => (
                <tr key={label} className="border-b border-stone-200 last:border-b-0">
                  <th className="w-48 bg-stone-100/70 px-4 py-3 text-left font-medium">{label}</th>
                  <td className="px-4 py-3 text-stone-600">{value}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </TableShell>
        <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_320px]">
          <TableShell>
            <div className="overflow-auto">
              <table className="min-w-full border-collapse text-sm">
                <thead className="bg-stone-100/80">
                  <tr>
                    {(detail.window?.columns ?? []).map((column) => (
                      <th key={column.name} className="border-l border-stone-200 px-4 py-3 text-left font-medium first:border-l-0">{column.name}</th>
                    ))}
                  </tr>
                </thead>
                <tbody>
                  {detail.window && detail.window.columns.length > 0 ? (
                    Array.from({ length: detail.window.row_count }).map((_, rowIndex) => (
                      <tr key={rowIndex} className="border-t border-stone-200">
                        {detail.window?.columns.map((column) => (
                          <td key={`${column.name}-${rowIndex}`} className="border-l border-stone-200 px-4 py-3 text-stone-600 first:border-l-0">{asText(column.values[rowIndex])}</td>
                        ))}
                      </tr>
                    ))
                  ) : (
                    <tr><td className="px-4 py-4 text-stone-600">No log window loaded.</td></tr>
                  )}
                </tbody>
              </table>
            </div>
          </TableShell>
          <div className="grid gap-4">
            <InfoBlock title="Session metadata" value={detail.metadata} />
            <InfoBlock title="Curve catalog" value={detail.catalog} />
            <InfoBlock title="Package files" value={detail.files} />
          </div>
        </div>
      </div>
    );
  }

  const columns = detail.rows.length > 0 ? Object.keys(detail.rows[0] as Record<string, unknown>) : [];
  return (
    <div className="grid gap-4 p-4">
      <InfoBlock title={`${assetKindLabel(detail.asset.asset_kind)} asset`} value={detail.asset} />
      <TableShell>
        <div className="overflow-auto">
          <table className="min-w-full border-collapse text-sm">
            <thead className="bg-stone-100/80">
              <tr>
                {columns.map((column) => (
                  <th key={column} className="border-l border-stone-200 px-4 py-3 text-left font-medium first:border-l-0">{column}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {detail.rows.length === 0 ? (
                <tr><td className="px-4 py-4 text-stone-600">No rows returned for this asset.</td></tr>
              ) : (
                detail.rows.map((row, index) => (
                  <tr key={index} className="border-t border-stone-200">
                    {columns.map((column) => (
                      <td key={`${index}-${column}`} className="border-l border-stone-200 px-4 py-3 text-stone-600 first:border-l-0">{asText((row as Record<string, unknown>)[column])}</td>
                    ))}
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </TableShell>
    </div>
  );
}

function InspectorPanel({ workspace, onRefresh }: { workspace: ProjectWorkspace; onRefresh: () => void }) {
  const selectedWell = workspace.wells.find((item) => item.id === workspace.selectedWellId);
  const selectedWellbore = workspace.wellbores.find((item) => item.id === workspace.selectedWellboreId);
  const selectedAsset = workspace.assets.find((item) => item.id === workspace.selectedAssetId);
  return (
    <Card>
      <CardHeader>
        <CardTitle>Inspector</CardTitle>
        <CardDescription>Always-visible project and selected entity state.</CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid gap-3 rounded-2xl border border-stone-300 bg-white/70 p-4 text-sm">
          <div><div className="font-medium">Project root</div><div className="break-all text-stone-600">{workspace.project.root}</div></div>
          <div><div className="font-medium">Catalog</div><div className="break-all text-stone-600">{workspace.project.catalog_path}</div></div>
          <div><div className="font-medium">Selected well</div><div className="text-stone-600">{selectedWell?.name ?? "None"}</div></div>
          <div><div className="font-medium">Selected wellbore</div><div className="text-stone-600">{selectedWellbore?.name ?? "None"}</div></div>
          <div><div className="font-medium">Selected asset</div><div className="text-stone-600">{selectedAsset ? `${assetKindLabel(selectedAsset.asset_kind)} · ${selectedAsset.id}` : "None"}</div></div>
          {workspace.detail?.kind === "log" ? (
            <div><div className="font-medium">Log session</div><div className="break-all text-stone-600">{workspace.detail.session.session_id}</div></div>
          ) : null}
        </div>
        <Button variant="outline" onClick={onRefresh}>Refresh Project</Button>
      </CardContent>
    </Card>
  );
}

function InfoBlock({ title, value }: { title: string; value: unknown }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base">{title}</CardTitle>
      </CardHeader>
      <CardContent>
        <pre className="max-h-[320px] overflow-auto rounded-2xl bg-stone-100/80 p-4 text-xs">{pretty(value)}</pre>
      </CardContent>
    </Card>
  );
}
