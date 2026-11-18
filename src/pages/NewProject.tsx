import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { api } from "@/lib/tauri";
import type { ScannedFileInfo } from "@/types";
import { FolderSearch, FilePlus2, CheckSquare, Square, Loader2 } from "lucide-react";

export default function NewProject() {
  const navigate = useNavigate();
  const [name, setName] = useState("");
  const [software, setSoftware] = useState("");
  const [version, setVersion] = useState("v1.0.0");
  const [exeName, setExeName] = useState("");
  const [defaultPathsText, setDefaultPathsText] = useState("");
  const [description, setDescription] = useState("");

  const [sourceDir, setSourceDir] = useState<string | null>(null);
  const [scanned, setScanned] = useState<ScannedFileInfo[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [scanning, setScanning] = useState(false);
  const [creating, setCreating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleScanFolder = async () => {
    setError(null);
    const dir = await api.pickFolder("选择目标软件安装目录或语言包目录");
    if (!dir) return;
    setSourceDir(dir);
    if (!name) {
      const folderName = dir.split(/[/\\]/).filter(Boolean).pop() || "新建汉化项目";
      setName(`${folderName} 汉化`);
      if (!software) setSoftware(folderName);
    }
    setScanning(true);
    try {
      const files = await api.scanDirectory(dir);
      setScanned(files);
      setSelected(new Set(files.map((f) => f.absPath)));
    } catch (e) {
      setError(String(e));
    } finally {
      setScanning(false);
    }
  };

  const handlePickFiles = async () => {
    setError(null);
    const paths = await api.pickFiles("选择需要汉化的资源文件");
    if (!paths || paths.length === 0) return;
    const items: ScannedFileInfo[] = paths.map((p) => {
      const fileName = p.split(/[/\\]/).pop() || p;
      const ext = fileName.split(".").pop()?.toLowerCase() || "json";
      return {
        absPath: p,
        relPath: fileName,
        format: ext,
        sizeBytes: 0,
        previewUnitCount: 0,
      };
    });
    setScanned((prev) => {
      const map = new Map(prev.map((x) => [x.absPath, x]));
      for (const it of items) map.set(it.absPath, it);
      return Array.from(map.values());
    });
    setSelected((prev) => {
      const next = new Set(prev);
      for (const it of items) next.add(it.absPath);
      return next;
    });
  };

  const toggleSelect = (absPath: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(absPath)) next.delete(absPath);
      else next.add(absPath);
      return next;
    });
  };

  const handleCreate = async () => {
    if (!name.trim() || !software.trim()) {
      setError("请填写项目名称和目标软件名称");
      return;
    }
    const chosenFiles = scanned.filter((f) => selected.has(f.absPath));
    if (chosenFiles.length === 0) {
      setError("请至少扫描或选择一个可翻译的资源文件");
      return;
    }

    setCreating(true);
    setError(null);
    try {
      const defaultPaths = defaultPathsText
        .split(/\r?\n/)
        .map((s) => s.trim())
        .filter(Boolean);
      const proj = await api.createProject({
        name: name.trim(),
        software: software.trim(),
        version: version.trim() || "v1.0.0",
        sourceDir,
        exeName: exeName.trim() || null,
        defaultPaths,
        description: description.trim(),
      });

      await api.importProjectFiles(
        proj.id,
        chosenFiles.map((f) => ({
          absPath: f.absPath,
          relPath: f.relPath,
          targetRelPath: f.relPath,
        }))
      );

      navigate(`/translate/${proj.id}`);
    } catch (e) {
      setError(String(e));
    } finally {
      setCreating(false);
    }
  };

  return (
    <div className="p-6 max-w-5xl mx-auto w-full space-y-6">
      <div>
        <h1 className="text-xl font-semibold text-slate-100">新建汉化项目</h1>
        <p className="text-xs text-slate-400 mt-1">
          直接扫描真实软件目录，保留完整相对路径结构，以便后续一键打包为 .exe 自动定位替换
        </p>
      </div>

      {error && (
        <div className="p-3 rounded-md bg-rose-500/10 border border-rose-500/30 text-xs text-rose-300">
          {error}
        </div>
      )}

      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 rounded-lg border border-slate-800 bg-[#121821] p-5">
        <div>
          <label className="block text-xs text-slate-400 mb-1.5">项目名称 *</label>
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="例如：XX 编辑器简体中文汉化"
            className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100 focus:outline-none focus:border-cyan-400"
          />
        </div>
        <div>
          <label className="block text-xs text-slate-400 mb-1.5">目标软件名称 *（用于注册表自动匹配）</label>
          <input
            value={software}
            onChange={(e) => setSoftware(e.target.value)}
            placeholder="例如：Trae / VSCode / GROMACS"
            className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100 focus:outline-none focus:border-cyan-400"
          />
        </div>
        <div>
          <label className="block text-xs text-slate-400 mb-1.5">目标软件版本号</label>
          <input
            value={version}
            onChange={(e) => setVersion(e.target.value)}
            placeholder="v1.0.0"
            className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100 font-mono focus:outline-none focus:border-cyan-400"
          />
        </div>
        <div>
          <label className="block text-xs text-slate-400 mb-1.5">主程序文件名（可选，供 .exe 补丁注册表定位）</label>
          <input
            value={exeName}
            onChange={(e) => setExeName(e.target.value)}
            placeholder="例如：AppName.exe"
            className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100 font-mono focus:outline-none focus:border-cyan-400"
          />
        </div>
        <div className="md:col-span-2">
          <label className="block text-xs text-slate-400 mb-1.5">
            默认安装候选路径（每行一条，支持 %LOCALAPPDATA%、%PROGRAMFILES% 等环境变量）
          </label>
          <textarea
            rows={2}
            value={defaultPathsText}
            onChange={(e) => setDefaultPathsText(e.target.value)}
            placeholder={"%LOCALAPPDATA%\\Programs\\MyApp\n%PROGRAMFILES%\\MyApp"}
            className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100 font-mono focus:outline-none focus:border-cyan-400"
          />
        </div>
        <div className="md:col-span-2">
          <label className="block text-xs text-slate-400 mb-1.5">软件背景说明（将注入 AI 大模型提示词以提升翻译准确度）</label>
          <input
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="例如：一款面向科研人员的分子动力学模拟与可视化桌面软件"
            className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100 focus:outline-none focus:border-cyan-400"
          />
        </div>
      </div>

      {/* 扫描资源文件 */}
      <div className="rounded-lg border border-slate-800 bg-[#121821] p-5 space-y-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <h2 className="text-sm font-medium text-slate-200">导入待汉化资源文件</h2>
            <p className="text-[11px] text-slate-500 mt-0.5">
              支持 JSON / ARB / YAML / TOML / Properties / INI / PO / XML / RESX / Qt .ts
            </p>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={handleScanFolder}
              disabled={scanning}
              className="flex items-center gap-1.5 px-3.5 py-2 rounded-md bg-cyan-500/15 border border-cyan-500/40 hover:bg-cyan-500/25 text-xs text-cyan-300 font-medium"
            >
              {scanning ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <FolderSearch className="w-3.5 h-3.5" />}
              扫描软件安装目录
            </button>
            <button
              onClick={handlePickFiles}
              className="flex items-center gap-1.5 px-3 py-2 rounded-md border border-slate-700 hover:border-slate-500 text-xs text-slate-200"
            >
              <FilePlus2 className="w-3.5 h-3.5" />
              单独选择文件
            </button>
          </div>
        </div>

        {sourceDir && (
          <div className="text-xs font-mono text-slate-400 bg-[#0A0E14] px-3 py-2 rounded border border-slate-800">
            扫描目录：{sourceDir}
          </div>
        )}

        {scanned.length > 0 && (
          <div className="max-h-64 overflow-auto border border-slate-800 rounded-md divide-y divide-slate-800/70">
            {scanned.map((f) => {
              const checked = selected.has(f.absPath);
              return (
                <div
                  key={f.absPath}
                  onClick={() => toggleSelect(f.absPath)}
                  className="px-3 py-2 flex items-center justify-between text-xs hover:bg-slate-800/40 cursor-pointer"
                >
                  <div className="flex items-center gap-2.5 min-w-0">
                    {checked ? (
                      <CheckSquare className="w-4 h-4 text-cyan-400 shrink-0" />
                    ) : (
                      <Square className="w-4 h-4 text-slate-600 shrink-0" />
                    )}
                    <span className="font-mono text-slate-200 truncate">{f.relPath}</span>
                  </div>
                  <div className="flex items-center gap-3 shrink-0 text-[11px] text-slate-400 font-mono">
                    <span className="px-1.5 py-0.5 rounded bg-slate-800 text-cyan-300 uppercase">
                      {f.format}
                    </span>
                    {f.previewUnitCount > 0 && <span>{f.previewUnitCount} 词条</span>}
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      <div className="flex justify-end gap-3">
        <button
          onClick={() => navigate("/projects")}
          className="px-4 py-2 rounded-md border border-slate-700 text-xs text-slate-300 hover:bg-slate-800"
        >
          取消
        </button>
        <button
          onClick={handleCreate}
          disabled={creating}
          className="flex items-center gap-1.5 px-5 py-2 rounded-md bg-cyan-500 hover:bg-cyan-400 disabled:opacity-50 text-slate-950 font-semibold text-xs"
        >
          {creating && <Loader2 className="w-3.5 h-3.5 animate-spin" />}
          创建项目并解析词条 ({selected.size} 个文件)
        </button>
      </div>
    </div>
  );
}
