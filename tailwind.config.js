/** @type {import('tailwindcss').Config} */

export default {
  darkMode: "class",
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    container: {
      center: true,
    },
    extend: {
      colors: {
        base: "var(--bg-base)",
        panel: "var(--bg-panel)",
        "panel-2": "var(--bg-panel-2)",
        elevated: "var(--bg-elevated)",
        ink: {
          DEFAULT: "var(--text-primary)",
          soft: "var(--text-secondary)",
          muted: "var(--text-muted)",
        },
        cyan: {
          DEFAULT: "var(--accent-cyan)",
          dim: "var(--accent-cyan-dim)",
        },
        amber: {
          DEFAULT: "var(--accent-amber)",
          dim: "var(--accent-amber-dim)",
        },
        line: {
          DEFAULT: "var(--border)",
          soft: "var(--border-soft)",
          strong: "var(--border-strong)",
        },
      },
      fontFamily: {
        display: ['"Space Grotesk"', '"PingFang SC"', "sans-serif"],
        sans: ['"Sora"', '"PingFang SC"', '"Microsoft YaHei"', "sans-serif"],
        mono: ['"JetBrains Mono"', '"Cascadia Code"', "monospace"],
      },
    },
  },
  plugins: [],
};
