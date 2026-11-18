import { useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { api } from "@/lib/tauri";
import type {
  LlmProfile,
  Project,
  ResourceFile,
  TranslateProgressEvent,
  TranslationUnit,
} from "@/types";
import {
  Sparkles,
  Eye,
  AlertTriangle,
  Check,
  Save,
  PackageCheck,
  Search,
  X,
  Loader2,
} from "lucide-react";

const STATUS_LABEL: Record<string, { text: string; cls: string }> = {
  untranslated: { text: "未翻译", cls: "bg-slate-700/60 text-slate-300" },
  machine: { text: "AI初稿", cls: "bg-violet-500/20 text-violet-300" },
  translated: { text: "已翻译", cls: "bg-cyan-500/20 text-cyan-300" },
  reviewed: { text: "已审校", cls: "bg-emerald-500/20 text-emerald-300" },
  flagged: { text: "存疑", cls: "bg-rose-500/20 text-rose-300" },
};

export default function Translate() {
  const { projectId } = useParams();
  const navigate = useNavigate();

  const [projects, setProjects] = useState<Project[]>([]);
  const [activeProjectId, setActiveProjectId] = useState<string>(projectId || "");
  const [files, setFiles] = useState<ResourceFile[]>([]);
  const [activeFileId, setActiveFileId] = useState<string | null>(null);
  const [units, setUnits] = useState<TranslationUnit[]>([]);
  const [activeUnitId, setActiveUnitId] = useState<string | null>(null);

  const [filterStatus, setFilterStatus] = useState<string>("all");
  const [search, setSearch] = useState("");
  const [draftTarget, setDraftTarget] = useState("");

  const [profiles, setProfiles] = useState<LlmProfile[]>([]);
  const [selectedProfileId, setSelectedProfileId] = useState("");
  const [translating, setTranslating] = useState(false);
  const [progressEv, setProgressEv] = useState<TranslateProgressEvent | null>(null);
  const [errMsg, setErrMsg] = useState<string | null>(null);

  const [previewText, setPreviewText] = useState<string | null>(null);
  const [editingTargetPath, setEditingTargetPath] = useState<string>("");
  const [refImageUrl, setRefImageUrl] = useState<string | null>(null);

  useEffect(() => {
    api.listProjects().then((list) => {
      setProjects(list);
      if (!activeProjectId && list.length > 0) {
        setActiveProjectId(list[0].id);
      }
    });
    api.listLlmProfiles().then((list) => {
      setProfiles(list);
      const def = list.find((x) => x.isDefault) || list[0];
      if (def) setSelectedProfileId(def.id);
    });
  }, []);

  useEffect(() => {
    if (projectId) setActiveProjectId(projectId);
  }, [projectId]);

  const loadProjectData = async (pid: string) => {
    if (!pid) return;
    const [fsList, uList] = await Promise.all([
      api.listFiles(pid),
      api.listUnits(pid, activeFileId),
    ]);
    setFiles(fsList);
    setUnits(uList);
    if (activeFileId) {
      const f = fsList.find((x) => x.id === activeFileId);
      if (f) setEditingTargetPath(f.targetRelPath);
    }
    if (uList.length > 0 && !uList.some((u) => u.id === activeUnitId)) {
      setActiveUnitId(uList[0].id);
      setDraftTarget(uList[0].target);
    }
  };

  useEffect(() => {
    if (activeProjectId) {
      loadProjectData(activeProjectId).catch(console.error);
    }
  }, [activeProjectId, activeFileId]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    api.onTranslateProgress((ev) => {
      if (ev.projectId === activeProjectId) {
        setProgressEv(ev);
        if (ev.done) {
          loadProjectData(activeProjectId);
        }
      }
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      if (unlisten) unlisten();
    };
  }, [activeProjectId, activeFileId]);

  const filteredUnits = useMemo(() => {
    const q = search.trim().toLowerCase();
    return units.filter((u) => {
      if (filterStatus === "qa") {
        if (!u.qaWarning) return false;
      } else if (filterStatus !== "all" && u.status !== filterStatus) {
        return false;
      }
      if (q) {
        return (
          u.keyPath.toLowerCase().includes(q) ||
          u.source.toLowerCase().includes(q) ||
          u.target.toLowerCase().includes(q)
        );
      }
      return true;
    });
  }, [units, filterStatus, search]);

  const activeUnit = useMemo(
    () => units.find((u) => u.id === activeUnitId) || null,
    [units, activeUnitId]
  );

  const selectUnit = (u: TranslationUnit) => {
    setActiveUnitId(u.id);
    setDraftTarget(u.target);
  };

  const saveUnit = async (status: string) => {
    if (!activeUnit) return;
    const qa = await api.updateUnit(activeUnit.id, draftTarget, status);
    setUnits((prev) =>
      prev.map((u) =>
        u.id === activeUnit.id
          ? { ...u, target: draftTarget, status: status as TranslationUnit["status"], qaWarning: qa }
          : u
      )
    );
    // 自动跳到下一条
    const idx = filteredUnits.findIndex((u) => u.id === activeUnit.id);
    if (idx >= 0 && idx + 1 < filteredUnits.length) {
      selectUnit(filteredUnits[idx + 1]);
    }
  };

  const handleRunAi = async () => {
    if (!activeProjectId || !selectedProfileId) return;
    setErrMsg(null);
    setTranslating(true);
    setProgressEv(null);
    try {
      await api.runLlmTranslate({
        projectId: activeProjectId,
        fileId: activeFileId,
        profileId: selectedProfileId,
        onlyUntranslated: true,
        referenceImageDataUrl: refImageUrl,
      });
      await loadProjectData(activeProjectId);
    } catch (e) {
      setErrMsg(String(e));
    } finally {
      setTranslating(false);
    }
  };

  const handlePreview = async () => {
    if (!activeProjectId) return;
    const targetFid = activeFileId || files[0]?.id;
    if (!targetFid) return;
    const content = await api.previewPatchedFile(activeProjectId, targetFid);
    setPreviewText(content);
  };

  const handleSaveTargetPath = async () => {
    if (!activeFileId || !editingTargetPath.trim()) return;
    await api.updateFileTargetPath(activeFileId, editingTargetPath.trim());
    await loadProjectData(activeProjectId);
  };

  return (
    <div className="flex flex-col h-screen overflow-hidden">
      {/* 顶部工具栏 */}
      <header className="px-4 py-2.5 border-b border-slate-800 bg-[#0E131B] flex flex-wrap items-center justify-between gap-3 shrink-0">
        <div className="flex items-center gap-2.5">
          <select
            value={activeProjectId}
            onChange={(e) => {
              setActiveFileId(null);
              setActiveProjectId(e.target.value);
            }}
            className="px-2.5 py-1.5 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100"
          >
            {projects.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name} ({p.translatedUnits}/{p.totalUnits})
              </option>
            ))}
          </select>

          <select
            value={selectedProfileId}
            onChange={(e) => setSelectedProfileId(e.target.value)}
            className="px-2.5 py-1.5 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-200"
          >
            {profiles.map((p) => (
              <option key={p.id} value={p.id}>
                AI: {p.name}
              </option>
            ))}
          </select>

          <button
            onClick={handleRunAi}
            disabled={translating || !activeProjectId}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded bg-cyan-500 hover:bg-cyan-400 disabled:opacity-50 text-slate-950 font-semibold text-xs"
          >
            {translating ? (
              <Loader2 className="w-3.5 h-3.5 animate-spin" />
            ) : (
              <Sparkles className="w-3.5 h-3.5" />
            )}
            AI 并发翻译未译词条
          </button>
        </div>

        <div className="flex items-center gap-2">
          <label className="flex items-center gap-1.5 px-2.5 py-1.5 rounded border border-slate-700 hover:border-slate-500 text-xs text-slate-300 cursor-pointer">
            <span>{refImageUrl ? "已附带界面截图" : "附带截图上下文"}</span>
            <input
              type="file"
              accept="image/*"
              className="hidden"
              onChange={(e) => {
                const file = e.target.files?.[0];
                if (!file) return;
                const reader = new FileReader();
                reader.onload = () => setRefImageUrl(String(reader.result));
                reader.readAsDataURL(file);
              }}
            />
          </label>
          <button
            onClick={handlePreview}
            disabled={files.length === 0}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded border border-slate-700 hover:border-slate-500 text-xs text-slate-200"
          >
            <Eye className="w-3.5 h-3.5 text-cyan-400" />
            预览回写文件
          </button>
          <button
            onClick={() => navigate(`/package/${activeProjectId}`)}
            disabled={!activeProjectId}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded bg-emerald-500/15 border border-emerald-500/30 hover:bg-emerald-500/25 text-xs text-emerald-300"
          >
            <PackageCheck className="w-3.5 h-3.5" />
            生成 .exe 补丁
          </button>
        </div>
      </header>

      {(errMsg || progressEv) && (
        <div
          className={`px-4 py-2 text-xs border-b border-slate-800 flex items-center justify-between shrink-0 ${
            errMsg
              ? "bg-rose-500/10 text-rose-300"
              : "bg-cyan-500/10 text-cyan-200"
          }`}
        >
          <span>{errMsg || progressEv?.message}</span>
          {progressEv && progressEv.total > 0 && (
            <span className="font-mono">
              {progressEv.completed}/{progressEv.total}
            </span>
          )}
        </div>
      )}

      {/* 三栏主体 */}
      <div className="flex-1 min-h-0 flex">
        {/* 左栏：文件树 */}
        <aside className="w-64 shrink-0 border-r border-slate-800 bg-[#0E131B]/60 flex flex-col overflow-hidden">
          <div className="p-2.5 border-b border-slate-800 text-[11px] font-medium text-slate-400">
            资源文件 ({files.length})
          </div>
          <div className="flex-1 overflow-auto p-1.5 space-y-0.5">
            <button
              onClick={() => setActiveFileId(null)}
              className={`w-full text-left px-2.5 py-1.5 rounded text-xs ${
                activeFileId === null
                  ? "bg-cyan-500/15 text-cyan-300"
                  : "text-slate-300 hover:bg-slate-800/50"
              }`}
            >
              全部文件 ({units.length})
            </button>
            {files.map((f) => (
              <button
                key={f.id}
                onClick={() => setActiveFileId(f.id)}
                className={`w-full text-left px-2.5 py-1.5 rounded text-xs flex items-center justify-between gap-2 ${
                  activeFileId === f.id
                    ? "bg-cyan-500/15 text-cyan-300"
                    : "text-slate-300 hover:bg-slate-800/50"
                }`}
              >
                <span className="truncate font-mono text-[11px]">{f.relPath}</span>
                <span className="text-[10px] font-mono text-slate-500 shrink-0">
                  {f.translatedCount}/{f.unitCount}
                </span>
              </button>
            ))}
          </div>

          {activeFileId && (
            <div className="p-2.5 border-t border-slate-800 space-y-1.5 bg-[#0A0E14]">
              <div className="text-[10px] text-slate-400">补丁写入目标相对路径：</div>
              <div className="flex gap-1">
                <input
                  value={editingTargetPath}
                  onChange={(e) => setEditingTargetPath(e.target.value)}
                  className="flex-1 min-w-0 px-2 py-1 rounded bg-[#121821] border border-slate-700 text-[11px] font-mono text-slate-200"
                />
                <button
                  onClick={handleSaveTargetPath}
                  className="px-2 py-1 rounded bg-slate-800 hover:bg-slate-700 text-[11px] text-cyan-300"
                >
                  改
                </button>
              </div>
            </div>
          )}
        </aside>

        {/* 中栏：词条列表 */}
        <section className="w-96 shrink-0 border-r border-slate-800 flex flex-col overflow-hidden bg-[#0A0E14]">
          <div className="p-2.5 border-b border-slate-800 space-y-2">
            <div className="relative">
              <Search className="w-3.5 h-3.5 text-slate-500 absolute left-2.5 top-2" />
              <input
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder="搜索键名、原文或译文..."
                className="w-full pl-8 pr-2.5 py-1.5 rounded bg-[#121821] border border-slate-800 text-xs text-slate-200 focus:outline-none focus:border-cyan-500/50"
              />
            </div>
            <div className="flex flex-wrap gap-1 text-[11px]">
              {[
                { id: "all", label: "全部" },
                { id: "untranslated", label: "未译" },
                { id: "machine", label: "AI初稿" },
                { id: "translated", label: "已译" },
                { id: "reviewed", label: "已审" },
                { id: "qa", label: "QA告警" },
              ].map((t) => (
                <button
                  key={t.id}
                  onClick={() => setFilterStatus(t.id)}
                  className={`px-2 py-0.5 rounded ${
                    filterStatus === t.id
                      ? "bg-cyan-500/20 text-cyan-300 border border-cyan-500/30"
                      : "text-slate-400 hover:bg-slate-800"
                  }`}
                >
                  {t.label}
                </button>
              ))}
            </div>
          </div>

          <div className="flex-1 overflow-auto divide-y divide-slate-800/60">
            {filteredUnits.map((u) => {
              const st = STATUS_LABEL[u.status] || STATUS_LABEL.untranslated;
              const isSelected = u.id === activeUnitId;
              return (
                <div
                  key={u.id}
                  onClick={() => selectUnit(u)}
                  className={`p-3 cursor-pointer transition ${
                    isSelected ? "bg-cyan-500/10 border-l-2 border-l-cyan-400" : "hover:bg-slate-900/50"
                  }`}
                >
                  <div className="flex items-center justify-between gap-2 mb-1">
                    <span className="text-[11px] font-mono text-slate-500 truncate">
                      {u.keyPath}
                    </span>
                    <div className="flex items-center gap-1 shrink-0">
                      {u.qaWarning && (
                        <span title={u.qaWarning}>
                          <AlertTriangle className="w-3.5 h-3.5 text-amber-400" />
                        </span>
                      )}
                      <span className={`text-[10px] px-1.5 py-0.5 rounded ${st.cls}`}>
                        {st.text}
                      </span>
                    </div>
                  </div>
                  <div className="text-xs text-slate-200 line-clamp-2">{u.source}</div>
                  {u.target && (
                    <div className="text-xs text-amber-300/90 mt-1 line-clamp-2">{u.target}</div>
                  )}
                </div>
              );
            })}
          </div>
        </section>

        {/* 右栏：编辑器 */}
        <section className="flex-1 min-w-0 p-5 overflow-auto bg-[#0D1219]">
          {activeUnit ? (
            <div className="max-w-3xl space-y-4">
              <div className="flex items-center justify-between">
                <span className="text-xs font-mono text-cyan-400">{activeUnit.keyPath}</span>
                <span className="text-[11px] text-slate-500">
                  Ctrl+Enter 保存并下一条
                </span>
              </div>

              <div className="rounded-lg border border-slate-800 bg-[#121821] p-4">
                <div className="text-[11px] text-slate-400 mb-1.5">原文 (Source)</div>
                <pre className="text-sm text-slate-100 whitespace-pre-wrap font-mono leading-relaxed">
                  {activeUnit.source}
                </pre>
              </div>

              <div className="rounded-lg border border-slate-800 bg-[#121821] p-4">
                <div className="text-[11px] text-slate-400 mb-1.5">中文译文 (Target)</div>
                <textarea
                  rows={5}
                  value={draftTarget}
                  onChange={(e) => setDraftTarget(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.ctrlKey && e.key === "Enter") {
                      e.preventDefault();
                      saveUnit("translated");
                    }
                  }}
                  placeholder="输入中文译文，或点击上方「AI 并发翻译」..."
                  className="w-full p-3 rounded bg-[#0A0E14] border border-slate-700 text-sm text-amber-200 font-sans focus:outline-none focus:border-cyan-400"
                />
              </div>

              {activeUnit.qaWarning && (
                <div className="p-3 rounded-md bg-amber-500/10 border border-amber-500/30 text-xs text-amber-300 flex items-center gap-2">
                  <AlertTriangle className="w-4 h-4 shrink-0" />
                  <span>QA 占位符告警：{activeUnit.qaWarning}</span>
                </div>
              )}

              <div className="flex items-center gap-2.5">
                <button
                  onClick={() => saveUnit("translated")}
                  className="flex items-center gap-1.5 px-4 py-2 rounded bg-cyan-500 hover:bg-cyan-400 text-slate-950 font-semibold text-xs"
                >
                  <Save className="w-3.5 h-3.5" /> 保存译文 (Ctrl+Enter)
                </button>
                <button
                  onClick={() => saveUnit("reviewed")}
                  className="flex items-center gap-1.5 px-4 py-2 rounded bg-emerald-500/20 border border-emerald-500/40 hover:bg-emerald-500/30 text-emerald-300 font-medium text-xs"
                >
                  <Check className="w-3.5 h-3.5" /> 标记已审校
                </button>
              </div>
            </div>
          ) : (
            <div className="h-full flex items-center justify-center text-xs text-slate-500">
              请在左侧选择项目和词条开始翻译
            </div>
          )}
        </section>
      </div>

      {/* 回写预览弹窗 */}
      {previewText !== null && (
        <div className="fixed inset-0 z-50 bg-black/70 flex items-center justify-center p-6">
          <div className="bg-[#121821] border border-slate-700 rounded-lg w-full max-w-4xl max-h-[85vh] flex flex-col">
            <div className="px-4 py-3 border-b border-slate-800 flex items-center justify-between">
              <span className="text-sm font-medium text-slate-200">
                格式保留回写实时预览（实际写入补丁的内容）
              </span>
              <button onClick={() => setPreviewText(null)} className="text-slate-400 hover:text-white">
                <X className="w-4 h-4" />
              </button>
            </div>
            <pre className="flex-1 overflow-auto p-4 text-xs font-mono text-slate-200 whitespace-pre-wrap">
              {previewText}
            </pre>
          </div>
        </div>
      )}
    </div>
  );
}
