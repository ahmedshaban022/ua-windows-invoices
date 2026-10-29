/**
 * An in-memory stand-in for the Rust command layer.
 *
 * It exists so the UI can be exercised in a plain browser (the Playwright
 * end-to-end suite) without the Tauri runtime. It is **only** used when the
 * app is not running inside Tauri.
 *
 * The arithmetic mirrors the Rust domain layer exactly: money is an integer
 * count of minor units and quantities are integer thousandths.
 */
import type {
  AppSettings,
  ComputedInvoice,
  Invoice,
  InvoiceDocument,
  InvoiceFilter,
  InvoiceInput,
  InvoicePage,
  Product,
  ProductPage,
} from "./types";

interface MockState {
  settings: AppSettings;
  products: Product[];
  invoices: Invoice[];
  drafts: Map<string, unknown>;
  sequence: number;
}

/** The payload the frontend sends to preview an unsaved invoice. */
interface PreviewPayload {
  invoiceNumber: string;
  invoiceDate: string;
  customerName: string;
  customerPhone?: string | null;
  notes?: string | null;
  terms?: string[];
  lines: InvoiceInput["lines"];
}

const state: MockState = {
  settings: {
    companyName: "",
    companyPhone: "",
    companyAddress: "",
    logoSvg:
      '<svg viewBox="0 0 40 40" width="40" height="40"><path fill="#0b68e6" d="M20 2 36 11v18L20 38 4 29V11z"/></svg>',
    brandColor: "#0b68e6",
    currencyCode: "EGP",
    currencyExponent: 2,
    dateFormat: "dd/MM/yyyy",
    backupLocation: "",
    autoBackupEnabled: false,
    autoBackupRetention: 7,
    invoiceTerms: ["حقوق النشر محفوظة", "ضمان شركة الأبنودي لمدة ثلاث أعوام"],
  },
  products: [
    {
      id: 1,
      name: "حليب كامل الدسم",
      sku: "MILK-1",
      basePriceMinor: 2500,
      deletedAt: null,
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-01T00:00:00Z",
    },
    {
      id: 2,
      name: "أرز مصري فاخر",
      sku: "RICE-1",
      basePriceMinor: 8000,
      deletedAt: null,
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-01T00:00:00Z",
    },
    {
      id: 3,
      name: "زيت زيتون بكر",
      sku: "OIL-1",
      basePriceMinor: 15000,
      deletedAt: null,
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-01T00:00:00Z",
    },
  ],
  invoices: [],
  drafts: new Map(),
  sequence: 1,
};

const STORAGE_KEY = "ua-invoice-mock-state";

interface PersistedState {
  products: Product[];
  invoices: Invoice[];
  drafts: [string, unknown][];
  sequence: number;
}

/** Restore the mock state from `localStorage` so a page reload keeps it. */
function restore(): void {
  if (typeof localStorage === "undefined") return;
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return;
    const saved = JSON.parse(raw) as PersistedState;
    state.products = saved.products ?? state.products;
    state.invoices = saved.invoices ?? [];
    state.drafts = new Map(saved.drafts ?? []);
    state.sequence = saved.sequence ?? 1;
  } catch {
    // A corrupt entry simply starts from the defaults.
  }
}

/** Persist the mock state so it survives a reload. */
function persist(): void {
  if (typeof localStorage === "undefined") return;
  try {
    const payload: PersistedState = {
      products: state.products,
      invoices: state.invoices,
      drafts: [...state.drafts.entries()],
      sequence: state.sequence,
    };
    localStorage.setItem(STORAGE_KEY, JSON.stringify(payload));
  } catch {
    // Storage may be unavailable (private mode); the mock still works.
  }
}

restore();

