import { useEffect, useState } from "react";
import { useParams } from "react-router-dom";
import { api } from "@/lib/tauri";
import type { PatchBuild, Project } from "@/types";
import { PackageCheck, FileArchive, Cpu, CheckCircle2, Loader2 } from "lucide-react";

export default function PackagePage() {
  const { projectId } = useParams();
  const [projects, setProjects] = useState<Project[]>([]);
  const [activeId, setActiveId] = useState(projectId || "");
  const [builds, setBuilds] = useState<PatchBuild[]>([]);

  const [version, setVersion] = useState("v1.0.0-cn");
  const [author, setAuthor] = useState("译铸 LexiPatch");
  const [note, setNote] = useState("本补丁会自动备份原版文件至 .transcore_backup 目录，支持随时一键还原。");
  const [building, setBuilding] = useState(false);
  const [resultMsg, setResultMsg] = useState<{ ok: boolean; text: string } | null>(null);

  useEffect(() => {
    api.listProjects().then((list) => {
      setProjects(list);
      if (!activeId && list.length > 0) {
        setActiveId(list[0].id);
      }
    });
  }, []);

  useEffect(() => {
    if (projectId) setActiveId(projectId);
  }, [projectId]);

  const activeProject = projects.find((p) => p.id === activeId) || null;

  const loadBuilds = (pid: string) => {
    if (!pid) return;
    api.listPatchBuilds(pid).then(setBuilds).catch(console.error);
  };

  useEffect(() => {
    loadBuilds(activeId);
    if (activeProject) {
      setVersion(`${activeProject.version}-汉化补丁`);
    }
  }, [activeId, projects]);

  const handleBuildExe = async () => {
    if (!activeProject) return;
    setResultMsg(null);
    const defaultName = `${activeProject.software}_${version}_汉化补丁.exe`.replace(/[\\/:*?"<>|]/g, "_");
    const outPath = await api.savePatchExe(defaultName);
    if (!outPath) return;

    setBuilding(true);
    try {
      const b = await api.buildExePatcher({
        projectId: activeProject.id,
        version,
        author,
        note,
        outputPath: outPath,
      });
      setResultMsg({
        ok: true,
        text: `独立 .exe 汉化补丁生成成功！路径：${b.outputPath}`,
      });
      loadBuilds(activeProject.id);
    } catch (e) {
      setResultMsg({ ok: false, text: String(e) });
    } finally {
      setBuilding(false);
    }
  };

  const handleBuildZip = async () => {
    if (!activeProject) return;
    setResultMsg(null);
    const defaultName = `${activeProject.software}_${version}_汉化包.zip`.replace(/[\\/:*?"<>|]/g, "_");
    const outPath = await api.savePackageZip(defaultName);
    if (!outPath) return;

    setBuilding(true);
    try {
      const b = await api.buildZipPackage({
        projectId: activeProject.id,
        version,
        note,
        outputPath: outPath,
      });
      setResultMsg({
        ok: true,
        text: `ZIP 汉化包导出成功！路径：${b.outputPath}`,
      });
      loadBuilds(activeProject.id);
    } catch (e) {
      setResultMsg({ ok: false, text: String(e) });
    } finally {
      setBuilding(false);
    }
  };

  return (
    <div className="p-6 max-w-5xl mx-auto w-full space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold text-slate-100">生成独立 .exe 汉化补丁</h1>
          <p className="text-xs text-slate-400 mt-1">
            将所有格式保留的汉化文件打包进独立原生 Windows .exe 安装器，发给任何人双击即可自动定位软件目录打补丁与还原
          </p>
        </div>
        <select
          value={activeId}
          onChange={(e) => setActiveId(e.target.value)}
          className="px-3 py-2 rounded bg-[#121821] border border-slate-700 text-xs text-slate-100"
        >
          {projects.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name} ({p.translatedUnits}/{p.totalUnits})
            </option>
          ))}
        </select>
      </div>

      {activeProject && (
        <div className="rounded-lg border border-slate-800 bg-[#121821] p-5 space-y-4">
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label className="block text-xs text-slate-400 mb-1">补丁版本号</label>
              <input
                value={version}
                onChange={(e) => setVersion(e.target.value)}
                className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs font-mono text-slate-100"
              />
            </div>
            <div>
              <label className="block text-xs text-slate-400 mb-1">制作者 / 汉化组署名</label>
              <input
                value={author}
                onChange={(e) => setAuthor(e.target.value)}
                className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100"
              />
            </div>
            <div className="sm:col-span-2">
              <label className="block text-xs text-slate-400 mb-1">补丁说明（将显示在 .exe 安装器界面）</label>
              <textarea
                rows={2}
                value={note}
                onChange={(e) => setNote(e.target.value)}
                className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100"
              />
            </div>
          </div>

          <div className="p-3 rounded bg-[#0A0E14] border border-slate-800 text-xs text-slate-400 flex items-center gap-2">
            <Cpu className="w-4 h-4 text-cyan-400 shrink-0" />
            <span>
              目标软件：<strong className="text-slate-200">{activeProject.software}</strong> ·
              已完成 <strong className="text-cyan-300">{activeProject.translatedUnits}</strong> / {activeProject.totalUnits} 词条 ·
              自动定位线索：{activeProject.exeName || "注册表名称匹配 + 手动浏览"}
            </span>
          </div>

          {resultMsg && (
            <div
              className={`p-3 rounded text-xs flex items-center gap-2 ${
                resultMsg.ok
                  ? "bg-emerald-500/10 border border-emerald-500/30 text-emerald-300"
                  : "bg-rose-500/10 border border-rose-500/30 text-rose-300"
              }`}
            >
              <CheckCircle2 className="w-4 h-4 shrink-0" />
              <span className="break-all">{resultMsg.text}</span>
            </div>
          )}

          <div className="flex flex-wrap items-center gap-3 pt-2">
            <button
              onClick={handleBuildExe}
              disabled={building}
              className="flex items-center gap-2 px-5 py-2.5 rounded-md bg-cyan-500 hover:bg-cyan-400 disabled:opacity-50 text-slate-950 font-semibold text-xs"
            >
              {building ? <Loader2 className="w-4 h-4 animate-spin" /> : <PackageCheck className="w-4 h-4" />}
              一键导出独立 .exe 汉化安装器
            </button>
            <button
              onClick={handleBuildZip}
              disabled={building}
              className="flex items-center gap-2 px-4 py-2.5 rounded-md border border-slate-700 hover:border-slate-500 text-slate-200 text-xs"
            >
              <FileArchive className="w-4 h-4 text-amber-400" />
              导出标准 .zip 汉化包
            </button>
          </div>
        </div>
      )}

      {/* 历史构建记录 */}
      <div className="rounded-lg border border-slate-800 bg-[#121821] overflow-hidden">
        <div className="px-4 py-3 border-b border-slate-800 text-sm font-medium text-slate-200">
          构建历史记录
        </div>
        {builds.length === 0 ? (
          <div className="p-8 text-center text-xs text-slate-500">暂无构建记录</div>
        ) : (
          <div className="divide-y divide-slate-800/70">
            {builds.map((b) => (
              <div key={b.id} className="px-4 py-3 flex items-center justify-between text-xs">
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="px-1.5 py-0.5 rounded bg-cyan-500/15 text-cyan-300 font-mono uppercase text-[10px]">
                      {b.buildType}
                    </span>
                    <span className="font-mono text-slate-200">{b.version}</span>
                    <span className="text-slate-500">
                      {b.fileCount} 文件 · {b.unitCount} 词条
                    </span>
                  </div>
                  <div className="text-[11px] font-mono text-slate-500 mt-1 truncate">
                    {b.outputPath}
                  </div>
                </div>
                <span className="text-[11px] text-slate-500 shrink-0">
                  {new Date(b.createdAt).toLocaleString()}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
