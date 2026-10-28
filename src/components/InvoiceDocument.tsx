import { useLayoutEffect, useMemo, useRef, useState } from "react";

import { formatDate } from "../lib/format";
import type { InvoiceDocument as DocumentModel, DocumentLine } from "../lib/types";

/** A4 content height in CSS pixels (297mm minus the page padding). */
const PAGE_CONTENT_HEIGHT = 1017;

/** A page is a list of line indices into the document's line array. */
type PageIndexes = number[];

interface Props {
  document: DocumentModel;
  /** Called once the document is laid out and the pages are final. */
  onReady?: () => void;
}

/**
 * The one and only invoice renderer.
 *
 * Preview, PDF, print and image export all render this component, so they
 * cannot drift apart.
 */
export function InvoiceDocument({ document, onReady }: Props) {
  const pages = usePaginate(document, onReady);

  return (
    <div className="invoice-document" dir="rtl">
      <MeasuringTable document={document} />
      {pages.map((indexes, pageIndex) => (
        <Page
          key={pageIndex}
          document={document}
          lines={indexes
            .map((index) => document.lines[index])
            .filter((line): line is DocumentLine => line !== undefined)}
          lineNumbers={indexes.map((index) => index + 1)}
          pageIndex={pageIndex}
          pageCount={pages.length}
        />
      ))}
    </div>
  );
}

interface PageProps {
  document: DocumentModel;
  lines: DocumentLine[];
  lineNumbers: number[];
  pageIndex: number;
  pageCount: number;
}

function Page({ document, lines, lineNumbers, pageIndex, pageCount }: PageProps) {
  const brand = document.seller.brandColor || "#0b68e6";
  const currency = document.currencyNameAr;
  const isLast = pageIndex === pageCount - 1;

  return (
    <section
      className="invoice-page"
      data-invoice-page
      style={{ ["--doc-brand" as string]: brand }}
      lang="ar"
    >
      <header className="invoice-head">
        <div
          className="invoice-head__logo"
          dangerouslySetInnerHTML={{ __html: document.seller.logoSvg }}
        />

        <div className="invoice-head__seller">
          <div className="invoice-head__company">{document.seller.companyName}</div>
          <div className="invoice-head__meta">
            {document.seller.companyPhone ? <div>{document.seller.companyPhone}</div> : null}
            {document.seller.companyAddress ? <div>{document.seller.companyAddress}</div> : null}
          </div>
        </div>

        <div className="invoice-head__title">
          <div className="invoice-head__title-word">فاتورة</div>
          <div className="invoice-head__title-sub">Invoice</div>
          {document.isDraft ? <span className="invoice-head__draft">مسودة</span> : null}
        </div>
      </header>

      <div className="invoice-meta">
        <div className="invoice-meta__block">
          <div className="invoice-meta__label">رقم الفاتورة</div>
          <div className="invoice-meta__value ltr" style={{ textAlign: "start" }}>
            {document.invoiceNumber}
          </div>
        </div>
        <div className="invoice-meta__block">
          <div className="invoice-meta__label">التاريخ</div>
          <div className="invoice-meta__value ltr" style={{ textAlign: "start" }}>
            {formatDate(document.invoiceDate, document.seller.dateFormat)}
          </div>
        </div>
        <div className="invoice-meta__block">
          <div className="invoice-meta__label">العميل</div>
          <div className="invoice-meta__value">{document.customerName}</div>
          {document.customerPhone ? (
            <div
              className="invoice-meta__value ltr"
              style={{ textAlign: "start", fontSize: "9.5pt" }}
            >
              {document.customerPhone}
            </div>
          ) : null}
        </div>
      </div>

      <table className="invoice-table">
        <thead>
          <tr>
            <th className="invoice-table__index">#</th>
            <th>البند</th>
            <th className="invoice-table__qty">الكمية</th>
            <th className="invoice-table__money">سعر الوحدة</th>
            <th className="invoice-table__money">الإجمالي</th>
          </tr>
        </thead>
        <tbody>
          {lines.length === 0 ? (
            <tr>
              <td colSpan={5} style={{ textAlign: "center", color: "#7b8794", padding: "18px" }}>
                لا توجد بنود
              </td>
            </tr>
          ) : (
            lines.map((line, index) => (
              <tr key={index}>
                <td className="invoice-table__index">{lineNumbers[index]}</td>
                <td className="invoice-table__name">{line.name}</td>
                <td className="invoice-table__qty">{line.quantityDisplay}</td>
                <td className="invoice-table__money">{line.unitPriceDisplay}</td>
                <td className="invoice-table__money">{line.lineTotalDisplay}</td>
              </tr>
            ))
          )}
        </tbody>
      </table>

      {isLast ? (
        <>
          <div className="invoice-totals">
            <div className="invoice-totals__box">
              <div className="invoice-totals__row invoice-totals__row--grand">
                <span className="invoice-totals__label">الإجمالي</span>
                <span className="invoice-totals__value">
                  {document.totalDisplay} {currency}
                </span>
              </div>
              <div className="invoice-totals__words">الإجمالي كتابةً: {document.totalWords}</div>
            </div>
          </div>

          {document.notes ? (
            <div className="invoice-notes">
              <div className="invoice-notes__label">ملاحظات</div>
              <div>{document.notes}</div>
            </div>
          ) : null}
        </>
      ) : null}

      <footer className="invoice-foot">
        <span>{document.seller.companyName}</span>
        <span className="invoice-foot__page">
          {pageIndex + 1} / {pageCount}
        </span>
      </footer>
    </section>
  );
}

