import { useEffect, useRef, useState } from "react";

import { InvoiceDocument } from "./InvoiceDocument";
import type { InvoiceDocument as DocumentModel } from "../lib/types";

/**
 * The on-screen preview of an invoice.
 *
 * The document is a real A4 page (210 mm wide), so it is scaled down to fit
 * the width it is given instead of forcing the reader to scroll sideways.
 *
 * The scaling is a CSS transform and never a `zoom`: a transform leaves the
 * measured layout untouched, and the page pagination is measured in pixels
 * (`InvoiceDocument` measures the real row heights).
 */
export function InvoicePreview({ document }: { document: DocumentModel }) {
  const frameRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const [scale, setScale] = useState(1);
  const [size, setSize] = useState({ width: 0, height: 0 });

  useEffect(() => {
    const frame = frameRef.current;
    const content = contentRef.current;
    if (!frame || !content) return;

    const update = () => {
      const page = content.querySelector<HTMLElement>("[data-invoice-page]");
      const width = page?.offsetWidth ?? 0;
      const height = content.offsetHeight;
      if (width > 0) {
        setSize((current) =>
          current.width === width && current.height === height ? current : { width, height },
        );
      }

      const style = window.getComputedStyle(frame);
      const available =
        frame.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight);
      if (available > 0 && width > 0) setScale(Math.min(1, available / width));
    };

    update();

    const observer = new ResizeObserver(update);
    observer.observe(frame);
    observer.observe(content);
    return () => observer.disconnect();
  }, [document]);

  return (
    <div className="invoice-preview" ref={frameRef}>
      {/* The sizer reserves exactly the scaled footprint of the document, so
          the scaled page never overflows its box. */}
      <div
        className="invoice-preview__sizer"
        style={{ width: size.width * scale, height: size.height * scale }}
      >
        <div
          className="invoice-preview__content"
          ref={contentRef}
          style={{ transform: `scale(${scale})`, width: size.width }}
        >
          <InvoiceDocument document={document} />
        </div>
      </div>
    </div>
  );
}