/** Normalize an Arabic string the way the Rust search does. */
function normalize(value: string): string {
  let out = "";
  let pendingSpace = false;

  for (const ch of value) {
    if (/\s/.test(ch)) {
      pendingSpace = out.length > 0;
      continue;
    }
    const code = ch.codePointAt(0) ?? 0;
    // tashkeel / harakat / superscript alef / quranic marks / tatweel
    if (
      (code >= 0x064b && code <= 0x0652) ||
      code === 0x0670 ||
      (code >= 0x0610 && code <= 0x061a) ||
      (code >= 0x06d6 && code <= 0x06ed) ||
      code === 0x0640
    ) {
      continue;
    }
    if (code >= 0x0660 && code <= 0x0669) {
      if (pendingSpace) {
        out += " ";
        pendingSpace = false;
      }
      out += String(code - 0x0660);
      continue;
    }
    if (code >= 0x06f0 && code <= 0x06f9) {
      if (pendingSpace) {
        out += " ";
        pendingSpace = false;
      }
      out += String(code - 0x06f0);
      continue;
    }
    const mapped = ((): string => {
      switch (code) {
        case 0x0622:
        case 0x0623:
        case 0x0625:
        case 0x0671:
        case 0x0672:
        case 0x0673:
          return "ا";
        case 0x0649:
          return "ي";
        case 0x0629:
          return "ه";
        case 0x0624:
          return "و";
        case 0x0626:
          return "ي";
        default:
          return ch;
      }
    })();
    if (pendingSpace) {
      out += " ";
      pendingSpace = false;
    }
    out += mapped.toLowerCase();
  }
  return out;
}

/** Parse a typed price into minor units (half-up). */
function parsePrice(text: string, exponent = 2): number | null {
  const cleaned = text.trim().replace(/٫/g, ".").replace(/[,\s]/g, "");
  if (!/^\d*\.?\d*$/.test(cleaned)) return null;
  const [intPart = "0", fracPart = ""] = cleaned.split(".");
  const intValue = Number(intPart || "0");
  if (!Number.isFinite(intValue)) return null;
  const kept = fracPart.slice(0, exponent).padEnd(exponent, "0");
  let minor = intValue * 10 ** exponent + Number(kept || "0");
  if (fracPart.length > exponent && Number(fracPart[exponent]) >= 5) minor += 1;
  return minor;
}

/** Parse a typed quantity into integer thousandths. */
function parseQuantity(text: string): number | null {
  const cleaned = text.trim().replace(/٫/g, ".");
  if (!/^\d*\.?\d*$/.test(cleaned)) return null;
  const [intPart = "0", fracPart = ""] = cleaned.split(".");
  const intValue = Number(intPart || "0");
  if (!Number.isFinite(intValue)) return null;
  const kept = (fracPart + "000").slice(0, 3);
  const value = intValue * 1000 + Number(kept || "0");
  return value > 0 ? value : null;
}

function formatMinor(minor: number, exponent = 2): string {
  const scale = 10 ** exponent;
  const negative = minor < 0;
  const abs = Math.abs(minor);
  const major = Math.trunc(abs / scale);
  const frac = abs % scale;
  return `${negative ? "-" : ""}${major}.${String(frac).padStart(exponent, "0")}`;
}

function formatThousandths(thousandths: number): string {
  const major = Math.trunc(thousandths / 1000);
  const frac = thousandths % 1000;
  if (frac === 0) return `${major}`;
  if (frac % 100 === 0) return `${major}.${frac / 100}`;
  if (frac % 10 === 0) return `${major}.${String(frac / 10).padStart(2, "0")}`;
  return `${major}.${String(frac).padStart(3, "0")}`;
}

function computeLines(lines: InvoiceInput["lines"]): ComputedInvoice {
  const exponent = state.settings.currencyExponent;
  const computed = lines.map((line, index) => {
    const quantity = parseQuantity(line.quantity) ?? 0;
    const price = parsePrice(line.unitPrice, exponent) ?? 0;
    // half-up on quantity * price, both integers.
    const numerator = quantity * price;
    const total = Math.floor(numerator / 1000) + (numerator % 1000 >= 500 ? 1 : 0);
    return {
      productId: line.productId ?? null,
      name: line.name,
      quantityThousandths: quantity,
      quantityDisplay: formatThousandths(quantity),
      unitPriceMinor: price,
      lineTotalMinor: total,
      sortOrder: index,
    };
  });
  return {
    lines: computed,
    totalMinor: computed.reduce((sum, line) => sum + line.lineTotalMinor, 0),
    currencyCode: state.settings.currencyCode,
    currencyExponent: exponent,
  };
}

