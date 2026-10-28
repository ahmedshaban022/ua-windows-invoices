import { useCallback, useEffect, useState } from "react";
import { RotateCcw, Trash2 } from "lucide-react";

import { Alert, Button, Card, EmptyState, IconButton, Loading } from "../components/ui";
import * as api from "../lib/api";
import { formatDate, formatMinor } from "../lib/format";
import type { Invoice } from "../lib/types";
import { useSettings } from "../state/settings";
import { toast } from "../state/toast";

export function TrashPage() {
  const settings = useSettings((state) => state.settings);
  const exponent = settings?.currencyExponent ?? 2;
  const currencyCode = settings?.currencyCode ?? "EGP";

  const [items, setItems] = useState<Invoice[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<number | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setItems(await api.listTrashedInvoices());
    } catch (caught) {
      setError(api.errorMessage(caught));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const restore = async (invoice: Invoice) => {
    setBusyId(invoice.id);
    try {
      await api.restoreInvoice(invoice.id);
      toast("تمت استعادة الفاتورة", "success");
      void load();
    } catch (caught) {
      toast(api.errorMessage(caught), "error");
    } finally {
      setBusyId(null);
    }
  };

  const purge = async (invoice: Invoice) => {
    if (
      !window.confirm(
        `الحذف النهائي لا يمكن التراجع عنه. هل تريد حذف الفاتورة ${invoice.displayNumber} نهائيًا؟`,
      )
    ) {
      return;
    }
    setBusyId(invoice.id);
    try {
      await api.purgeInvoice(invoice.id);
      toast("تم الحذف النهائي", "success");
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
          <h1 className="page__heading">المحذوفات</h1>
          <p className="page__subheading">
            الفواتير المحذوفة يمكن استعادتها. أرقام الفواتير لا تُعاد حتى بعد الحذف النهائي.
          </p>
        </div>
      </div>

      {error ? <Alert kind="error">{error}</Alert> : null}

      <Card flush>
        {loading ? (
          <Loading />
        ) : items.length === 0 ? (
          <EmptyState title="المحذوفات فارغة" description="لا توجد فواتير محذوفة." />
        ) : (
          <div className="table-wrap">
            <table className="table">
              <thead>
                <tr>
                  <th>رقم الفاتورة</th>
                  <th>التاريخ</th>
                  <th>العميل</th>
                  <th>الإجمالي</th>
                  <th />
                </tr>
              </thead>
              <tbody>
                {items.map((invoice) => (
                  <tr key={invoice.id}>
                    <td className="ltr">{invoice.displayNumber}</td>
                    <td className="ltr">{formatDate(invoice.invoiceDate, settings?.dateFormat)}</td>
                    <td>{invoice.customerName}</td>
                    <td className="table__numeric">
                      {formatMinor(invoice.totalMinor, exponent)} {currencyCode}
                    </td>
                    <td>
                      <div className="table__actions">
                        <IconButton
                          label="استعادة"
                          onClick={() => restore(invoice)}
                          disabled={busyId === invoice.id}
                        >
                          <RotateCcw size={16} aria-hidden="true" />
                        </IconButton>
                        <IconButton
                          label="حذف نهائي"
                          onClick={() => purge(invoice)}
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
        )}
      </Card>

      <div className="row">
        <Button onClick={() => void load()}>تحديث</Button>
      </div>
    </>
  );
}
