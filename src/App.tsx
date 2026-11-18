import { HashRouter as Router, Routes, Route, Navigate } from "react-router-dom";
import { Layout } from "@/components/Layout";
import Dashboard from "@/pages/Dashboard";
import Projects from "@/pages/Projects";
import NewProject from "@/pages/NewProject";
import Translate from "@/pages/Translate";
import Glossary from "@/pages/Glossary";
import PackagePage from "@/pages/PackagePage";
import Settings from "@/pages/Settings";

export default function App() {
  return (
    <Router>
      <Layout>
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/projects" element={<Projects />} />
          <Route path="/projects/new" element={<NewProject />} />
          <Route path="/translate" element={<Translate />} />
          <Route path="/translate/:projectId" element={<Translate />} />
          <Route path="/glossary" element={<Glossary />} />
          <Route path="/glossary/:projectId" element={<Glossary />} />
          <Route path="/package" element={<PackagePage />} />
          <Route path="/package/:projectId" element={<PackagePage />} />
          <Route path="/settings" element={<Settings />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </Layout>
    </Router>
  );
}
