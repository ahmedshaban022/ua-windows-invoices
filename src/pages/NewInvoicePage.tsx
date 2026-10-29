import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Eye, Plus, Printer, Save, Trash2 } from "lucide-react";

import { InvoicePreview } from "../components/InvoicePreview";
import { ProductCombobox } from "../components/ProductCombobox";
import { Alert, Button, Card, Field, IconButton, Loading, Modal } from "../components/ui";
import * as api from "../lib/api";
import {
  formatMinor,
  formatThousandths,
  parseAmount,
  parseQuantity,
  todayIso,
} from "../lib/format";
import type {
  ComputedInvoice,
  InvoiceDocument as DocumentModel,
  InvoiceInput,
  InvoiceLineInput,
  ProductSuggestion,
} from "../lib/types";
import { useSettings } from "../state/settings";
import { toast } from "../state/toast";

interface DraftLine {
  key: string;
  productId: number | null;
  name: string;
  quantity: string;
  unitPrice: string;
}

const DRAFT_KEY = "new-invoice";

let lineCounter = 0;
function nextKey() {
  lineCounter += 1;
  return `line-${lineCounter}`;
}

function emptyLine(): DraftLine {
  return { key: nextKey(), productId: null, name: "", quantity: "1", unitPrice: "" };
}

export function NewInvoicePage() {
  const navigate = useNavigate();
  const settings = useSettings((state) => state.settings);
  const exponent = settings?.currencyExponent ?? 2;
  const currencyCode = settings?.currencyCode ?? "EGP";

  const [invoiceDate, setInvoiceDate] = useState(todayIso());
  const [customerName, setCustomerName] = useState("");
  const [customerPhone, setCustomerPhone] = useState("");
  const [notes, setNotes] = useState("");
  const [lines, setLines] = useState<DraftLine[]>([emptyLine()]);
  const [computed, setComputed] = useState<ComputedInvoice | null>(null);
  const [preview, setPreview] = useState<DocumentModel | null>(null);
  const [previewOpen, setPreviewOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const [recovered, setRecovered] = useState<unknown>(null);

  const hydrated = useRef(false);

  /* ------------------------------------------------- crash recovery (FR-08) */
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const drafts = await api.listDrafts();
        if (cancelled) return;
        const match = drafts.find((draft) => draft.key === DRAFT_KEY);
        if (match && match.data) setRecovered(match.data);
      } catch {
        /* A missing draft is not an error. */
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const applyRecovered = useCallback(() => {
    const data = recovered as {
      invoiceDate?: string;
      customerName?: string;
      customerPhone?: string;
      notes?: string;
      lines?: DraftLine[];
    } | null;
    if (!data) return;
    if (data.invoiceDate) setInvoiceDate(data.invoiceDate);
    if (data.customerName) setCustomerName(data.customerName);
    if (data.customerPhone) setCustomerPhone(data.customerPhone);
    if (data.notes) setNotes(data.notes);
    if (data.lines && data.lines.length > 0) setLines(data.lines);
    setRecovered(null);
    toast("تم استعادة المسودة غير المحفوظة", "success");
  }, [recovered]);

  const discardRecovered = useCallback(async () => {
    setRecovered(null);
    try {
      await api.deleteDraft(DRAFT_KEY);
    } catch {
      /* Ignore: the banner simply disappears. */
    }
  }, []);

  /* ------------------------------------------------------ live Rust totals */
  const lineInputs: InvoiceLineInput[] = useMemo(
    () =>
      lines.map((line) => ({
        productId: line.productId,
        name: line.name,
        quantity: line.quantity,
        unitPrice: line.unitPrice,
      })),
    [lines],
  );

  useEffect(() => {
    if (!hydrated.current) {
      hydrated.current = true;
      return;
    }
    let cancelled = false;
    const handle = window.setTimeout(async () => {
      const meaningful = lineInputs.filter((line) => line.name.trim().length > 0);
      if (meaningful.length === 0) {
        setComputed(null);
        setPreview(null);
        return;
      }
      try {
        const result = await api.previewInvoice(meaningful);
        if (!cancelled) setComputed(result);
      } catch {
        if (!cancelled) setComputed(null);
      }
    }, 200);
    return () => {
      cancelled = true;
      window.clearTimeout(handle);
    };
  }, [lineInputs]);

  /* ----------------------------------------------- autosave the working copy */
  useEffect(() => {
    const handle = window.setTimeout(() => {
      const hasContent =
        customerName.trim().length > 0 ||
        notes.trim().length > 0 ||
        lines.some((line) => line.name.trim().length > 0);
      if (!hasContent) return;
      api
        .saveDraft(DRAFT_KEY, {
          invoiceDate,
          customerName,
          customerPhone,
          notes,
          lines,
        })
        .catch(() => undefined);
    }, 900);
    return () => window.clearTimeout(handle);
  }, [invoiceDate, customerName, customerPhone, notes, lines]);

  /* ------------------------------------------------------- document preview */
  useEffect(() => {
    if (!computed) {
      setPreview(null);
      return;
    }
    let cancelled = false;
    (async () => {
      try {
        const document = await api.previewDocument({
          invoiceNumber: "مسودة",
          invoiceDate,
          customerName: customerName || "العميل",
          customerPhone: customerPhone || null,
          notes: notes || null,
          lines: lineInputs.filter((line) => line.name.trim().length > 0),
        });
        if (!cancelled) setPreview(document);
      } catch {
        if (!cancelled) setPreview(null);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [computed, invoiceDate, customerName, customerPhone, notes, lineInputs]);

  /* --------------------------------------------------------------- actions */
  const updateLine = (key: string, patch: Partial<DraftLine>) => {
    setLines((current) => current.map((line) => (line.key === key ? { ...line, ...patch } : line)));
  };

  /**
   * Apply a product selection to a line.
   *
   * The update is functional so a late callback can never re-apply a stale
   * copy of the line and undo an edit the user made in the meantime.
   */
  const applyProduct = (key: string, name: string, product: ProductSuggestion | null) => {
    setLines((current) =>
      current.map((line) => {
        if (line.key !== key) return line;
        return {
          ...line,
          name,
          productId: product?.id ?? null,
          unitPrice:
            product && line.unitPrice === ""
              ? formatMinor(product.basePriceMinor, exponent)
              : line.unitPrice,
        };
      }),
    );
  };

  const removeLine = (key: string) => {
    setLines((current) => {
      const next = current.filter((line) => line.key !== key);
      return next.length > 0 ? next : [emptyLine()];
    });
  };

  const addLine = () => setLines((current) => [...current, emptyLine()]);

  const validate = (): boolean => {
    const meaningful = lines.filter((line) => line.name.trim().length > 0);
    if (meaningful.length === 0) {
      toast("أضف بندًا واحدًا على الأقل", "error");
      return false;
    }
    for (const line of meaningful) {
      if (parseQuantity(line.quantity) === null) {
        toast("الكمية يجب أن تكون رقمًا أكبر من صفر", "error");
        return false;
      }
      if (parseAmount(line.unitPrice, exponent) === null) {
        toast(`سعر الوحدة غير صحيح للبند: ${line.name}`, "error");
        return false;
      }
    }
    if (customerName.trim().length === 0) {
      toast("اكتب اسم العميل", "error");
      return false;
    }
    return true;
  };

  const buildInput = (): InvoiceInput => ({
    invoiceDate,
    customerName: customerName.trim(),
    customerPhone: customerPhone.trim() || null,
    notes: notes.trim() || null,
    lines: lines
      .filter((line) => line.name.trim().length > 0)
      .map((line) => ({
        productId: line.productId,
        name: line.name.trim(),
        quantity: line.quantity,
        unitPrice: line.unitPrice,
      })),
  });

  const saveDraftInvoice = async () => {
    if (!validate()) return;
    setBusy(true);
    try {
      const invoice = await api.createDraft(buildInput());
      await api.deleteDraft(DRAFT_KEY).catch(() => undefined);
      toast("تم حفظ المسودة", "success");
      navigate(`/preview/${invoice.id}`);
    } catch (error) {
      toast(api.errorMessage(error), "error");
    } finally {
      setBusy(false);
    }
  };

  const issueInvoice = async () => {
    if (!validate()) return;
    setBusy(true);
    try {
      const draft = await api.createDraft(buildInput());
      const issued = await api.issueInvoice(draft.id);
      await api.deleteDraft(DRAFT_KEY).catch(() => undefined);
      toast(`تم إصدار الفاتورة رقم ${issued.displayNumber}`, "success");
      navigate(`/preview/${issued.id}`);
    } catch (error) {
      toast(api.errorMessage(error), "error");
    } finally {
      setBusy(false);
    }
  };

  const printDraft = async () => {
    if (!validate()) return;
    setBusy(true);
    try {
      const draft = await api.createDraft(buildInput());
      await api.printInvoice(draft.id);
      toast("تم فتح نافذة الطباعة", "success");
    } catch (error) {
      toast(api.errorMessage(error), "error");
    } finally {
      setBusy(false);
    }
  };

  if (!settings) return <Loading label="جارٍ تحميل الإعدادات…" />;

  const totalMinor = computed?.totalMinor ?? 0;
  const totalQuantity = formatThousandths(
    computed ? computed.lines.reduce((sum, line) => sum + line.quantityThousandths, 0) : 0,
  );

  return (
    <>
      <div className="page__header">
        <div>
          <h1 className="page__heading">فاتورة جديدة</h1>
          <p className="page__subheading">
            اكتب بيانات العميل والبنود. الإجمالي يُحسب داخل البرنامج مباشرة.
          </p>
        </div>
        <div className="page__actions">
          <Button onClick={() => setPreviewOpen(true)}>
            <Eye size={16} aria-hidden="true" /> معاينة الفاتورة
          </Button>
          <Button variant="soft" onClick={printDraft} loading={busy}>
            <Printer size={16} aria-hidden="true" /> طباعة
          </Button>
          <Button onClick={saveDraftInvoice} loading={busy}>
            <Save size={16} aria-hidden="true" /> حفظ كمسودة
          </Button>
          <Button variant="primary" onClick={issueInvoice} loading={busy}>
            إصدار الفاتورة
          </Button>
        </div>
      </div>

      {recovered ? (
        <Alert kind="warning">
          <div className="stack" style={{ gap: 6, flex: 1 }}>
            <div>يوجد مسودة غير محفوظة من جلسة سابقة. هل تريد استعادتها؟</div>
            <div className="btn-row">
              <Button size="sm" variant="primary" onClick={applyRecovered}>
                استعادة المسودة
              </Button>
              <Button size="sm" onClick={discardRecovered}>
                تجاهل
              </Button>
            </div>
          </div>
        </Alert>
      ) : null}

      <Card title="بيانات الفاتورة">
        <div className="meta-row">
          <Field label="تاريخ الفاتورة" htmlFor="invoice-date">
            <input
              id="invoice-date"
              className="input"
              type="date"
              value={invoiceDate}
              onChange={(event) => setInvoiceDate(event.target.value)}
            />
          </Field>
          <Field label="اسم العميل" htmlFor="customer-name">
            <input
              id="customer-name"
              className="input"
              value={customerName}
              placeholder="اسم العميل"
              onChange={(event) => setCustomerName(event.target.value)}
            />
          </Field>
          <Field label="رقم الهاتف" htmlFor="customer-phone">
            <input
              id="customer-phone"
              className="input ltr"
              value={customerPhone}
              placeholder="01xxxxxxxxx"
              onChange={(event) => setCustomerPhone(event.target.value)}
            />
          </Field>
        </div>
      </Card>

      <Card
        title="بنود الفاتورة"
        flush
        actions={
          <Button size="sm" variant="soft" onClick={addLine}>
            <Plus size={15} aria-hidden="true" /> بند جديد
          </Button>
        }
        footer={
          <div className="totals">
            <div className="totals__row">
              <div className="totals__stat">
                <span className="totals__label">عدد البنود</span>
                <span className="totals__value">{computed?.lines.length ?? 0}</span>
              </div>
              <div className="totals__stat">
                <span className="totals__label">الكمية الإجمالية</span>
                <span className="totals__value ltr">{totalQuantity}</span>
              </div>
              <div className="spacer" />
              <div className="totals__stat totals__stat--total">
                <span className="totals__label">الإجمالي</span>
                <span className="totals__value">
                  {formatMinor(totalMinor, exponent)} {currencyCode}
                </span>
              </div>
            </div>
            <p className="totals__note">
              يُحفظ رقم الفاتورة نهائيًا عند الإصدار فقط، ولا يُعاد استخدامه أبدًا.
            </p>
          </div>
        }
      >
        <table className="sheet">
          <colgroup>
            <col />
            <col className="sheet__col-qty" />
            <col className="sheet__col-price" />
            <col className="sheet__col-total" />
            <col className="sheet__col-actions" />
          </colgroup>
          <thead>
            <tr>
              <th scope="col">البند</th>
              <th scope="col">الكمية</th>
              <th scope="col">سعر الوحدة</th>
              <th scope="col">الإجمالي</th>
              <th scope="col">
                <span className="sr-only">إجراءات</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {lines.map((line) => {
              const quantity = parseQuantity(line.quantity);
              const price = parseAmount(line.unitPrice, exponent);
              const lineTotal =
                quantity !== null && price !== null ? Math.round((quantity * price) / 1000) : null;
              return (
                <tr key={line.key}>
                  <td className="sheet__item">
                    <ProductCombobox
                      value={line.name}
                      exponent={exponent}
                      currencyCode={currencyCode}
                      onSelect={(name, product) => applyProduct(line.key, name, product)}
                    />
                  </td>
                  <td>
                    <input
                      className="input input--numeric"
                      value={line.quantity}
                      inputMode="decimal"
                      onChange={(event) => updateLine(line.key, { quantity: event.target.value })}
                      aria-label="الكمية"
                    />
                  </td>
                  <td>
                    <input
                      className="input input--numeric"
                      value={line.unitPrice}
                      inputMode="decimal"
                      placeholder="0.00"
                      onChange={(event) => updateLine(line.key, { unitPrice: event.target.value })}
                      aria-label="سعر الوحدة"
                    />
                  </td>
                  <td className="sheet__total">
                    {lineTotal === null ? (
                      <span className="sheet__total--empty">—</span>
                    ) : (
                      `${formatMinor(lineTotal, exponent)} ${currencyCode}`
                    )}
                  </td>
                  <td>
                    <div className="table__actions">
                      <IconButton
                        label="حذف البند"
                        onClick={() => removeLine(line.key)}
                        disabled={lines.length === 1}
                      >
                        <Trash2 size={16} aria-hidden="true" />
                      </IconButton>
                    </div>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </Card>

      <Card title="ملاحظات">
        <textarea
          className="textarea textarea--note"
          value={notes}
          placeholder="ملاحظات تظهر في الفاتورة المطبوعة"
          onChange={(event) => setNotes(event.target.value)}
        />
      </Card>

      <Modal
        open={previewOpen}
        title="معاينة الفاتورة"
        onClose={() => setPreviewOpen(false)}
        size="wide"
        footer={<Button onClick={() => setPreviewOpen(false)}>إغلاق</Button>}
      >
        {preview ? (
          <InvoicePreview document={preview} />
        ) : (
          <div className="muted text-sm">أضف بنودًا لعرض معاينة الفاتورة.</div>
        )}
      </Modal>
    </>
  );
}
