/**
 * Typed wrappers around the Rust command layer.
 *
 * Tauri converts command names from `snake_case` to `camelCase` for the
 * JavaScript side, so `list_products` becomes `list_products` invoked as
 * `list_products` — the plugin keeps the original name. Arguments and return
 * values are camelCase.
 */
import { invoke as tauriInvoke } from "@tauri-apps/api/core";

import { mockInvoke } from "./mock";
import type {
  AppSettings,
  BackupFile,
  BackupInfo,
  BackupValidation,
  BrandLogo,
  ComputedInvoice,
  ImageExportResult,
  ImageFormat,
  ImportPreview,
  ImportResult,
  Invoice,
  InvoiceDocument,
  InvoiceFilter,
  InvoiceInput,
  InvoicePage,
  MatchStrategy,
  Product,
  ProductInput,
  ProductPage,
  ProductSuggestion,
  SheetPreview,
  ColumnMapping,
  StoredDraft,
  UpdateStatus,
} from "./types";

/**
 * The command transport.
 *
 * Inside Tauri the real Rust commands are used. In a plain browser — which is
 * how the Playwright end-to-end suite drives the UI — an in-memory mock with
 * the same semantics stands in.
 */
const inTauri =
  typeof window !== "undefined" && ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);

const invoke = (inTauri ? tauriInvoke : mockInvoke) as <T>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T>;

/** The error shape produced by the Rust `AppError` enum. */
export interface AppErrorPayload {
  kind?: string;
  message?: string;
}

/** Extract a readable Arabic message from a rejected command. */
export function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error && typeof error === "object") {
    const payload = error as AppErrorPayload;
    if (typeof payload.message === "string" && payload.message.length > 0) {
      return payload.message;
    }
    if (typeof payload.kind === "string") return payload.kind;
  }
  return "حدث خطأ غير متوقع";
}

/* ------------------------------------------------------------------ products */

export const listProducts = (query: string, page = 1, pageSize = 25, includeDeleted = false) =>
  invoke<ProductPage>("list_products", {
    query,
    page,
    pageSize,
    includeDeleted,
  });

export const getProduct = (id: number) => invoke<Product | null>("get_product", { id });

export const createProduct = (input: ProductInput) => invoke<Product>("create_product", { input });

export const updateProduct = (id: number, input: ProductInput) =>
  invoke<Product>("update_product", { id, input });

export const deleteProduct = (id: number) => invoke<void>("delete_product", { id });

export const restoreProduct = (id: number) => invoke<void>("restore_product", { id });

export const suggestProducts = (query: string, limit = 10) =>
  invoke<ProductSuggestion[]>("suggest_products", { query, limit });

export const countProducts = () => invoke<number>("count_products");

/* ------------------------------------------------------------------ invoices */

export const listInvoices = (filter: InvoiceFilter) =>
  invoke<InvoicePage>("list_invoices", { filter });

export const getInvoice = (id: number) => invoke<Invoice | null>("get_invoice", { id });

export const createDraft = (input: InvoiceInput) => invoke<Invoice>("create_draft", { input });

export const updateDraft = (id: number, input: InvoiceInput) =>
  invoke<Invoice>("update_draft", { id, input });

export const issueInvoice = (id: number) => invoke<Invoice>("issue_invoice", { id });

export const previewInvoice = (lines: InvoiceInput["lines"]) =>
  invoke<ComputedInvoice>("preview_invoice", { lines });

export const deleteInvoice = (id: number) => invoke<void>("delete_invoice", { id });

export const restoreInvoice = (id: number) => invoke<void>("restore_invoice", { id });

export const purgeInvoice = (id: number) => invoke<void>("purge_invoice", { id });

export const listTrashedInvoices = () => invoke<Invoice[]>("list_trashed_invoices");

export const duplicateInvoice = (id: number) => invoke<Invoice>("duplicate_invoice", { id });

