/**
 * The render host entry point.
 *
 * This window is hidden from the user. The Rust side pushes a canonical
 * invoice document into it with `ExecuteScript`, then reads it back for PDF,
 * print and image export. Because it renders the exact same
 * `InvoiceDocument` component as the on-screen preview, the four outputs can
 * never disagree.
 */
import { StrictMode, useCallback, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";

import { InvoiceDocument } from "./components/InvoiceDocument";
import type { InvoiceDocument as DocumentModel } from "./lib/types";
import "./styles/global.css";
import "./styles/invoice.css";

declare global {
  interface Window {
    __renderInvoice?: (document: DocumentModel) => void;
    __renderReset?: () => void;
    __renderReady?: boolean;
  }
}

function RenderHost() {
  const [document, setDocument] = useState<DocumentModel | null>(null);

  const markReady = useCallback(() => {
    window.__renderReady = true;
  }, []);

  useEffect(() => {
    window.__renderReady = false;
    window.__renderReset = () => {
      window.__renderReady = false;
      setDocument(null);
    };
    window.__renderInvoice = (next: DocumentModel) => {
      window.__renderReady = false;
      setDocument(next);
    };
  }, []);

  return (
    <div style={{ background: "#ffffff", minHeight: "100vh" }}>
      {document ? <InvoiceDocument document={document} onReady={markReady} /> : null}
    </div>
  );
}

createRoot(document.getElementById("render-root") as HTMLElement).render(
  <StrictMode>
    <RenderHost />
  </StrictMode>,
);
