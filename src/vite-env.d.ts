/// <reference types="vite/client" />

interface ElectronAPI {
  getVersion: () => Promise<string>;
  openExternal: (url: string) => Promise<void>;
  saveFile: (opts: { defaultPath?: string; content: Uint8Array }) => Promise<{ success: boolean; path?: string }>;
  openFile: () => Promise<{ success: boolean; files: Array<{ path: string; name: string; content: string }> }>;
  onNewProject: (callback: () => void) => void;
}

declare global {
  interface Window {
    electronAPI?: ElectronAPI;
  }
}

export {};