function buildDocument(
  invoiceNumber: string,
  input: InvoiceInput,
  computed: ComputedInvoice,
  isDraft: boolean,
): InvoiceDocument {
  const settings = state.settings;
  return {
    invoiceNumber,
    isDraft,
    invoiceDate: input.invoiceDate,
    customerName: input.customerName,
    customerPhone: input.customerPhone ?? null,
    notes: input.notes ?? null,
    terms: (input.terms ?? []).map((term) => term.trim()).filter(Boolean),
    currencyCode: settings.currencyCode,
    currencyNameAr: "جنيه مصري",
    seller: {
      companyName: settings.companyName,
      companyPhone: settings.companyPhone,
      companyAddress: settings.companyAddress,
      logoSvg: settings.logoSvg,
      brandColor: settings.brandColor,
      dateFormat: settings.dateFormat,
    },
    lines: computed.lines.map((line) => ({
      name: line.name,
      quantityDisplay: line.quantityDisplay,
      unitPriceDisplay: formatMinor(line.unitPriceMinor, settings.currencyExponent),
      lineTotalDisplay: formatMinor(line.lineTotalMinor, settings.currencyExponent),
    })),
    totalDisplay: formatMinor(computed.totalMinor, settings.currencyExponent),
    totalWords: `${formatMinor(computed.totalMinor, settings.currencyExponent)} جنيه`,
  };
}

function nowIso() {
  return new Date().toISOString();
}

function makeInvoice(input: InvoiceInput, status: "draft" | "issued"): Invoice {
  const computed = computeLines(input.lines);
  const id = state.invoices.length + 1;
  const issued = status === "issued";
  const number = issued ? `INV-${String(state.sequence++).padStart(6, "0")}` : null;
  const invoice: Invoice = {
    id,
    invoiceNumber: number,
    displayNumber: number ?? `مسودة-${id}`,
    invoiceDate: input.invoiceDate,
    customerName: input.customerName,
    customerPhone: input.customerPhone ?? null,
    notes: input.notes ?? null,
    terms: (input.terms ?? []).map((term) => term.trim()).filter(Boolean),
    status,
    sellerSnapshot: {
      companyName: state.settings.companyName,
      companyPhone: state.settings.companyPhone,
      companyAddress: state.settings.companyAddress,
      logoSvg: state.settings.logoSvg,
      brandColor: state.settings.brandColor,
      currencyCode: state.settings.currencyCode,
      currencyExponent: state.settings.currencyExponent,
      dateFormat: state.settings.dateFormat,
    },
    totalMinor: computed.totalMinor,
    lines: computed.lines.map((line, index) => ({
      id: index + 1,
      productId: line.productId,
      name: line.name,
      quantityThousandths: line.quantityThousandths,
      quantityDisplay: line.quantityDisplay,
      unitPriceMinor: line.unitPriceMinor,
      lineTotalMinor: line.lineTotalMinor,
      sortOrder: line.sortOrder,
    })),
    deletedAt: null,
    createdAt: nowIso(),
    updatedAt: nowIso(),
  };
  state.invoices.push(invoice);
  return invoice;
}

