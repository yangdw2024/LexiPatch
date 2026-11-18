export type UnitStatus = "untranslated" | "machine" | "translated" | "reviewed" | "flagged";

export interface Project {
  id: string;
  name: string;
  software: string;
  version: string;
  sourceDir?: string | null;
  exeName?: string | null;
  defaultPaths: string[];
  description: string;
  status: string;
  totalUnits: number;
  translatedUnits: number;
  reviewedUnits: number;
  createdAt: number;
  updatedAt: number;
}

export interface ResourceFile {
  id: string;
  projectId: string;
  relPath: string;
  targetRelPath: string;
  format: string;
  originalSha256: string;
  unitCount: number;
  translatedCount: number;
  rawContent: string;
}

export interface TranslationUnit {
  id: string;
  fileId: string;
  projectId: string;
  keyPath: string;
  source: string;
  target: string;
  status: UnitStatus;
  qaWarning?: string | null;
  termHits: string[];
  fuzzyScore: number;
  updatedAt: number;
}

export interface GlossaryEntry {
  id: string;
  projectId: string;
  source: string;
  target: string;
  domain: string;
  context: string;
  doNotTranslate: boolean;
}

export interface TranslationMemory {
  id: string;
  source: string;
  target: string;
  domain: string;
  updatedAt: number;
}

export interface LlmProfile {
  id: string;
  name: string;
  provider: string;
  baseUrl: string;
  apiKey: string;
  model: string;
  concurrency: number;
  batchSize: number;
  temperature: number;
  isDefault: boolean;
}

export interface PatchBuild {
  id: string;
  projectId: string;
  version: string;
  buildType: "exe" | "zip";
  outputPath: string;
  fileCount: number;
  unitCount: number;
  note: string;
  createdAt: number;
}

export interface ScannedFileInfo {
  absPath: string;
  relPath: string;
  format: string;
  sizeBytes: number;
  previewUnitCount: number;
}

export interface TranslateProgressEvent {
  projectId: string;
  total: number;
  completed: number;
  succeeded: number;
  failed: number;
  message: string;
  done: boolean;
}

export interface DashboardStats {
  projectCount: number;
  totalUnits: number;
  translatedUnits: number;
  reviewedUnits: number;
  qaWarningUnits: number;
  patchCount: number;
  glossaryCount: number;
  memoryCount: number;
}

export interface ImportFileInput {
  absPath?: string;
  relPath: string;
  targetRelPath?: string;
  format?: string;
  rawContent?: string;
}
