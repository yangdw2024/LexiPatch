import { useEffect, useState } from "react";
import { api } from "@/lib/tauri";
import type { GlossaryEntry, TranslationMemory } from "@/types";
import { Plus, Trash2 } from "lucide-react";

export default function Glossary() {
  const [tab, setTab] = useState<"glossary" | "memory">("glossary");
  const [glossary, setGlossary] = useState<GlossaryEntry[]>([]);
  const [memory, setMemory] = useState<TranslationMemory[]>([]);

  const [source, setSource] = useState("");
  const [target, setTarget] = useState("");
  const [domain, setDomain] = useState("ui");
  const [dnt, setDnt] = useState(false);

  const load = () => {
    api.listGlossary(null).then(setGlossary).catch(console.error);
    api.listMemory().then(setMemory).catch(console.error);
  };

  useEffect(() => {
    load();
  }, []);

  const handleAdd = async () => {
    if (!source.trim()) return;
    await api.upsertGlossary({
      id: "",
      projectId: "global",
      source: source.trim(),
      target: dnt ? source.trim() : target.trim(),
      domain,
      context: "",
      doNotTranslate: dnt,
    });
    setSource("");
    setTarget("");
    setDnt(false);
    load();
  };

  const handleDelete = async (id: string) => {
    await api.deleteGlossary(id);
    load();
  };

  return (
    <div className="p-6 max-w-6xl mx-auto w-full space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold text-slate-100">术语库与翻译记忆</h1>
          <p className="text-xs text-slate-400 mt-1">
            术语库会自动注入大模型提示词以保证译名统一；人工确认的译文会自动沉淀为翻译记忆
          </p>
        </div>
        <div className="flex rounded-md border border-slate-800 bg-[#121821] p-0.5 text-xs">
          <button
            onClick={() => setTab("glossary")}
            className={`px-3 py-1.5 rounded ${tab === "glossary" ? "bg-cyan-500 text-slate-950 font-medium" : "text-slate-400"}`}
          >
            术语库 ({glossary.length})
          </button>
          <button
            onClick={() => setTab("memory")}
            className={`px-3 py-1.5 rounded ${tab === "memory" ? "bg-cyan-500 text-slate-950 font-medium" : "text-slate-400"}`}
          >
            翻译记忆 ({memory.length})
          </button>
        </div>
      </div>

      {tab === "glossary" ? (
        <div className="space-y-4">
          <div className="rounded-lg border border-slate-800 bg-[#121821] p-4 flex flex-wrap items-end gap-3">
            <div className="flex-1 min-w-[160px]">
              <label className="block text-[11px] text-slate-400 mb-1">英文术语</label>
              <input
                value={source}
                onChange={(e) => setSource(e.target.value)}
                placeholder="例如：Workspace"
                className="w-full px-3 py-1.5 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100"
              />
            </div>
            <div className="flex-1 min-w-[160px]">
              <label className="block text-[11px] text-slate-400 mb-1">标准中文译名</label>
              <input
                value={target}
                disabled={dnt}
                onChange={(e) => setTarget(e.target.value)}
                placeholder={dnt ? "保持原文不翻译" : "例如：工作区"}
                className="w-full px-3 py-1.5 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100 disabled:opacity-50"
              />
            </div>
            <div className="w-28">
              <label className="block text-[11px] text-slate-400 mb-1">领域</label>
              <input
                value={domain}
                onChange={(e) => setDomain(e.target.value)}
                className="w-full px-3 py-1.5 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100"
              />
            </div>
            <label className="flex items-center gap-1.5 text-xs text-slate-300 pb-1.5 cursor-pointer">
              <input type="checkbox" checked={dnt} onChange={(e) => setDnt(e.target.checked)} />
              禁译词
            </label>
            <button
              onClick={handleAdd}
              className="flex items-center gap-1 px-3.5 py-1.5 rounded bg-cyan-500 hover:bg-cyan-400 text-slate-950 font-medium text-xs"
            >
              <Plus className="w-3.5 h-3.5" /> 添加
            </button>
          </div>

          <div className="rounded-lg border border-slate-800 bg-[#121821] overflow-hidden">
            <table className="w-full text-left text-xs">
              <thead className="bg-slate-900/70 text-slate-400 border-b border-slate-800">
                <tr>
                  <th className="py-2.5 px-4">原文</th>
                  <th className="py-2.5 px-4">译文</th>
                  <th className="py-2.5 px-4">领域</th>
                  <th className="py-2.5 px-4">属性</th>
                  <th className="py-2.5 px-4 text-right">操作</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-800/70">
                {glossary.map((g) => (
                  <tr key={g.id} className="hover:bg-slate-800/30">
                    <td className="py-2.5 px-4 font-mono text-slate-200">{g.source}</td>
                    <td className="py-2.5 px-4 text-amber-300">{g.target}</td>
                    <td className="py-2.5 px-4 text-slate-400">{g.domain}</td>
                    <td className="py-2.5 px-4">
                      {g.doNotTranslate ? (
                        <span className="px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-300 text-[10px]">
                          保持原文
                        </span>
                      ) : (
                        <span className="px-1.5 py-0.5 rounded bg-cyan-500/15 text-cyan-300 text-[10px]">
                          强制译名
                        </span>
                      )}
                    </td>
                    <td className="py-2.5 px-4 text-right">
                      <button
                        onClick={() => handleDelete(g.id)}
                        className="text-slate-500 hover:text-rose-400"
                      >
                        <Trash2 className="w-3.5 h-3.5 inline" />
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      ) : (
        <div className="rounded-lg border border-slate-800 bg-[#121821] overflow-hidden">
          <table className="w-full text-left text-xs">
            <thead className="bg-slate-900/70 text-slate-400 border-b border-slate-800">
              <tr>
                <th className="py-2.5 px-4">原文</th>
                <th className="py-2.5 px-4">已沉淀译文</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/70">
              {memory.length === 0 ? (
                <tr>
                  <td colSpan={2} className="py-10 text-center text-slate-500">
                    在工作台保存或审校译文后，会自动沉淀到翻译记忆库
                  </td>
                </tr>
              ) : (
                memory.map((m) => (
                  <tr key={m.id} className="hover:bg-slate-800/30">
                    <td className="py-2.5 px-4 font-mono text-slate-300">{m.source}</td>
                    <td className="py-2.5 px-4 text-emerald-300">{m.target}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
