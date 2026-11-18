import { useEffect, useState } from "react";
import { api } from "@/lib/tauri";
import type { LlmProfile } from "@/types";
import { Plus, Trash2, CheckCircle2, Loader2, Zap } from "lucide-react";

export default function Settings() {
  const [profiles, setProfiles] = useState<LlmProfile[]>([]);
  const [editing, setEditing] = useState<LlmProfile | null>(null);
  const [testingId, setTestingId] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<{ ok: boolean; msg: string } | null>(null);

  const load = () => {
    api.listLlmProfiles().then((list) => {
      setProfiles(list);
      if (!editing && list.length > 0) {
        setEditing(list.find((x) => x.isDefault) || list[0]);
      }
    });
  };

  useEffect(() => {
    load();
  }, []);

  const handleSave = async () => {
    if (!editing) return;
    await api.upsertLlmProfile(editing);
    setTestResult({ ok: true, msg: "配置已保存至 SQLite" });
    load();
  };

  const handleTest = async () => {
    if (!editing) return;
    setTestingId(editing.id);
    setTestResult(null);
    try {
      const msg = await api.testLlmConnection(editing);
      setTestResult({ ok: true, msg });
    } catch (e) {
      setTestResult({ ok: false, msg: String(e) });
    } finally {
      setTestingId(null);
    }
  };

  const handleAddNew = () => {
    setEditing({
      id: "",
      name: "自定义 OpenAI 兼容模型",
      provider: "custom",
      baseUrl: "https://api.openai.com/v1",
      apiKey: "",
      model: "gpt-4o-mini",
      concurrency: 4,
      batchSize: 25,
      temperature: 0.2,
      isDefault: false,
    });
    setTestResult(null);
  };

  const handleDelete = async (id: string) => {
    await api.deleteLlmProfile(id);
    setEditing(null);
    load();
  };

  return (
    <div className="p-6 max-w-5xl mx-auto w-full space-y-5">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold text-slate-100">AI 大模型配置</h1>
          <p className="text-xs text-slate-400 mt-1">
            支持 DeepSeek、通义千问、OpenAI、本地 Ollama 及任意 OpenAI 兼容接口（Rust 原生高并发调用）
          </p>
        </div>
        <button
          onClick={handleAddNew}
          className="flex items-center gap-1.5 px-3 py-2 rounded-md bg-cyan-500 hover:bg-cyan-400 text-slate-950 font-medium text-xs"
        >
          <Plus className="w-3.5 h-3.5" /> 新增配置
        </button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-5">
        {/* 左侧列表 */}
        <div className="space-y-2">
          {profiles.map((p) => (
            <div
              key={p.id}
              onClick={() => {
                setEditing(p);
                setTestResult(null);
              }}
              className={`p-3 rounded-lg border cursor-pointer transition ${
                editing?.id === p.id
                  ? "border-cyan-500/50 bg-cyan-500/10"
                  : "border-slate-800 bg-[#121821] hover:border-slate-700"
              }`}
            >
              <div className="flex items-center justify-between">
                <span className="text-xs font-medium text-slate-100">{p.name}</span>
                {p.isDefault && (
                  <span className="text-[10px] px-1.5 py-0.5 rounded bg-emerald-500/15 text-emerald-300">
                    默认
                  </span>
                )}
              </div>
              <div className="text-[11px] font-mono text-slate-400 mt-1 truncate">
                {p.model} · 并发 {p.concurrency}
              </div>
            </div>
          ))}
        </div>

        {/* 右侧表单 */}
        <div className="md:col-span-2 rounded-lg border border-slate-800 bg-[#121821] p-5 space-y-4">
          {editing ? (
            <>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <div>
                  <label className="block text-xs text-slate-400 mb-1">配置名称</label>
                  <input
                    value={editing.name}
                    onChange={(e) => setEditing({ ...editing, name: e.target.value })}
                    className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100"
                  />
                </div>
                <div>
                  <label className="block text-xs text-slate-400 mb-1">模型名称 (Model)</label>
                  <input
                    value={editing.model}
                    onChange={(e) => setEditing({ ...editing, model: e.target.value })}
                    className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100 font-mono"
                  />
                </div>
                <div className="sm:col-span-2">
                  <label className="block text-xs text-slate-400 mb-1">API Base URL</label>
                  <input
                    value={editing.baseUrl}
                    onChange={(e) => setEditing({ ...editing, baseUrl: e.target.value })}
                    className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100 font-mono"
                  />
                </div>
                <div className="sm:col-span-2">
                  <label className="block text-xs text-slate-400 mb-1">API Key（本地 Ollama 可留空或填 ollama）</label>
                  <input
                    type="password"
                    value={editing.apiKey}
                    onChange={(e) => setEditing({ ...editing, apiKey: e.target.value })}
                    placeholder="sk-..."
                    className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100 font-mono"
                  />
                </div>
                <div>
                  <label className="block text-xs text-slate-400 mb-1">并发请求数 (1-12)</label>
                  <input
                    type="number"
                    min={1}
                    max={12}
                    value={editing.concurrency}
                    onChange={(e) => setEditing({ ...editing, concurrency: Number(e.target.value) || 1 })}
                    className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100 font-mono"
                  />
                </div>
                <div>
                  <label className="block text-xs text-slate-400 mb-1">单批词条数 (5-60)</label>
                  <input
                    type="number"
                    min={5}
                    max={60}
                    value={editing.batchSize}
                    onChange={(e) => setEditing({ ...editing, batchSize: Number(e.target.value) || 20 })}
                    className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100 font-mono"
                  />
                </div>
              </div>

              <label className="flex items-center gap-2 text-xs text-slate-300 cursor-pointer">
                <input
                  type="checkbox"
                  checked={editing.isDefault}
                  onChange={(e) => setEditing({ ...editing, isDefault: e.target.checked })}
                />
                设为默认翻译模型
              </label>

              {testResult && (
                <div
                  className={`p-2.5 rounded text-xs flex items-center gap-2 ${
                    testResult.ok
                      ? "bg-emerald-500/10 border border-emerald-500/30 text-emerald-300"
                      : "bg-rose-500/10 border border-rose-500/30 text-rose-300"
                  }`}
                >
                  <CheckCircle2 className="w-4 h-4 shrink-0" />
                  <span className="break-all">{testResult.msg}</span>
                </div>
              )}

              <div className="flex items-center justify-between pt-2 border-t border-slate-800">
                <div className="flex items-center gap-2">
                  <button
                    onClick={handleTest}
                    disabled={testingId !== null}
                    className="flex items-center gap-1.5 px-3 py-2 rounded border border-slate-700 hover:border-cyan-500/50 text-xs text-slate-200"
                  >
                    {testingId ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <Zap className="w-3.5 h-3.5 text-amber-400" />}
                    测试连通性
                  </button>
                  {editing.id && profiles.length > 1 && (
                    <button
                      onClick={() => handleDelete(editing.id)}
                      className="flex items-center gap-1 px-2.5 py-2 rounded text-xs text-rose-400 hover:bg-rose-500/10"
                    >
                      <Trash2 className="w-3.5 h-3.5" /> 删除
                    </button>
                  )}
                </div>
                <button
                  onClick={handleSave}
                  className="px-4 py-2 rounded bg-cyan-500 hover:bg-cyan-400 text-slate-950 font-semibold text-xs"
                >
                  保存配置
                </button>
              </div>
            </>
          ) : (
            <div className="text-xs text-slate-500 py-8 text-center">请选择或新增一个模型配置</div>
          )}
        </div>
      </div>
    </div>
  );
}
