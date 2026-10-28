import { useEffect } from "react";
import { HashRouter, Navigate, Route, Routes } from "react-router-dom";

import { AppShell } from "./components/AppShell";
import { ToastStack } from "./components/ui";
import { UpdateBanner } from "./components/UpdateBanner";
import { HistoryPage } from "./pages/HistoryPage";
import { NewInvoicePage } from "./pages/NewInvoicePage";
import { PreviewPage } from "./pages/PreviewPage";
import { ProductsPage } from "./pages/ProductsPage";
import { SettingsPage } from "./pages/SettingsPage";
import { TrashPage } from "./pages/TrashPage";
import { useSettings } from "./state/settings";

export function App() {
  const load = useSettings((state) => state.load);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <HashRouter>
      <AppShell>
        <Routes>
          <Route path="/" element={<NewInvoicePage />} />
          <Route path="/history" element={<HistoryPage />} />
          <Route path="/products" element={<ProductsPage />} />
          <Route path="/trash" element={<TrashPage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route path="/preview/:id" element={<PreviewPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </AppShell>
      <UpdateBanner />
      <ToastStack />
    </HashRouter>
  );
}
