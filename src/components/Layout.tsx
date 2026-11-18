import type { ReactNode } from "react";
import { NavLink, useLocation } from "react-router-dom";
import {
  LayoutDashboard,
  FolderKanban,
  Languages,
  BookText,
  PackageCheck,
  Cpu,
  Plus,
} from "lucide-react";

const NAV_ITEMS = [
  { to: "/", label: "工作台", icon: LayoutDashboard, exact: true },
  { to: "/projects", label: "项目管理", icon: FolderKanban },
  { to: "/translate", label: "翻译工作台", icon: Languages },
  { to: "/glossary", label: "术语与记忆", icon: BookText },
  { to: "/package", label: "生成 .exe 补丁", icon: PackageCheck },
  { to: "/settings", label: "AI 模型配置", icon: Cpu },
];

export function Layout({ children }: { children: ReactNode }) {
  const { pathname } = useLocation();

  return (
    <div className="min-h-screen flex bg-[#0A0E14] text-slate-100 select-none">
      {/* 左侧导航栏 */}
      <aside className="w-56 shrink-0 border-r border-slate-800/80 bg-[#0E131B] flex flex-col">
        <div className="px-4 py-4 border-b border-slate-800/80 flex items-center gap-2.5">
          <div className="w-8 h-8 rounded-lg bg-cyan-500/15 border border-cyan-400/40 flex items-center justify-center text-cyan-300 font-bold text-sm">
            铸
          </div>
          <div>
            <div className="font-semibold text-sm tracking-wide text-slate-100">
              译铸 LexiPatch
            </div>
            <div className="text-[11px] text-cyan-400/80 font-mono">
              v2.0 · Rust + Tauri
            </div>
          </div>
        </div>

        <div className="p-3">
          <NavLink
            to="/projects/new"
            className="w-full flex items-center justify-center gap-1.5 px-3 py-2 rounded-md bg-cyan-500 hover:bg-cyan-400 text-slate-950 font-medium text-xs transition"
          >
            <Plus className="w-3.5 h-3.5" />
            新建汉化项目
          </NavLink>
        </div>

        <nav className="flex-1 px-2.5 space-y-1">
          {NAV_ITEMS.map((item) => {
            const active = item.exact
              ? pathname === item.to
              : pathname === item.to || pathname.startsWith(item.to + "/");
            const Icon = item.icon;
            return (
              <NavLink
                key={item.to}
                to={item.to}
                className={`flex items-center gap-2.5 px-3 py-2 rounded-md text-xs font-medium transition ${
                  active
                    ? "bg-cyan-500/15 text-cyan-300 border border-cyan-500/30"
                    : "text-slate-400 hover:text-slate-200 hover:bg-slate-800/60 border border-transparent"
                }`}
              >
                <Icon className="w-4 h-4 shrink-0" />
                <span>{item.label}</span>
              </NavLink>
            );
          })}
        </nav>

        <div className="p-3 border-t border-slate-800/80 text-[11px] text-slate-500 space-y-1">
          <div className="flex items-center justify-between">
            <span>存储引擎</span>
            <span className="text-emerald-400 font-mono">SQLite WAL</span>
          </div>
          <div className="flex items-center justify-between">
            <span>补丁内核</span>
            <span className="text-cyan-400 font-mono">SFX Win32</span>
          </div>
        </div>
      </aside>

      {/* 主内容区 */}
      <main className="flex-1 min-w-0 flex flex-col overflow-auto">
        {children}
      </main>
    </div>
  );
}
