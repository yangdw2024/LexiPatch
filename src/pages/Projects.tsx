import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api } from "@/lib/tauri";
import type { Project } from "@/types";
import { Plus, Trash2, Languages, PackageCheck } from "lucide-react";

export default function Projects() {
  const [projects, setProjects] = useState<Project[]>([]);

  const load = () => {
    api.listProjects().then(setProjects).catch(console.error);
  };

  useEffect(() => {
    load();
  }, []);

  const handleDelete = async (id: string, name: string) => {
    if (!confirm(`确定删除项目「${name}」及其所有词条和构建记录吗？`)) return;
    await api.deleteProject(id);
    load();
  };

  return (
    <div className="p-6 space-y-5 max-w-7xl mx-auto w-full">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold text-slate-100">项目管理</h1>
          <p className="text-xs text-slate-400 mt-1">
            管理本地软件汉化工程、目标目录定位规则与补丁发布状态
          </p>
        </div>
        <Link
          to="/projects/new"
          className="flex items-center gap-1.5 px-3.5 py-2 rounded-md bg-cyan-500 hover:bg-cyan-400 text-slate-950 font-medium text-xs"
        >
          <Plus className="w-3.5 h-3.5" />
          新建项目
        </Link>
      </div>

      <div className="rounded-lg border border-slate-800 bg-[#121821] overflow-hidden">
        <table className="w-full text-left text-xs">
          <thead className="bg-slate-900/70 text-slate-400 border-b border-slate-800">
            <tr>
              <th className="py-3 px-4 font-medium">项目名称</th>
              <th className="py-3 px-4 font-medium">目标软件 / 版本</th>
              <th className="py-3 px-4 font-medium">词条进度</th>
              <th className="py-3 px-4 font-medium">自动定位可执行文件</th>
              <th className="py-3 px-4 font-medium text-right">操作</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-800/80">
            {projects.length === 0 ? (
              <tr>
                <td colSpan={5} className="py-12 text-center text-slate-500">
                  暂无项目，请点击右上角「新建项目」
                </td>
              </tr>
            ) : (
              projects.map((p) => {
                const pct =
                  p.totalUnits > 0
                    ? Math.round((p.translatedUnits / p.totalUnits) * 100)
                    : 0;
                return (
                  <tr key={p.id} className="hover:bg-slate-800/30">
                    <td className="py-3 px-4">
                      <div className="font-medium text-slate-100">{p.name}</div>
                      <div className="text-[11px] text-slate-500 truncate max-w-xs">
                        {p.sourceDir || "手动导入"}
                      </div>
                    </td>
                    <td className="py-3 px-4 font-mono text-slate-300">
                      {p.software} {p.version}
                    </td>
                    <td className="py-3 px-4">
                      <div className="flex items-center gap-2">
                        <div className="w-24 h-1.5 rounded-full bg-slate-800 overflow-hidden">
                          <div
                            className="h-full bg-cyan-400"
                            style={{ width: `${pct}%` }}
                          />
                        </div>
                        <span className="font-mono text-slate-300">
                          {p.translatedUnits}/{p.totalUnits} ({pct}%)
                        </span>
                      </div>
                    </td>
                    <td className="py-3 px-4 font-mono text-slate-400">
                      {p.exeName || "-"}
                    </td>
                    <td className="py-3 px-4 text-right space-x-2">
                      <Link
                        to={`/translate/${p.id}`}
                        className="inline-flex items-center gap-1 px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-200"
                      >
                        <Languages className="w-3 h-3" /> 工作台
                      </Link>
                      <Link
                        to={`/package/${p.id}`}
                        className="inline-flex items-center gap-1 px-2.5 py-1 rounded bg-cyan-500/15 border border-cyan-500/30 hover:bg-cyan-500/25 text-cyan-300"
                      >
                        <PackageCheck className="w-3 h-3" /> 生成补丁
                      </Link>
                      <button
                        onClick={() => handleDelete(p.id, p.name)}
                        className="inline-flex items-center gap-1 px-2 py-1 rounded hover:bg-rose-500/15 text-slate-400 hover:text-rose-400"
                        title="删除项目"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </td>
                  </tr>
                );
              })
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
