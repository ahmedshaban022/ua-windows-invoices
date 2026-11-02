import { useCallback, useEffect, useMemo, useState } from "react";
import { open as openDialog, save } from "@tauri-apps/plugin-dialog";
import { Download, FileSpreadsheet, Plus, Search, Trash2, Upload } from "lucide-react";
import {
  Alert,
  Badge,
  Button,
  Card,
  EmptyState,
  Field,
  IconButton,
  Loading,
  Modal,
  Pagination,
  Select,
} from "../components/ui";
import * as api from "../lib/api";
import { formatMinor } from "../lib/format";
import type {
  ColumnMapping,
  ImportPreview,
  MatchStrategy,
  Product,
  SheetPreview,
} from "../lib/types";
import { useSettings } from "../state/settings";
import { toast } from "../state/toast";

const PAGE_SIZE = 25;

export function ProductsPage() {
  const settings = useSettings((state) => state.settings);
  const exponent = settings?.currencyExponent ?? 2;
  const currencyCode = settings?.currencyCode ?? "EGP";

  const [query, setQuery] = useState("");
  const [page, setPage] = useState(1);
  const [items, setItems] = useState<Product[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [editing, setEditing] = useState<Product | null>(null);
  const [creating, setCreating] = useState(false);
  const [form, setForm] = useState({ name: "", price: "", sku: "" });
  const [saving, setSaving] = useState(false);

  const [importOpen, setImportOpen] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await api.listProducts(query, page, PAGE_SIZE);
      setItems(result.items);
      setTotal(result.total);
    } catch (caught) {
      setError(api.errorMessage(caught));
    } finally {
      setLoading(false);
    }
  }, [query, page]);

  useEffect(() => {
    const handle = window.setTimeout(() => void load(), 180);
    return () => window.clearTimeout(handle);
  }, [load]);

  const openCreate = () => {
    setForm({ name: "", price: "", sku: "" });
    setEditing(null);
    setCreating(true);
  };

  const openEdit = (product: Product) => {
    setForm({
      name: product.name,
      price: formatMinor(product.basePriceMinor, exponent),
      sku: product.sku ?? "",
    });
    setEditing(product);
    setCreating(false);
  };

  const submit = async () => {
    setSaving(true);
    try {
      const parsed = parsePriceToMinor(form.price, exponent);
      if (parsed === null) {
        toast("السعر غير صحيح", "error");
        return;
      }
      if (form.name.trim().length === 0) {
        toast("اكتب اسم الصنف", "error");
        return;
      }
      const input = {
        name: form.name.trim(),
        basePriceMinor: parsed,
        sku: form.sku.trim() || null,
      };
      if (editing) {
        await api.updateProduct(editing.id, input);
        toast("تم تحديث الصنف", "success");
      } else {
        await api.createProduct(input);
        toast("تمت إضافة الصنف", "success");
      }
      setCreating(false);
      setEditing(null);
      void load();
    } catch (caught) {
      toast(api.errorMessage(caught), "error");
    } finally {
      setSaving(false);
    }
  };

  const remove = async (product: Product) => {
    if (!window.confirm(`هل تريد حذف الصنف «${product.name}»؟`)) return;
    try {
      await api.deleteProduct(product.id);
      toast("تم نقل الصنف إلى المحذوفات", "success");
      void load();
    } catch (caught) {
      toast(api.errorMessage(caught), "error");
    }
  };

  const exportExcel = async () => {
    try {
      const target = await save({
        defaultPath: "الاصناف.xlsx",
        filters: [{ name: "Excel", extensions: ["xlsx"] }],
      });
      if (!target) return;
      const count = await api.excelExport(target, query);
      toast(`تم تصدير ${count} صنف`, "success");
    } catch (caught) {
      toast(api.errorMessage(caught), "error");
    }
  };

  const writeTemplate = async () => {
    try {
      const target = await save({
        defaultPath: "قالب-الاصناف.xlsx",
        filters: [{ name: "Excel", extensions: ["xlsx"] }],
      });
      if (!target) return;
      await api.excelWriteTemplate(target);
      toast("تم إنشاء قالب الاستيراد", "success");
    } catch (caught) {
      toast(api.errorMessage(caught), "error");
    }
  };

  return (
    <>
      <div className="page__header">
        <div>
          <h1 className="page__heading">الأصناف</h1>
          <p className="page__subheading">البحث يعمل فورًا ويقبل الهمزات والتشكيل المختلف.</p>
        </div>
        <div className="page__actions">
          <Button onClick={openCreate}>
            <Plus size={16} aria-hidden="true" /> صنف جديد
          </Button>
          <Button variant="soft" onClick={exportExcel}>
            <Download size={16} aria-hidden="true" /> تصدير Excel
          </Button>
          <Button variant="soft" onClick={() => setImportOpen(true)}>
            <Upload size={16} aria-hidden="true" /> استيراد Excel
          </Button>
        </div>
      </div>

      <Card>
        <div className="row">
          <div style={{ position: "relative", flex: 1, minWidth: 240 }}>
            <Search
              size={16}
              aria-hidden="true"
              style={{
                position: "absolute",
                insetInlineStart: 10,
                top: "50%",
                transform: "translateY(-50%)",
                color: "var(--text-muted)",
                pointerEvents: "none",
              }}
            />
            <input
              className="input"
              style={{ paddingInlineStart: 34 }}
              value={query}
              placeholder="ابحث بالاسم أو الرمز"
              onChange={(event) => {
                setQuery(event.target.value);
                setPage(1);
              }}
              aria-label="بحث الأصناف"
            />
          </div>
        </div>
      </Card>

      {error ? <Alert kind="error">{error}</Alert> : null}

      <Card flush>
        {loading ? (
          <Loading />
        ) : items.length === 0 ? (
          <EmptyState
            title="لا توجد أصناف"
            description="ابدأ بإضافة صنف أو استورد قائمة أصناف من ملف Excel."
            action={
              <Button variant="primary" onClick={openCreate}>
                إضافة صنف
              </Button>
            }
          />
        ) : (
          <>
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr>
                    <th>اسم الصنف</th>
                    <th>الرمز (SKU)</th>
                    <th>السعر</th>
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {items.map((product) => (
                    <tr key={product.id}>
                      <td>
                        <button
                          className="btn btn--ghost"
                          style={{ padding: 0 }}
                          onClick={() => openEdit(product)}
                        >
                          {product.name}
                        </button>
                      </td>
                      <td className="ltr">{product.sku ?? "—"}</td>
                      <td className="table__numeric">
                        {formatMinor(product.basePriceMinor, exponent)} {currencyCode}
                      </td>
                      <td>
                        <div className="table__actions">
                          <IconButton label="تعديل" onClick={() => openEdit(product)}>
                            ✎
                          </IconButton>
                          <IconButton label="حذف" onClick={() => remove(product)}>
                            <Trash2 size={16} aria-hidden="true" />
                          </IconButton>
                        </div>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <Pagination page={page} pageSize={PAGE_SIZE} total={total} onPage={setPage} />
          </>
        )}
      </Card>

      <Modal
        open={creating || editing !== null}
        title={editing ? "تعديل الصنف" : "صنف جديد"}
        onClose={() => {
          setCreating(false);
          setEditing(null);
        }}
        size="narrow"
        footer={
          <>
            <Button variant="primary" onClick={submit} loading={saving}>
              حفظ
            </Button>
            <Button
              onClick={() => {
                setCreating(false);
                setEditing(null);
              }}
            >
              إلغاء
            </Button>
          </>
        }
      >
        <Field label="اسم الصنف" htmlFor="product-name">
          <input
            id="product-name"
            className="input"
            value={form.name}
            onChange={(event) => setForm({ ...form, name: event.target.value })}
            autoFocus
          />
        </Field>
        <Field label="السعر" hint={`بـ ${currencyCode}`} htmlFor="product-price">
          <input
            id="product-price"
            className="input input--numeric"
            value={form.price}
            inputMode="decimal"
            placeholder="0.00"
            onChange={(event) => setForm({ ...form, price: event.target.value })}
          />
        </Field>
        <Field
          label="الرمز (SKU)"
          hint="اختياري — يتيح المطابقة الدقيقة عند الاستيراد"
          htmlFor="product-sku"
        >
          <input
            id="product-sku"
            className="input ltr"
            value={form.sku}
            onChange={(event) => setForm({ ...form, sku: event.target.value })}
          />
        </Field>
      </Modal>

      <ExcelImportModal
        open={importOpen}
        onClose={() => setImportOpen(false)}
        exponent={exponent}
        onDone={() => void load()}
        onWriteTemplate={writeTemplate}
      />
    </>
  );
}

/** Parse a typed price into minor units (mirrors the Rust rounding rules). */
function parsePriceToMinor(text: string, exponent: number): number | null {
  const cleaned = text.trim().replace(/[,\s]/g, "").replace(/٫/g, ".");
  if (cleaned.length === 0) return null;
  if (!/^\d*\.?\d*$/.test(cleaned)) return null;
  const [intPart = "0", fracPart = ""] = cleaned.split(".");
  const intValue = Number(intPart || "0");
  if (!Number.isFinite(intValue)) return null;
  const kept = fracPart.slice(0, exponent).padEnd(exponent, "0");
  let minor = intValue * 10 ** exponent + Number(kept || "0");
  if (fracPart.length > exponent && Number(fracPart[exponent]) >= 5) minor += 1;
  return minor;
}

/* --------------------------------------------------------------- import UI */

function ExcelImportModal({
  open,
  onClose,
  exponent,
  onDone,
  onWriteTemplate,
}: {
  open: boolean;
  onClose: () => void;
  exponent: number;
  onDone: () => void;
  onWriteTemplate: () => void;
}) {
  const [path, setPath] = useState<string | null>(null);
  const [sheet, setSheet] = useState<SheetPreview | null>(null);
  const [sheetName, setSheetName] = useState<string | null>(null);
  const [mapping, setMapping] = useState<ColumnMapping | null>(null);
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [strategy, setStrategy] = useState<MatchStrategy>("skipMatched");
  const [confirmed, setConfirmed] = useState<number[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) {
      setPath(null);
      setSheet(null);
      setSheetName(null);
      setMapping(null);
      setPreview(null);
      setConfirmed([]);
      setError(null);
    }
  }, [open]);

  const choose = async () => {
    try {
      const picked = await openDialog({
        multiple: false,
        filters: [{ name: "Excel", extensions: ["xlsx", "xls", "ods"] }],
      });
      if (typeof picked !== "string") return;
      setPath(picked);
      const inspected = await api.excelInspect(picked);
      setSheet(inspected);
      setMapping(inspected.suggestedMapping);
      setSheetName(inspected.sheets[0]?.name ?? null);
      setPreview(null);
    } catch (caught) {
      setError(api.errorMessage(caught));
    }
  };

  const runDryRun = async () => {
    if (!path || !mapping) return;
    setBusy(true);
    setError(null);
    try {
      const result = await api.excelDryRun(path, sheetName, mapping);
      setPreview(result);
      setConfirmed([]);
    } catch (caught) {
      setError(api.errorMessage(caught));
    } finally {
      setBusy(false);
    }
  };

  const apply = async () => {
    if (!path || !mapping) return;
    setBusy(true);
    setError(null);
    try {
      const result = await api.excelApply(path, sheetName, mapping, strategy, confirmed);
      toast(
        `تمت الإضافة: ${result.imported} — التحديث: ${result.updated} — المتجاوز: ${result.skipped}`,
        "success",
      );
      onDone();
      onClose();
    } catch (caught) {
      setError(api.errorMessage(caught));
    } finally {
      setBusy(false);
    }
  };

  const columnOptions = useMemo(() => sheet?.headers ?? [], [sheet]);

  return (
    <Modal
      open={open}
      title="استيراد الأصناف من Excel"
      onClose={onClose}
      size="wide"
      footer={
        <>
          <Button variant="primary" onClick={apply} disabled={!preview} loading={busy}>
            تنفيذ الاستيراد
          </Button>
          <Button onClick={runDryRun} disabled={!path || !mapping} loading={busy}>
            معاينة النتائج
          </Button>
          <Button onClick={onWriteTemplate}>تنزيل قالب</Button>
          <Button onClick={onClose}>إغلاق</Button>
        </>
      }
    >
      {error ? <Alert kind="error">{error}</Alert> : null}

      <div className="row">
        <Button onClick={choose}>
          <FileSpreadsheet size={16} aria-hidden="true" /> اختيار ملف
        </Button>
        {path ? <span className="text-sm muted ltr">{path}</span> : null}
      </div>

      {sheet ? (
        <>
          <div className="grid grid--3">
            <Field label="ورقة العمل" htmlFor="import-sheet">
              <Select
                id="import-sheet"
                value={sheetName ?? ""}
                onChange={(value) => {
                  setSheetName(value);
                  setPreview(null);
                }}
                options={sheet.sheets.map((item) => ({
                  value: item.name,
                  label: `${item.name} (${item.rowCount})`,
                }))}
              />
            </Field>
            <Field label="عمود اسم الصنف" htmlFor="import-name-column">
              <Select
                id="import-name-column"
                value={mapping?.name === undefined ? "" : String(mapping.name)}
                onChange={(value) =>
                  setMapping({
                    name: Number(value),
                    price: mapping?.price ?? 0,
                    sku: mapping?.sku ?? null,
                  })
                }
                options={columnOptions.map((header, index) => ({
                  value: String(index),
                  label: header || `عمود ${index + 1}`,
                }))}
              />
            </Field>
            <Field label="عمود السعر" htmlFor="import-price-column">
              <Select
                id="import-price-column"
                value={mapping?.price === undefined ? "" : String(mapping.price)}
                onChange={(value) =>
                  setMapping({
                    name: mapping?.name ?? 0,
                    price: Number(value),
                    sku: mapping?.sku ?? null,
                  })
                }
                options={columnOptions.map((header, index) => ({
                  value: String(index),
                  label: header || `عمود ${index + 1}`,
                }))}
              />
            </Field>
          </div>

          <Field
            label="عمود الرمز (SKU)"
            hint="اتركه فارغًا إذا لم يوجد عمود للرمز"
            htmlFor="import-sku-column"
          >
            <Select
              id="import-sku-column"
              value={mapping?.sku === null || mapping?.sku === undefined ? "" : String(mapping.sku)}
              onChange={(value) =>
                setMapping({
                  name: mapping?.name ?? 0,
                  price: mapping?.price ?? 0,
                  sku: value === "" ? null : Number(value),
                })
              }
              options={[
                { value: "", label: "بدون رمز" },
                ...columnOptions.map((header, index) => ({
                  value: String(index),
                  label: header || `عمود ${index + 1}`,
                })),
              ]}
            />
          </Field>
        </>
      ) : null}

      {preview ? (
        <>
          <div className="row">
            <Badge kind="success">جديد: {preview.validNew}</Badge>
            <Badge kind="brand">مطابق: {preview.matches}</Badge>
            <Badge kind="warning">مكرر: {preview.duplicates}</Badge>
            <Badge kind="danger">غير صالح: {preview.invalid}</Badge>
          </div>

          <Field label="كيف نتعامل مع الصفقات المطابقة؟" htmlFor="import-strategy">
            <Select
              id="import-strategy"
              value={strategy}
              onChange={(value) => setStrategy(value as MatchStrategy)}
              options={[
                { value: "skipMatched", label: "تجاهلها (لا تعديل)" },
                { value: "updateConfirmed", label: "تحديث المطابق بالرمز فقط" },
                { value: "addNewOnly", label: "إضافة الجديد فقط" },
              ]}
            />
          </Field>

          <div className="table-wrap" style={{ maxHeight: 320, overflowY: "auto" }}>
            <table className="table">
              <thead>
                <tr>
                  <th>الصف</th>
                  <th>الاسم</th>
                  <th>السعر</th>
                  <th>الحالة</th>
                  <th>تأكيد</th>
                </tr>
              </thead>
              <tbody>
                {preview.rows.map((row) => (
                  <tr key={row.rowNumber}>
                    <td className="ltr">{row.rowNumber}</td>
                    <td>{row.name || "—"}</td>
                    <td className="ltr">{row.price || "—"}</td>
                    <td>
                      <span className="text-sm">{row.reason}</span>
                    </td>
                    <td>
                      {row.classification === "name_match" ? (
                        <input
                          type="checkbox"
                          aria-label={`تأكيد تحديث ${row.name}`}
                          checked={confirmed.includes(row.rowNumber)}
                          onChange={(event) =>
                            setConfirmed((current) =>
                              event.target.checked
                                ? [...current, row.rowNumber]
                                : current.filter((value) => value !== row.rowNumber),
                            )
                          }
                        />
                      ) : null}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="muted text-xs">
            الأسعار تُقرأ بــ{exponent === 2 ? "قريش" : "وحدة فرعية"} — التحويل يتم داخل التطبيق.
          </div>
        </>
      ) : null}
    </Modal>
  );
}