/* -------------------------------------------------------------------- drafts */

export const listDrafts = () => invoke<StoredDraft[]>("list_drafts");

export const saveDraft = (key: string, data: unknown) =>
  invoke<StoredDraft>("save_draft", { key, data });

export const deleteDraft = (key: string) => invoke<void>("delete_draft", { key });

/* ------------------------------------------------------------------ settings */

export const getSettings = () => invoke<AppSettings>("get_settings");

export const saveSettings = (settings: AppSettings) =>
  invoke<AppSettings>("save_settings", { settings });

export const getBrandLogo = () => invoke<BrandLogo>("get_brand_logo");

export const saveBrandLogo = (svg: string) => invoke<BrandLogo>("save_brand_logo", { svg });

export const importBrandLogo = (path: string) => invoke<BrandLogo>("import_brand_logo", { path });

export const resetBrandLogo = () => invoke<BrandLogo>("reset_brand_logo");

export const applyBrandColor = () => invoke<BrandLogo>("apply_brand_color");

/* --------------------------------------------------------------------- excel */

export const excelInspect = (path: string, sheetName?: string) =>
  invoke<SheetPreview>("excel_inspect", { path, sheetName: sheetName ?? null });

export const excelDryRun = (path: string, sheetName: string | null, mapping: ColumnMapping) =>
  invoke<ImportPreview>("excel_dry_run", { path, sheetName, mapping });

export const excelApply = (
  path: string,
  sheetName: string | null,
  mapping: ColumnMapping,
  strategy: MatchStrategy,
  confirmedRows: number[],
) => invoke<ImportResult>("excel_apply", { path, sheetName, mapping, strategy, confirmedRows });

export const excelExport = (path: string, query: string) =>
  invoke<number>("excel_export", { path, query });

export const excelWriteTemplate = (path: string) => invoke<void>("excel_write_template", { path });

/* -------------------------------------------------------------------- backup */

export const createBackup = (destination?: string) =>
  invoke<BackupInfo>("create_backup", { destination: destination ?? null });

export const validateBackup = (path: string) =>
  invoke<BackupValidation>("validate_backup", { path });

export const restoreBackup = (path: string) => invoke<BackupInfo>("restore_backup", { path });

export const listBackups = () => invoke<BackupFile[]>("list_backups");

export const pruneBackups = (retention?: number) =>
  invoke<number>("prune_backups", { retention: retention ?? null });

export const lastBackup = () => invoke<string | null>("last_backup");

/* -------------------------------------------------------------------- export */

export const invoiceDocument = (invoiceId: number) =>
  invoke<InvoiceDocument>("invoice_document", { invoiceId });

export const previewDocument = (payload: {
  invoiceNumber: string;
  invoiceDate: string;
  customerName: string;
  customerPhone?: string | null;
  notes?: string | null;
  terms?: string[];
  lines: InvoiceInput["lines"];
}) => invoke<InvoiceDocument>("preview_document", { payload });

export const exportInvoicePdf = (invoiceId: number, path: string) =>
  invoke<string>("export_invoice_pdf", { invoiceId, path });

export const printInvoice = (invoiceId: number) => invoke<void>("print_invoice", { invoiceId });

export const exportInvoiceImages = (
  invoiceId: number,
  directory: string,
  format: ImageFormat,
  scale = 2,
) => invoke<ImageExportResult>("export_invoice_images", { invoiceId, directory, format, scale });

export const openContainingFolder = (path: string) =>
  invoke<void>("open_containing_folder", { path });

/* ------------------------------------------------------------------- updates */

export const currentVersion = () => invoke<string>("current_version");

export const checkForUpdates = () => invoke<UpdateStatus>("check_for_updates");

export const installUpdate = () => invoke<void>("install_update");

export const deferUpdate = (version: string) => invoke<void>("defer_update", { version });

export const deferredUpdate = () => invoke<string | null>("deferred_update");