/** Dispatch a command to the mock backend. */
export function mockInvoke(command: string, args: Record<string, unknown> = {}): Promise<unknown> {
  switch (command) {
    case "get_settings":
      return Promise.resolve(state.settings);
    case "save_settings":
      state.settings = { ...state.settings, ...(args.settings as AppSettings) };
      return Promise.resolve(state.settings);

    case "list_products": {
      const query = normalize(String(args.query ?? ""));
      const page = Number(args.page ?? 1);
      const pageSize = Number(args.pageSize ?? 25);
      const includeDeleted = Boolean(args.includeDeleted);
      const filtered = state.products.filter((product) => {
        if (!includeDeleted && product.deletedAt) return false;
        if (query.length === 0) return true;
        return (
          normalize(product.name).includes(query) || normalize(product.sku ?? "").includes(query)
        );
      });
      const items = filtered.slice((page - 1) * pageSize, page * pageSize);
      return Promise.resolve({
        items,
        total: filtered.length,
        page,
        pageSize,
      } satisfies ProductPage);
    }
    case "get_product":
      return Promise.resolve(
        state.products.find((product) => product.id === Number(args.id)) ?? null,
      );
    case "create_product": {
      const input = args.input as { name: string; basePriceMinor: number; sku?: string | null };
      const product: Product = {
        id: Math.max(0, ...state.products.map((item) => item.id)) + 1,
        name: input.name,
        sku: input.sku ?? null,
        basePriceMinor: input.basePriceMinor,
        deletedAt: null,
        createdAt: nowIso(),
        updatedAt: nowIso(),
      };
      state.products.push(product);
      persist();
      return Promise.resolve(product);
    }
    case "update_product": {
      const id = Number(args.id);
      const input = args.input as { name: string; basePriceMinor: number; sku?: string | null };
      const product = state.products.find((item) => item.id === id);
      if (!product) return Promise.reject(new Error("المنتج غير موجود"));
      Object.assign(product, input, { updatedAt: nowIso() });
      return Promise.resolve(product);
    }
    case "delete_product": {
      const product = state.products.find((item) => item.id === Number(args.id));
      if (product) product.deletedAt = nowIso();
      return Promise.resolve(undefined);
    }
    case "restore_product": {
      const product = state.products.find((item) => item.id === Number(args.id));
      if (product) product.deletedAt = null;
      return Promise.resolve(undefined);
    }
    case "suggest_products": {
      const query = normalize(String(args.query ?? ""));
      const limit = Number(args.limit ?? 10);
      const items = state.products
        .filter((product) => !product.deletedAt)
        .filter(
          (product) =>
            query.length === 0 ||
            normalize(product.name).includes(query) ||
            normalize(product.sku ?? "").includes(query),
        )
        .slice(0, limit)
        .map((product) => ({
          id: product.id,
          name: product.name,
          sku: product.sku,
          basePriceMinor: product.basePriceMinor,
        }));
      return Promise.resolve(items);
    }
    case "count_products":
      return Promise.resolve(state.products.filter((product) => !product.deletedAt).length);

    case "preview_invoice": {
      const lines = (args.lines ?? []) as InvoiceInput["lines"];
      return Promise.resolve(computeLines(lines));
    }
    case "create_draft":
      return Promise.resolve(makeInvoice(args.input as InvoiceInput, "draft"));
    case "update_draft": {
      const id = Number(args.id);
      const invoice = state.invoices.find((item) => item.id === id);
      if (!invoice) return Promise.reject(new Error("الفاتورة غير موجودة"));
      const input = args.input as InvoiceInput;
      const computed = computeLines(input.lines);
      invoice.invoiceDate = input.invoiceDate;
      invoice.customerName = input.customerName;
      invoice.customerPhone = input.customerPhone ?? null;
      invoice.notes = input.notes ?? null;
      invoice.totalMinor = computed.totalMinor;
      invoice.lines = computed.lines.map((line, index) => ({
        id: index + 1,
        productId: line.productId,
        name: line.name,
        quantityThousandths: line.quantityThousandths,
        quantityDisplay: line.quantityDisplay,
        unitPriceMinor: line.unitPriceMinor,
        lineTotalMinor: line.lineTotalMinor,
        sortOrder: line.sortOrder,
      }));
      return Promise.resolve(invoice);
    }
    case "issue_invoice": {
      const id = Number(args.id);
      const invoice = state.invoices.find((item) => item.id === id);
      if (!invoice) return Promise.reject(new Error("الفاتورة غير موجودة"));
      invoice.status = "issued";
      invoice.invoiceNumber = `INV-${String(state.sequence++).padStart(6, "0")}`;
      invoice.displayNumber = invoice.invoiceNumber;
      persist();
      return Promise.resolve(invoice);
    }
    case "list_invoices": {
      const filter = (args.filter ?? {}) as InvoiceFilter;
      const query = normalize(String(filter.query ?? ""));
      const filtered = state.invoices.filter(
        (invoice) =>
          !invoice.deletedAt &&
          invoice.status === "issued" &&
          (query.length === 0 ||
            normalize(invoice.customerName).includes(query) ||
            (invoice.invoiceNumber ?? "").includes(query)),
      );
      const page = filter.page ?? 1;
      const pageSize = filter.pageSize ?? 25;
      return Promise.resolve({
        items: filtered.slice((page - 1) * pageSize, page * pageSize),
        total: filtered.length,
        page,
        pageSize,
      } satisfies InvoicePage);
    }
    case "get_invoice":
      return Promise.resolve(
        state.invoices.find((invoice) => invoice.id === Number(args.id)) ?? null,
      );
    case "delete_invoice": {
      const invoice = state.invoices.find((item) => item.id === Number(args.id));
      if (invoice) invoice.deletedAt = nowIso();
      persist();
      return Promise.resolve(undefined);
    }
    case "restore_invoice": {
      const invoice = state.invoices.find((item) => item.id === Number(args.id));
      if (invoice) invoice.deletedAt = null;
      persist();
      return Promise.resolve(undefined);
    }
    case "purge_invoice": {
      state.invoices = state.invoices.filter((item) => item.id !== Number(args.id));
      persist();
      return Promise.resolve(undefined);
    }
    case "list_trashed_invoices":
      return Promise.resolve(state.invoices.filter((invoice) => invoice.deletedAt));
    case "duplicate_invoice": {
      const source = state.invoices.find((item) => item.id === Number(args.id));
      if (!source) return Promise.reject(new Error("الفاتورة غير موجودة"));
      return Promise.resolve(
        makeInvoice(
          {
            invoiceDate: source.invoiceDate,
            customerName: source.customerName,
            customerPhone: source.customerPhone,
            notes: source.notes,
            terms: source.terms,
            lines: source.lines.map((line) => ({
              productId: line.productId,
              name: line.name,
              quantity: line.quantityDisplay,
              unitPrice: formatMinor(line.unitPriceMinor, state.settings.currencyExponent),
            })),
          },
          "draft",
        ),
      );
    }

    case "list_drafts":
      return Promise.resolve(
        [...state.drafts.entries()].map(([key, data]) => ({
          key,
          data,
          updatedAt: nowIso(),
        })),
      );
    case "save_draft":
      state.drafts.set(String(args.key), args.data);
      persist();
      return Promise.resolve({ key: args.key, data: args.data, updatedAt: nowIso() });
    case "delete_draft":
      state.drafts.delete(String(args.key));
      persist();
      return Promise.resolve(undefined);

    case "preview_document": {
      const payload =
        (args as unknown as { payload: PreviewPayload }).payload ??
        (args as unknown as PreviewPayload);
      const computed = computeLines(payload.lines ?? []);
      return Promise.resolve(
        buildDocument(
          payload.invoiceNumber,
          {
            invoiceDate: payload.invoiceDate,
            customerName: payload.customerName,
            customerPhone: payload.customerPhone ?? null,
            notes: payload.notes ?? null,
            terms: payload.terms ?? [],
            lines: payload.lines ?? [],
          },
          computed,
          true,
        ),
      );
    }
    case "invoice_document": {
      const invoice = state.invoices.find((item) => item.id === Number(args.invoiceId));
      if (!invoice) return Promise.reject(new Error("الفاتورة غير موجودة"));
      const computed = computeLines(
        invoice.lines.map((line) => ({
          productId: line.productId,
          name: line.name,
          quantity: line.quantityDisplay,
          unitPrice: formatMinor(line.unitPriceMinor, state.settings.currencyExponent),
        })),
      );
      return Promise.resolve(
        buildDocument(
          invoice.displayNumber,
          {
            invoiceDate: invoice.invoiceDate,
            customerName: invoice.customerName,
            customerPhone: invoice.customerPhone,
            notes: invoice.notes,
            terms: invoice.terms,
            lines: [],
          },
          computed,
          invoice.status === "draft",
        ),
      );
    }

    default:
      return Promise.reject(new Error(`الأمر «${command}» غير متاح في وضع المتصفح التجريبي`));
  }
}

/** Reset the mock state (used between tests). */
export function resetMock() {
  state.invoices = [];
  state.drafts.clear();
  state.sequence = 1;
}