/**
 * Split the lines over A4 pages.
 *
 * The rows are measured in a hidden copy of the table so the split follows the
 * real content height instead of a guessed number of rows per page.
 */
function usePaginate(document: DocumentModel, onReady?: () => void): PageIndexes[] {
  const [pages, setPages] = useState<PageIndexes[]>([document.lines.map((_, index) => index)]);
  const measureRef = useRef<HTMLDivElement>(null);

  const lineKey = useMemo(
    () =>
      document.lines
        .map((line) => `${line.name}|${line.quantityDisplay}|${line.unitPriceDisplay}`)
        .join("\n"),
    [document.lines],
  );

  useLayoutEffect(() => {
    const measure = measureRef.current;
    if (!measure) return;

    const rows = Array.from(measure.querySelectorAll<HTMLElement>("[data-row-index]"));
    const head = measure.querySelector<HTMLElement>("[data-head-height]");
    const foot = measure.querySelector<HTMLElement>("[data-foot-height]");

    const overhead =
      (head?.offsetHeight ?? 0) +
      (foot?.offsetHeight ?? 0) +
      // The totals box and the notes block only live on the last page.
      150;

    const available = Math.max(120, PAGE_CONTENT_HEIGHT - overhead);

    const chunks: PageIndexes[] = [];
    let current: PageIndexes = [];
    let used = 0;

    for (let index = 0; index < document.lines.length; index += 1) {
      const height = rows[index]?.offsetHeight ?? 26;
      if (current.length > 0 && used + height > available) {
        chunks.push(current);
        current = [];
        used = 0;
      }
      current.push(index);
      used += height;
    }
    chunks.push(current);

    setPages(chunks.length > 0 ? chunks : [[]]);
    // `lineKey` is the stable identity of the measured content.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [lineKey]);

  useLayoutEffect(() => {
    // Two frames guarantee the browser has laid the pages out.
    requestAnimationFrame(() => {
      requestAnimationFrame(() => onReady?.());
    });
  }, [pages, onReady]);

  return pages;
}

/** The hidden measuring table. It must mirror the real table exactly. */
function MeasuringTable({ document }: { document: DocumentModel }) {
  return (
    <div className="invoice-page__measure" aria-hidden="true" ref={undefined}>
      <div data-head-height>
        <header className="invoice-head">
          <div className="invoice-head__logo" />
          <div className="invoice-head__seller">
            <div className="invoice-head__company">{document.seller.companyName}</div>
            <div className="invoice-head__meta">قياس</div>
          </div>
          <div className="invoice-head__title">
            <div className="invoice-head__title-word">فاتورة</div>
          </div>
        </header>
        <div className="invoice-meta">
          <div className="invoice-meta__block">
            <div className="invoice-meta__label">رقم الفاتورة</div>
            <div className="invoice-meta__value">000000</div>
          </div>
          <div className="invoice-meta__block">
            <div className="invoice-meta__label">التاريخ</div>
            <div className="invoice-meta__value">00/00/0000</div>
          </div>
          <div className="invoice-meta__block">
            <div className="invoice-meta__label">العميل</div>
            <div className="invoice-meta__value">اسم العميل</div>
          </div>
        </div>
      </div>

      <table className="invoice-table">
        <thead>
          <tr>
            <th className="invoice-table__index">#</th>
            <th>البند</th>
            <th className="invoice-table__qty">الكمية</th>
            <th className="invoice-table__money">سعر الوحدة</th>
            <th className="invoice-table__money">الإجمالي</th>
          </tr>
        </thead>
        <tbody>
          {document.lines.map((line, index) => (
            <tr key={index} data-row-index={index}>
              <td className="invoice-table__index">{index + 1}</td>
              <td className="invoice-table__name">{line.name}</td>
              <td className="invoice-table__qty">{line.quantityDisplay}</td>
              <td className="invoice-table__money">{line.unitPriceDisplay}</td>
              <td className="invoice-table__money">{line.lineTotalDisplay}</td>
            </tr>
          ))}
        </tbody>
      </table>

      <div data-foot-height>
        <footer className="invoice-foot">
          <span>قياس</span>
          <span className="invoice-foot__page">1 / 1</span>
        </footer>
      </div>
    </div>
  );
}
