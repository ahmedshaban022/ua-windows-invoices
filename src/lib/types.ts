/**
 * Types mirroring the Rust command layer. Every amount is an integer count of
 * minor currency units (piastres for EGP); quantities are integer thousandths.
 * No binary floating point is used for money anywhere in the application.
 */

export type CurrencyCode = "EGP" | "USD" | "EUR" | "SAR" | "AED" | string;

export interface Product {
  id: number;
  name: string;
  sku: string | null;
  basePriceMinor: number;
  deletedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface ProductSuggestion {
  id: number;
  name: string;
  sku: string | null;
  basePriceMinor: number;
}

export interface ProductPage {
  items: Product[];
  total: number;
  page: number;
  pageSize: number;
}

export interface ProductInput {
  name: string;
  basePriceMinor: number;
  sku?: string | null;
}

export type InvoiceStatus = "draft" | "issued";

export interface SellerSnapshot {
  companyName: string;
  companyPhone: string;
  companyAddress: string;
  logoSvg: string;
  brandColor: string;
  currencyCode: string;
  currencyExponent: number;
  dateFormat: string;
}

export interface StoredLine {
  id: number;
  productId: number | null;
  name: string;
  quantityThousandths: number;
  quantityDisplay: string;
  unitPriceMinor: number;
  lineTotalMinor: number;
  sortOrder: number;
}

export interface Invoice {
  id: number;
  invoiceNumber: string | null;
  displayNumber: string;
  invoiceDate: string;
  customerName: string;
  customerPhone: string | null;
  notes: string | null;
  /** The custom terms (بنود) the user attached to this invoice. */
  terms: string[];
  status: InvoiceStatus;
  sellerSnapshot: SellerSnapshot;
  totalMinor: number;
  lines: StoredLine[];
  deletedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface InvoicePage {
  items: Invoice[];
  total: number;
  page: number;
  pageSize: number;
}

export interface InvoiceLineInput {
  productId?: number | null;
  name: string;
  quantity: string;
  unitPrice: string;
}

export interface InvoiceInput {
  invoiceDate: string;
  customerName: string;
  customerPhone?: string | null;
  notes?: string | null;
  /** The custom terms (بنود) to print at the end of the invoice. */
  terms?: string[];
  lines: InvoiceLineInput[];
}

export interface InvoiceFilter {
  query?: string;
  dateFrom?: string | null;
  dateTo?: string | null;
  page?: number | null;
  pageSize?: number | null;
}

export interface ComputedLine {
  productId: number | null;
  name: string;
  quantityThousandths: number;
  quantityDisplay: string;
  unitPriceMinor: number;
  lineTotalMinor: number;
  sortOrder: number;
}

export interface ComputedInvoice {
  lines: ComputedLine[];
  totalMinor: number;
  currencyCode: string;
  currencyExponent: number;
}

export interface AppSettings {
  companyName: string;
  companyPhone: string;
  companyAddress: string;
  logoSvg: string;
  brandColor: string;
  currencyCode: CurrencyCode;
  currencyExponent: number;
  dateFormat: string;
  backupLocation: string;
  autoBackupEnabled: boolean;
  autoBackupRetention: number;
  /** The custom terms (بنود) the user can attach to an invoice. */
  invoiceTerms: string[];
}

export interface BrandLogo {
  svg: string;
  brandColor: string;
}

/** The canonical document shared by preview, PDF, print and images. */
export interface DocumentLine {
  name: string;
  quantityDisplay: string;
  unitPriceDisplay: string;
  lineTotalDisplay: string;
}

export interface DocumentSeller {
  companyName: string;
  companyPhone: string;
  companyAddress: string;
  logoSvg: string;
  brandColor: string;
  dateFormat: string;
}

export interface InvoiceDocument {
  invoiceNumber: string;
  isDraft: boolean;
  invoiceDate: string;
  customerName: string;
  customerPhone: string | null;
  notes: string | null;
  /** The custom terms (بنود) printed at the end of the document. */
  terms: string[];
  currencyCode: string;
  currencyNameAr: string;
  seller: DocumentSeller;
  lines: DocumentLine[];
  totalDisplay: string;
  totalWords: string;
}

export interface ColumnMapping {
  name: number;
  price: number;
  sku?: number | null;
}

export interface SheetInfo {
  name: string;
  rowCount: number;
}

export interface SheetPreview {
  sheets: SheetInfo[];
  headers: string[];
  sampleRows: string[][];
  suggestedMapping: ColumnMapping | null;
}

export type RowClassification =
  "new" | "duplicate_in_file" | "sku_match" | "name_match" | "invalid";

export interface RowVerdict {
  rowNumber: number;
  name: string;
  price: string;
  sku: string | null;
  valid: boolean;
  reason: string;
  classification: RowClassification;
  existingProductId: number | null;
  existingProductName: string | null;
}

export interface ImportPreview {
  rows: RowVerdict[];
  totalRows: number;
  validNew: number;
  duplicates: number;
  matches: number;
  invalid: number;
}

export interface ImportResult {
  imported: number;
  updated: number;
  skipped: number;
  invalid: number;
}

export type MatchStrategy = "skipMatched" | "updateConfirmed" | "addNewOnly";

export interface BackupInfo {
  filePath: string;
  sha256: string;
  sizeBytes: number;
  createdAt: string;
}

export interface BackupValidation {
  valid: boolean;
  sha256: string;
  sizeBytes: number;
  message: string;
}

export interface BackupFile {
  fileName: string;
  filePath: string;
  sizeBytes: number;
  createdAt: string;
}

export type ImageFormat = "png" | "jpg";

export interface ImageExportResult {
  files: string[];
  pageCount: number;
}

export interface UpdateStatus {
  currentVersion: string;
  updateAvailable: boolean;
  latestVersion: string | null;
  notes: string | null;
  publishedAt: string | null;
  message: string;
}

export interface UpdateProgress {
  downloaded: number;
  total: number | null;
}

export interface StoredDraft {
  key: string;
  data: unknown;
  updatedAt: string;
}
