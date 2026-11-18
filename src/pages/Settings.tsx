import { useEffect, useState } from "react";
import { api } from "@/lib/tauri";
import type { LlmProfile } from "@/types";
import { Plus, Trash2, CheckCircle2, Loader2, Zap, RefreshCw } from "lucide-react";

export default function Settings() {
  const [profiles, setProfiles] = useState<LlmProfile[]>([]);
  const [editing, setEditing] = useState<LlmProfile | null>(null);
  const [availableModels, setAvailableModels] = useState<string[]>([]);
  const [fetchingModels, setFetchingModels] = useState(false);
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

  const handleFetchModels = async () => {
    if (!editing) return;
    setFetchingModels(true);
    setTestResult(null);
    try {
      const models = await api.fetchLlmModels(editing.baseUrl, editing.apiKey);
      setAvailableModels(models);
      if (models.length > 0 && !models.includes(editing.model)) {
        setEditing({ ...editing, model: models[0] });
      }
      setTestResult({ ok: true, msg: `成功获取到 ${models.length} 个可用模型` });
    } catch (e) {
      setTestResult({ ok: false, msg: String(e) });
    } finally {
      setFetchingModels(false);
    }
  };

  const handleTest = async () => {
    if (!editing) return;
    setTestingId(editing.id || "new");
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
    setAvailableModels([]);
    setEditing({
      id: "",
      name: "本地网关 / 自定义 OpenAI",
      provider: "custom",
      baseUrl: "http://127.0.0.1:8045/v1",
      apiKey: "",
      model: "gemini-3.8-flash",
      concurrency: 4,
      batchSize: 25,
      temperature: 0.2,
      reasoningEffort: "low",
      maxTokens: 16384,
      contextWindow: 1048576,
      maxRetries: 3,
      supportsVision: true,
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
            支持任意 OpenAI 兼容网关、自动拉取模型目录、思考强度 (reasoning_effort)、上下文窗口、多模态与失败重试
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
        <div className="space-y-2">
          {profiles.map((p) => (
            <div
              key={p.id}
              onClick={() => {
                setEditing(p);
                setAvailableModels([]);
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
                {p.reasoningEffort !== "none" ? ` · 思考:${p.reasoningEffort}` : ""}
              </div>
            </div>
          ))}
        </div>

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
                  <label className="block text-xs text-slate-400 mb-1">思考强度 (reasoning_effort)</label>
                  <select
                    value={editing.reasoningEffort || "none"}
                    onChange={(e) => setEditing({ ...editing, reasoningEffort: e.target.value })}
                    className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100 font-mono"
                  >
                    <option value="none">关闭 (默认)</option>
                    <option value="low">low (轻量推理，速度快)</option>
                    <option value="medium">medium (均衡推理)</option>
                    <option value="high">high (深度推理)</option>
                  </select>
                </div>

                <div className="sm:col-span-2">
                  <label className="block text-xs text-slate-400 mb-1">API Base URL</label>
                  <input
                    value={editing.baseUrl}
                    onChange={(e) => setEditing({ ...editing, baseUrl: e.target.value })}
                    placeholder="http://127.0.0.1:8045/v1"
                    className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100 font-mono"
                  />
                </div>

                <div className="sm:col-span-2">
                  <label className="block text-xs text-slate-400 mb-1">API Key</label>
                  <input
                    type="password"
                    value={editing.apiKey}
                    onChange={(e) => setEditing({ ...editing, apiKey: e.target.value })}
                    placeholder="sk-..."
                    className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100 font-mono"
                  />
                </div>

                <div className="sm:col-span-2">
                  <div className="flex items-center justify-between mb-1">
                    <label className="text-xs text-slate-400">模型名称 (Model)</label>
                    <button
                      onClick={handleFetchModels}
                      disabled={fetchingModels}
                      className="flex items-center gap-1 text-[11px] text-cyan-400 hover:text-cyan-300"
                    >
                      <RefreshCw className={`w-3 h-3 ${fetchingModels ? "animate-spin" : ""}`} />
                      获取可用模型 (/v1/models)
                    </button>
                  </div>
                  {availableModels.length > 0 ? (
                    <select
                      value={editing.model}
                      onChange={(e) => setEditing({ ...editing, model: e.target.value })}
                      className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-cyan-500/40 text-xs text-slate-100 font-mono"
                    >
                      {availableModels.map((m) => (
                        <option key={m} value={m}>
                          {m}
                        </option>
                      ))}
                    </select>
                  ) : (
                    <input
                      value={editing.model}
                      onChange={(e) => setEditing({ ...editing, model: e.target.value })}
                      className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100 font-mono"
                    />
                  )}
                </div>

                <div>
                  <label className="block text-xs text-slate-400 mb-1">上下文窗口 (Context Window)</label>
                  <input
                    type="number"
                    value={editing.contextWindow}
                    onChange={(e) => setEditing({ ...editing, contextWindow: Number(e.target.value) || 128000 })}
                    className="w-full px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100 font-mono"
                  />
                </div>
                <div>
                  <label className="block text-xs text-slate-400 mb-1">最大输出 Token (max_tokens)</label>
                  <input
                    type="number"
                    value={editing.maxTokens}
                    onChange={(e) => setEditing({ ...editing, maxTokens: Number(e.target.value) || 8192 })}
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
                  <label className="block text-xs text-slate-400 mb-1">单批词条数 / 失败重试次数</label>
                  <div className="flex gap-2">
                    <input
                      type="number"
                      min={5}
                      max={60}
                      value={editing.batchSize}
                      onChange={(e) => setEditing({ ...editing, batchSize: Number(e.target.value) || 20 })}
                      className="w-1/2 px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100 font-mono"
                      title="单批词条数"
                    />
                    <input
                      type="number"
                      min={0}
                      max={5}
                      value={editing.maxRetries}
                      onChange={(e) => setEditing({ ...editing, maxRetries: Number(e.target.value) || 0 })}
                      className="w-1/2 px-3 py-2 rounded bg-[#0A0E14] border border-slate-700 text-xs text-slate-100 font-mono"
                      title="失败重试次数"
                    />
                  </div>
                </div>
              </div>

              <div className="flex flex-wrap items-center gap-5">
                <label className="flex items-center gap-2 text-xs text-slate-300 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={editing.supportsVision}
                    onChange={(e) => setEditing({ ...editing, supportsVision: e.target.checked })}
                  />
                  启用多模态图片上下文（支持界面截图辅助翻译）
                </label>
                <label className="flex items-center gap-2 text-xs text-slate-300 cursor-pointer">
                  <input
                    type="checkbox"
                    checked={editing.isDefault}
                    onChange={(e) => setEditing({ ...editing, isDefault: e.target.checked })}
                  />
                  设为默认翻译模型
                </label>
              </div>

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
