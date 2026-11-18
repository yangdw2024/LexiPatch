import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";
import type {
  DashboardStats,
  GlossaryEntry,
  ImportFileInput,
  LlmProfile,
  PatchBuild,
  Project,
  ResourceFile,
  ScannedFileInfo,
  TranslateProgressEvent,
  TranslationMemory,
  TranslationUnit,
} from "@/types";

export const api = {
  // Dialogs
  pickFolder: (title = "选择目标软件安装目录") =>
    open({ directory: true, multiple: false, title }) as Promise<string | null>,

  pickFiles: (title = "选择资源文件") =>
    open({
      directory: false,
      multiple: true,
      title,
      filters: [
        {
          name: "软件资源文件",
          extensions: ["json", "arb", "yaml", "yml", "toml", "properties", "ini", "cfg", "conf", "po", "pot", "xml", "resx", "ts"],
        },
        { name: "所有文件", extensions: ["*"] },
      ],
    }) as Promise<string[] | null>,

  savePatchExe: (defaultName: string) =>
    save({
      title: "导出独立 .exe 汉化补丁",
      defaultPath: defaultName,
      filters: [{ name: "Windows 可执行补丁", extensions: ["exe"] }],
    }),

  savePackageZip: (defaultName: string) =>
    save({
      title: "导出 ZIP 汉化包",
      defaultPath: defaultName,
      filters: [{ name: "ZIP 压缩包", extensions: ["zip"] }],
    }),

  // Dashboard
  getDashboardStats: () => invoke<DashboardStats>("get_dashboard_stats"),

  // Projects
  listProjects: () => invoke<Project[]>("list_projects"),
  getProject: (id: string) => invoke<Project | null>("get_project", { id }),
  createProject: (params: {
    name: string;
    software: string;
    version: string;
    sourceDir: string | null;
    exeName: string | null;
    defaultPaths: string[];
    description: string;
  }) => invoke<Project>("create_project", params),
  updateProjectMeta: (params: {
    id: string;
    name: string;
    software: string;
    version: string;
    exeName: string | null;
    defaultPaths: string[];
    description: string;
  }) => invoke<void>("update_project_meta", params),
  deleteProject: (id: string) => invoke<void>("delete_project", { id }),

  // Scanning & Files
  scanDirectory: (dirPath: string) => invoke<ScannedFileInfo[]>("scan_directory", { dirPath }),
  importProjectFiles: (projectId: string, files: ImportFileInput[]) =>
    invoke<ResourceFile[]>("import_project_files", { projectId, files }),
  listFiles: (projectId: string) => invoke<ResourceFile[]>("list_files", { projectId }),
  updateFileTargetPath: (fileId: string, targetRelPath: string) =>
    invoke<void>("update_file_target_path", { fileId, targetRelPath }),

  // Units & Preview
  listUnits: (projectId: string, fileId?: string | null) =>
    invoke<TranslationUnit[]>("list_units", { projectId, fileId: fileId ?? null }),
  updateUnit: (id: string, target: string, status: string) =>
    invoke<string | null>("update_unit", { id, target, status }),
  previewPatchedFile: (projectId: string, fileId: string) =>
    invoke<string>("preview_patched_file", { projectId, fileId }),

  // Glossary & Memory
  listGlossary: (projectId?: string | null) =>
    invoke<GlossaryEntry[]>("list_glossary", { projectId: projectId ?? null }),
  upsertGlossary: (entry: GlossaryEntry) => invoke<void>("upsert_glossary", { entry }),
  deleteGlossary: (id: string) => invoke<void>("delete_glossary", { id }),
  listMemory: () => invoke<TranslationMemory[]>("list_memory"),

  // LLM Profiles & Translation
  listLlmProfiles: () => invoke<LlmProfile[]>("list_llm_profiles"),
  upsertLlmProfile: (profile: LlmProfile) => invoke<void>("upsert_llm_profile", { profile }),
  deleteLlmProfile: (id: string) => invoke<void>("delete_llm_profile", { id }),
  testLlmConnection: (profile: LlmProfile) => invoke<string>("test_llm_connection", { profile }),
  runLlmTranslate: (params: {
    projectId: string;
    fileId: string | null;
    profileId: string;
    onlyUntranslated: boolean;
  }) => invoke<[number, number, number]>("run_llm_translate", params),

  onTranslateProgress: (cb: (ev: TranslateProgressEvent) => void): Promise<UnlistenFn> =>
    listen<TranslateProgressEvent>("translate:progress", (e) => cb(e.payload)),

  // Patcher & Packaging
  buildExePatcher: (params: {
    projectId: string;
    version: string;
    author: string;
    note: string;
    outputPath: string;
  }) => invoke<PatchBuild>("build_exe_patcher", params),
  buildZipPackage: (params: {
    projectId: string;
    version: string;
    note: string;
    outputPath: string;
  }) => invoke<PatchBuild>("build_zip_package", params),
  listPatchBuilds: (projectId: string) => invoke<PatchBuild[]>("list_patch_builds", { projectId }),
};
