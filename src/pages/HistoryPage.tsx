import { useCallback, useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { Copy, Eye, Printer, Search, Trash2 } from "lucide-react";

import { Alert, Badge, Card, EmptyState, IconButton, Loading, Pagination } from "../components/ui";
import * as api from "../lib/api";
import { formatDate, formatMinor } from "../lib/format";
import type { Invoice } from "../lib/types";
import { useSettings } from "../state/settings";
import { toast } from "../state/toast";

const PAGE_SIZE = 25;

export function HistoryPage() {
  const navigate = useNavigate();
  const settings = useSettings((state) => state.settings);
  const exponent = settings?.currencyExponent ?? 2;
  const currencyCode = settings?.currencyCode ?? "EGP";

  const [query, setQuery] = useState("");
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [page, setPage] = useState(1);
  const [items, setItems] = useState<Invoice[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<number | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await api.listInvoices({
        query,
        dateFrom: dateFrom || null,
        dateTo: dateTo || null,
        page,
        pageSize: PAGE_SIZE,
      });
      setItems(result.items);
      setTotal(result.total);
    } catch (caught) {
      setError(api.errorMessage(caught));
    } finally {
      setLoading(false);
    }
  }, [query, dateFrom, dateTo, page]);

  useEffect(() => {
    const handle = window.setTimeout(() => void load(), 200);
    return () => window.clearTimeout(handle);
  }, [load]);

  const duplicate = async (invoice: Invoice) => {
    setBusyId(invoice.id);
    try {
      const draft = await api.duplicateInvoice(invoice.id);
      toast("تم إنشاء مسودة من الفاتورة", "success");
      navigate(`/preview/${draft.id}`);
    } catch (caught) {
      toast(api.errorMessage(caught), "error");
    } finally {
      setBusyId(null);
    }
  };

  const printInvoice = async (invoice: Invoice) => {
    setBusyId(invoice.id);
    try {
      await api.printInvoice(invoice.id);
    } catch (caught) {
      toast(api.errorMessage(caught), "error");
    } finally {
      setBusyId(null);
    }
  };

  const remove = async (invoice: Invoice) => {
    if (!window.confirm(`هل تريد نقل الفاتورة ${invoice.displayNumber} إلى المحذوفات؟`)) return;
    setBusyId(invoice.id);
    try {
      await api.deleteInvoice(invoice.id);
      toast("تم نقل الفاتورة إلى المحذوفات", "success");
      void load();
    } catch (caught) {
      toast(api.errorMessage(caught), "error");
    } finally {
      setBusyId(null);
    }
  };

  return (
    <>
      <div className="page__header">
        <div>
          <h1 className="page__heading">سجل الفواتير</h1>
          <p className="page__subheading">
            الفواتير الصادرة فقط. أرقام الفواتير لا تُعاد حتى بعد الحذف.
          </p>
        </div>
      </div>

      <Card>
        <div className="grid grid--3">
          <div className="field">
            <label className="field__label" htmlFor="history-search">
              بحث
            </label>
            <div style={{ position: "relative" }}>
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
                id="history-search"
                className="input"
                style={{ paddingInlineStart: 34 }}
                value={query}
                placeholder="اسم العميل أو رقم الفاتورة"
                onChange={(event) => {
                  setQuery(event.target.value);
                  setPage(1);
                }}
              />
            </div>
          </div>
          <div className="field">
            <label className="field__label" htmlFor="history-from">
              من تاريخ
            </label>
            <input
              id="history-from"
              className="input"
              type="date"
              value={dateFrom}
              onChange={(event) => {
                setDateFrom(event.target.value);
                setPage(1);
              }}
            />
          </div>
          <div className="field">
            <label className="field__label" htmlFor="history-to">
              إلى تاريخ
            </label>
            <input
              id="history-to"
              className="input"
              type="date"
              value={dateTo}
              onChange={(event) => {
                setDateTo(event.target.value);
                setPage(1);
              }}
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
            title="لا توجد فواتير"
            description="أصدر أول فاتورة من شاشة «فاتورة جديدة»."
          />
        ) : (
          <>
            <div className="table-wrap">
              <table className="table">
                <thead>
                  <tr>
                    <th>رقم الفاتورة</th>
                    <th>التاريخ</th>
                    <th>العميل</th>
                    <th>الإجمالي</th>
                    <th>الحالة</th>
                    <th />
                  </tr>
                </thead>
                <tbody>
                  {items.map((invoice) => (
                    <tr key={invoice.id}>
                      <td className="ltr">{invoice.displayNumber}</td>
                      <td className="ltr">
                        {formatDate(invoice.invoiceDate, settings?.dateFormat)}
                      </td>
                      <td>{invoice.customerName}</td>
                      <td className="table__numeric">
                        {formatMinor(invoice.totalMinor, exponent)} {currencyCode}
                      </td>
                      <td>
                        <Badge kind={invoice.status === "issued" ? "success" : "warning"}>
                          {invoice.status === "issued" ? "صادرة" : "مسودة"}
                        </Badge>
                      </td>
                      <td>
                        <div className="table__actions">
                          <IconButton
                            label="عرض"
                            onClick={() => navigate(`/preview/${invoice.id}`)}
                          >
                            <Eye size={16} aria-hidden="true" />
                          </IconButton>
                          <IconButton
                            label="طباعة"
                            onClick={() => printInvoice(invoice)}
                            disabled={busyId === invoice.id}
                          >
                            <Printer size={16} aria-hidden="true" />
                          </IconButton>
                          <IconButton
                            label="نسخ"
                            onClick={() => duplicate(invoice)}
                            disabled={busyId === invoice.id}
                          >
                            <Copy size={16} aria-hidden="true" />
                          </IconButton>
                          <IconButton
                            label="حذف"
                            onClick={() => remove(invoice)}
                            disabled={busyId === invoice.id}
                          >
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
    </>
  );
}
