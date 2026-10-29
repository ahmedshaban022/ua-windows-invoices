import { useCallback, useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { open, save } from "@tauri-apps/plugin-dialog";
import { ArrowRight, FileImage, FileText, Printer } from "lucide-react";

import { InvoicePreview } from "../components/InvoicePreview";
import { Alert, Badge, Button, Card, Loading } from "../components/ui";
import * as api from "../lib/api";
import { formatDate, formatMinor } from "../lib/format";
import type { Invoice, InvoiceDocument as DocumentModel } from "../lib/types";
import { useSettings } from "../state/settings";
import { toast } from "../state/toast";

export function PreviewPage() {
  const { id } = useParams();
  const navigate = useNavigate();
  const settings = useSettings((state) => state.settings);
  const exponent = settings?.currencyExponent ?? 2;
  const currencyCode = settings?.currencyCode ?? "EGP";

  const [invoice, setInvoice] = useState<Invoice | null>(null);
  const [document, setDocument] = useState<DocumentModel | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    if (!id) return;
    setError(null);
    try {
      const invoiceId = Number(id);
      const found = await api.getInvoice(invoiceId);
      if (!found) {
        setError("الفاتورة غير موجودة");
        return;
      }
      setInvoice(found);
      setDocument(await api.invoiceDocument(invoiceId));
    } catch (caught) {
      setError(api.errorMessage(caught));
    }
  }, [id]);

  useEffect(() => {
    void load();
  }, [load]);

  const exportPdf = async () => {
    if (!invoice) return;
    setBusy(true);
    try {
      const target = await save({
        defaultPath: `${invoice.displayNumber}.pdf`,
        filters: [{ name: "PDF", extensions: ["pdf"] }],
      });
      if (!target) return;
      await api.exportInvoicePdf(invoice.id, target);
      toast("تم إنشاء ملف PDF", "success");
      await api.openContainingFolder(target).catch(() => undefined);
    } catch (caught) {
      toast(api.errorMessage(caught), "error");
    } finally {
      setBusy(false);
    }
  };

  const exportImages = async () => {
    if (!invoice) return;
    setBusy(true);
    try {
      const folder = await open({ directory: true, multiple: false, title: "اختر مجلد الحفظ" });
      if (!folder || typeof folder !== "string") return;
      const result = await api.exportInvoiceImages(invoice.id, folder, "png", 2);
      toast(`تم حفظ ${result.pageCount} صفحة`, "success");
      const first = result.files[0];
      if (first) await api.openContainingFolder(first).catch(() => undefined);
    } catch (caught) {
      toast(api.errorMessage(caught), "error");
    } finally {
      setBusy(false);
    }
  };

  const print = async () => {
    if (!invoice) return;
    setBusy(true);
    try {
      await api.printInvoice(invoice.id);
    } catch (caught) {
      toast(api.errorMessage(caught), "error");
    } finally {
      setBusy(false);
    }
  };

  if (error) {
    return (
      <Alert kind="error">
        <div className="stack" style={{ gap: 8 }}>
          <div>{error}</div>
          <Button onClick={() => navigate("/history")}>العودة إلى السجل</Button>
        </div>
      </Alert>
    );
  }

  if (!invoice || !document) return <Loading label="جارٍ تحميل الفاتورة…" />;

  return (
    <>
      <div className="page__header">
        <div>
          <h1 className="page__heading">
            <span className="ltr">{invoice.displayNumber}</span>{" "}
            {invoice.status === "issued" ? (
              <Badge kind="success">صادرة</Badge>
            ) : (
              <Badge kind="warning">مسودة</Badge>
            )}
          </h1>
          <p className="page__subheading">
            {invoice.customerName} — {formatDate(invoice.invoiceDate, settings?.dateFormat)} —{" "}
            {formatMinor(invoice.totalMinor, exponent)} {currencyCode}
          </p>
        </div>
        <div className="page__actions">
          <Button onClick={() => navigate("/history")}>
            <ArrowRight size={16} aria-hidden="true" /> رجوع
          </Button>
          <Button variant="soft" onClick={print} loading={busy}>
            <Printer size={16} aria-hidden="true" /> طباعة
          </Button>
          <Button variant="soft" onClick={exportImages} loading={busy}>
            <FileImage size={16} aria-hidden="true" /> صور
          </Button>
          <Button variant="primary" onClick={exportPdf} loading={busy}>
            <FileText size={16} aria-hidden="true" /> تصدير PDF
          </Button>
        </div>
      </div>

      <Card>
        <InvoicePreview document={document} />
      </Card>
    </>
  );
}
