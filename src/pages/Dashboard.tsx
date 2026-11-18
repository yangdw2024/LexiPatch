import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api } from "@/lib/tauri";
import type { DashboardStats, Project } from "@/types";
import {
  FolderKanban,
  Languages,
  AlertTriangle,
  PackageCheck,
  ArrowRight,
  Plus,
  Cpu,
} from "lucide-react";

export default function Dashboard() {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [projects, setProjects] = useState<Project[]>([]);

  useEffect(() => {
    api.getDashboardStats().then(setStats).catch(console.error);
    api.listProjects().then(setProjects).catch(console.error);
  }, []);

  const pct =
    stats && stats.totalUnits > 0
      ? Math.round((stats.translatedUnits / stats.totalUnits) * 100)
      : 0;

  return (
    <div className="p-6 space-y-6 max-w-7xl mx-auto w-full">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold text-slate-100">工作台概览</h1>
          <p className="text-xs text-slate-400 mt-1">
            Rust + SQLite 驱动 · 扫描真实软件目录 → 并发大模型翻译 → 导出独立 .exe 汉化补丁
          </p>
        </div>
        <div className="flex items-center gap-2.5">
          <Link
            to="/settings"
            className="flex items-center gap-1.5 px-3 py-2 rounded-md border border-slate-700 hover:border-slate-500 text-xs text-slate-200 transition"
          >
            <Cpu className="w-3.5 h-3.5 text-cyan-400" />
            配置 AI 模型
          </Link>
          <Link
            to="/projects/new"
            className="flex items-center gap-1.5 px-3.5 py-2 rounded-md bg-cyan-500 hover:bg-cyan-400 text-slate-950 font-medium text-xs transition"
          >
            <Plus className="w-3.5 h-3.5" />
            新建汉化项目
          </Link>
        </div>
      </div>

      {/* 统计卡片 */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <StatCard
          label="汉化项目"
          value={stats?.projectCount ?? 0}
          sub={`词条总数 ${stats?.totalUnits ?? 0}`}
          icon={<FolderKanban className="w-4 h-4 text-cyan-400" />}
        />
        <StatCard
          label="已翻译词条"
          value={`${pct}%`}
          sub={`${stats?.translatedUnits ?? 0} / ${stats?.totalUnits ?? 0}（已审校 ${stats?.reviewedUnits ?? 0}）`}
          icon={<Languages className="w-4 h-4 text-emerald-400" />}
        />
        <StatCard
          label="QA 占位符告警"
          value={stats?.qaWarningUnits ?? 0}
          sub="自动校验 %s / {0} / 标签"
          icon={<AlertTriangle className="w-4 h-4 text-amber-400" />}
        />
        <StatCard
          label="已生成补丁"
          value={stats?.patchCount ?? 0}
          sub={`术语 ${stats?.glossaryCount ?? 0} · 记忆 ${stats?.memoryCount ?? 0}`}
          icon={<PackageCheck className="w-4 h-4 text-cyan-400" />}
        />
      </div>

      {/* 最近项目 */}
      <div className="rounded-lg border border-slate-800 bg-[#121821] overflow-hidden">
        <div className="px-4 py-3 border-b border-slate-800 flex items-center justify-between">
          <span className="text-sm font-medium text-slate-200">最近项目</span>
          <Link
            to="/projects"
            className="text-xs text-cyan-400 hover:text-cyan-300 flex items-center gap-1"
          >
            查看全部 <ArrowRight className="w-3 h-3" />
          </Link>
        </div>
        {projects.length === 0 ? (
          <div className="p-10 text-center space-y-3">
            <p className="text-sm text-slate-400">
              当前还没有汉化项目。点击「新建汉化项目」，直接扫描任意软件安装目录即可开始。
            </p>
            <Link
              to="/projects/new"
              className="inline-flex items-center gap-1.5 px-4 py-2 rounded-md bg-cyan-500 text-slate-950 font-medium text-xs"
            >
              <Plus className="w-3.5 h-3.5" /> 立即创建第一个项目
            </Link>
          </div>
        ) : (
          <div className="divide-y divide-slate-800/80">
            {projects.slice(0, 6).map((p) => {
              const progress =
                p.totalUnits > 0
                  ? Math.round((p.translatedUnits / p.totalUnits) * 100)
                  : 0;
              return (
                <div
                  key={p.id}
                  className="px-4 py-3 flex items-center justify-between hover:bg-slate-800/30 transition"
                >
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium text-slate-100 truncate">
                        {p.name}
                      </span>
                      <span className="text-[11px] font-mono px-1.5 py-0.5 rounded bg-slate-800 text-slate-300">
                        {p.software} {p.version}
                      </span>
                    </div>
                    <div className="text-xs text-slate-500 mt-0.5 truncate">
                      {p.sourceDir || "手动导入文件"} · {p.translatedUnits}/{p.totalUnits} 词条
                    </div>
                  </div>
                  <div className="flex items-center gap-4 shrink-0">
                    <div className="w-28">
                      <div className="flex justify-between text-[11px] text-slate-400 mb-1">
                        <span>进度</span>
                        <span className="font-mono text-cyan-300">{progress}%</span>
                      </div>
                      <div className="h-1.5 rounded-full bg-slate-800 overflow-hidden">
                        <div
                          className="h-full bg-cyan-400 transition-all"
                          style={{ width: `${progress}%` }}
                        />
                      </div>
                    </div>
                    <Link
                      to={`/translate/${p.id}`}
                      className="px-2.5 py-1.5 rounded bg-slate-800 hover:bg-slate-700 text-xs text-slate-200"
                    >
                      翻译
                    </Link>
                    <Link
                      to={`/package/${p.id}`}
                      className="px-2.5 py-1.5 rounded bg-cyan-500/15 border border-cyan-500/30 hover:bg-cyan-500/25 text-xs text-cyan-300"
                    >
                      打包 .exe
                    </Link>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

function StatCard({
  label,
  value,
  sub,
  icon,
}: {
  label: string;
  value: string | number;
  sub: string;
  icon: React.ReactNode;
}) {
  return (
    <div className="rounded-lg border border-slate-800 bg-[#121821] p-4">
      <div className="flex items-center justify-between text-xs text-slate-400">
        <span>{label}</span>
        {icon}
      </div>
      <div className="text-2xl font-semibold font-mono text-slate-100 mt-2">
        {value}
      </div>
      <div className="text-[11px] text-slate-500 mt-1">{sub}</div>
    </div>
  );
}
