import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Archive, Download, HardDriveDownload, RotateCcw, Upload } from "lucide-react";

import { Alert, Badge, Button, Card, Field, Loading } from "../components/ui";
import * as api from "../lib/api";
import { formatBytes, formatTimestamp } from "../lib/format";
import type { BackupFile } from "../lib/types";
import { useSettings } from "../state/settings";
import { toast } from "../state/toast";

const CURRENCIES = [
  { code: "EGP", label: "جنيه مصري" },
  { code: "USD", label: "دولار أمريكي" },
  { code: "EUR", label: "أورو" },
  { code: "SAR", label: "ريال سعودي" },
  { code: "AED", label: "درهم إماراتي" },
];

const DATE_FORMATS = [
  { value: "dd/MM/yyyy", label: "يوم/شهر/سنة" },
  { value: "yyyy-MM-dd", label: "سنة-شهر-يوم" },
  { value: "dd-MM-yyyy", label: "يوم-شهر-سنة" },
];

export function SettingsPage() {
  const settings = useSettings((state) => state.settings);
  const save = useSettings((state) => state.save);
  const load = useSettings((state) => state.load);

  const [companyName, setCompanyName] = useState("");
  const [companyPhone, setCompanyPhone] = useState("");
  const [companyAddress, setCompanyAddress] = useState("");
  const [brandColor, setBrandColor] = useState("#0b68e6");
  const [currencyCode, setCurrencyCode] = useState("EGP");
  const [dateFormat, setDateFormat] = useState("dd/MM/yyyy");
  const [backupLocation, setBackupLocation] = useState("");
  const [autoBackup, setAutoBackup] = useState(false);
  const [retention, setRetention] = useState(7);

  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [backups, setBackups] = useState<BackupFile[]>([]);
  const [lastBackup, setLastBackup] = useState<string | null>(null);

  useEffect(() => {
    if (!settings) return;
    setCompanyName(settings.companyName);
    setCompanyPhone(settings.companyPhone);
    setCompanyAddress(settings.companyAddress);
    setBrandColor(settings.brandColor);
    setCurrencyCode(settings.currencyCode);
    setDateFormat(settings.dateFormat);
    setBackupLocation(settings.backupLocation);
    setAutoBackup(settings.autoBackupEnabled);
    setRetention(settings.autoBackupRetention);
  }, [settings]);

  const refreshBackups = async () => {
    try {
      setBackups(await api.listBackups());
      setLastBackup(await api.lastBackup());
    } catch {
      setBackups([]);
    }
  };

  useEffect(() => {
    void refreshBackups();
  }, []);

  if (!settings) return <Loading label="جارٍ تحميل الإعدادات…" />;

  const persist = async () => {
    setSaving(true);
    setError(null);
    try {
      await save({
        ...settings,
        companyName,
        companyPhone,
        companyAddress,
        brandColor,
        currencyCode,
        currencyExponent: settings.currencyExponent,
        dateFormat,
        backupLocation,
        autoBackupEnabled: autoBackup,
        autoBackupRetention: retention,
        logoSvg: settings.logoSvg,
      });
      await api.applyBrandColor();
      toast("تم حفظ الإعدادات", "success");
    } catch (caught) {
      setError(api.errorMessage(caught));
      toast(api.errorMessage(caught), "error");
    } finally {
      setSaving(false);
    }
  };

  const pickBackupFolder = async () => {
    const folder = await open({ directory: true, multiple: false });
    if (typeof folder === "string") setBackupLocation(folder);
  };

  const importLogo = async () => {
    try {
      const picked = await open({
        multiple: false,
        filters: [{ name: "SVG", extensions: ["svg"] }],
      });
      if (typeof picked !== "string") return;
      const logo = await api.importBrandLogo(picked);
      await load();
      toast("تم تحديث الشعار", "success");
      void logo;
    } catch (caught) {
      toast(api.errorMessage(caught), "error");
    }
  };

  const resetLogo = async () => {
    try {
      await api.resetBrandLogo();
      await load();
      toast("تمت استعادة الشعار الافتراضي", "success");
    } catch (caught) {
      toast(api.errorMessage(caught), "error");
    }
  };

  const createBackup = async () => {
    try {
      const info = await api.createBackup();
      toast(`تم إنشاء النسخة الاحتياطية (${formatBytes(info.sizeBytes)})`, "success");
      void refreshBackups();
    } catch (caught) {
      toast(api.errorMessage(caught), "error");
    }
  };

  const restore = async (file: BackupFile) => {
    if (!window.confirm("سيتم استبدال كل البيانات الحالية بمحتوى هذه النسخة. هل تريد المتابعة؟")) {
      return;
    }
    try {
      await api.restoreBackup(file.filePath);
      toast("تمت الاستعادة بنجاح", "success");
      void refreshBackups();
    } catch (caught) {
      toast(api.errorMessage(caught), "error");
    }
  };

  return (
    <>
      <div className="page__header">
        <div>
          <h1 className="page__heading">الإعدادات</h1>
          <p className="page__subheading">
            كل البيانات تبقى على هذا الجهاز. لا يوجد أي خادم أو تتبّع.
          </p>
        </div>
        <div className="page__actions">
          <Button variant="primary" onClick={persist} loading={saving}>
            حفظ الإعدادات
          </Button>
        </div>
      </div>

      {error ? <Alert kind="error">{error}</Alert> : null}

      <div className="grid grid--2">
        <Card title="بيانات الشركة">
          <div className="stack">
            <Field label="اسم الشركة">
              <input
                className="input"
                value={companyName}
                onChange={(event) => setCompanyName(event.target.value)}
              />
            </Field>
            <Field label="رقم الهاتف">
              <input
                className="input ltr"
                value={companyPhone}
                onChange={(event) => setCompanyPhone(event.target.value)}
              />
            </Field>
            <Field label="العنوان">
              <input
                className="input"
                value={companyAddress}
                onChange={(event) => setCompanyAddress(event.target.value)}
              />
            </Field>
            <Field label="لون العلامة" hint="بصيغة ست عشري مثل #0b68e6">
              <div className="row">
                <input
                  className="input ltr"
                  style={{ maxWidth: 160 }}
                  value={brandColor}
                  onChange={(event) => setBrandColor(event.target.value)}
                />
                <input
                  type="color"
                  aria-label="اختيار اللون"
                  value={/^#[0-9a-f]{6}$/i.test(brandColor) ? brandColor : "#0b68e6"}
                  onChange={(event) => setBrandColor(event.target.value)}
                  style={{ width: 44, height: 36 }}
                />
              </div>
            </Field>
          </div>
        </Card>

        <Card title="الفاتورة">
          <div className="stack">
            <Field label="العملة">
              <select
                className="select"
                value={currencyCode}
                onChange={(event) => setCurrencyCode(event.target.value)}
              >
                {CURRENCIES.map((item) => (
                  <option key={item.code} value={item.code}>
                    {item.label} ({item.code})
                  </option>
                ))}
              </select>
            </Field>
            <Field label="صيغة التاريخ">
              <select
                className="select"
                value={dateFormat}
                onChange={(event) => setDateFormat(event.target.value)}
              >
                {DATE_FORMATS.map((item) => (
                  <option key={item.value} value={item.value}>
                    {item.label}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="الشعار" hint="ملف SVG فقط — يُفضّلخلو من النصوص">
              <div className="btn-row">
                <Button onClick={importLogo}>
                  <Upload size={15} aria-hidden="true" /> استيراد شعار
                </Button>
                <Button onClick={resetLogo}>
                  <RotateCcw size={15} aria-hidden="true" /> استعادة الافتراضي
                </Button>
              </div>
              <div
                className="row"
                style={{
                  marginTop: 10,
                  padding: 12,
                  border: "1px solid var(--border)",
                  borderRadius: "var(--radius)",
                  background: "var(--surface-muted)",
                }}
              >
                <div
                  aria-hidden="true"
                  style={{ height: 40, display: "flex", alignItems: "center" }}
                  dangerouslySetInnerHTML={{ __html: settings.logoSvg }}
                />
              </div>
            </Field>
          </div>
        </Card>

        <Card
          title="النسخ الاحتياطي"
          actions={
            <Button size="sm" variant="soft" onClick={createBackup}>
              <Download size={15} aria-hidden="true" /> نسخة الآن
            </Button>
          }
        >
          <div className="stack">
            <Field label="مجلد النسخ الاحتياطي" hint="اتركه فارغًا لاستخدام المجلد الافتراضي">
              <div className="btn-row">
                <input
                  className="input ltr"
                  value={backupLocation}
                  placeholder="المجلد الافتراضي"
                  onChange={(event) => setBackupLocation(event.target.value)}
                />
                <Button onClick={pickBackupFolder}>
                  <HardDriveDownload size={15} aria-hidden="true" /> اختيار
                </Button>
              </div>
            </Field>

            <label className="row" style={{ cursor: "pointer" }}>
              <input
                type="checkbox"
                checked={autoBackup}
                onChange={(event) => setAutoBackup(event.target.checked)}
              />
              <span>إنشاء نسخة احتياطية تلقائيًا عند تشغيل البرنامج</span>
            </label>

            <Field label="عدد النسخ المحفوظة">
              <input
                className="input input--numeric"
                type="number"
                min={1}
                max={100}
                value={retention}
                onChange={(event) => setRetention(Number(event.target.value) || 1)}
              />
            </Field>

            {lastBackup ? (
              <div className="muted text-sm">آخر نسخة: {formatTimestamp(lastBackup)}</div>
            ) : null}

            <hr className="divider" />

            {backups.length === 0 ? (
              <div className="muted text-sm">لا توجد نسخ محفوظة بعد.</div>
            ) : (
              <div className="table-wrap" style={{ maxHeight: 260, overflowY: "auto" }}>
                <table className="table">
                  <thead>
                    <tr>
                      <th>الملف</th>
                      <th>الحجم</th>
                      <th />
                    </tr>
                  </thead>
                  <tbody>
                    {backups.map((file) => (
                      <tr key={file.filePath}>
                        <td className="text-sm">
                          <div>{file.fileName}</div>
                          <div className="muted text-xs">{formatTimestamp(file.createdAt)}</div>
                        </td>
                        <td className="table__numeric">{formatBytes(file.sizeBytes)}</td>
                        <td>
                          <div className="table__actions">
                            <Button size="sm" onClick={() => restore(file)}>
                              <Archive size={14} aria-hidden="true" /> استعادة
                            </Button>
                          </div>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </Card>

        <Card title="عن البرنامج">
          <div className="stack">
            <div className="row row--between">
              <span className="muted">الإصدار</span>
              <Badge kind="brand">1.0.0</Badge>
            </div>
            <div className="row row--between">
              <span className="muted">التخزين</span>
              <span>SQLite محلية داخل مجلد بيانات المستخدم</span>
            </div>
            <div className="row row--between">
              <span className="muted">الاتصال بالإنترنت</span>
              <span>لا يوجد —(Update فقط)</span>
            </div>
            <div className="muted text-xs">
              عند حذف أو استعادة نسخة احتياطية، تُحفظ نسخة أمان تلقائيًا قبل التنفيذ.
            </div>
          </div>
        </Card>
      </div>
    </>
  );
}
