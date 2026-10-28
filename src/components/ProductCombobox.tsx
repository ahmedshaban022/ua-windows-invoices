import { useEffect, useId, useRef, useState } from "react";

import * as api from "../lib/api";
import { formatMinor } from "../lib/format";
import type { ProductSuggestion } from "../lib/types";

interface Props {
  /** The current product name. */
  value: string;
  /** Called when the user picks a product or finishes typing a name. */
  onSelect: (name: string, product: ProductSuggestion | null) => void;
  /** Called when a suggestion is picked, so the price can be filled in. */
  onPick?: (product: ProductSuggestion) => void;
  placeholder?: string;
  exponent?: number;
  currencyCode?: string;
  id?: string;
}

/**
 * The product combobox (FR-02).
 *
 * Searches the catalog through the Rust FTS5 trigram index and also accepts
 * free text, because an invoice line does not have to exist in the catalog.
 */
export function ProductCombobox({
  value,
  onSelect,
  onPick,
  placeholder = "ابحث عن صنف أو اكتب اسمًا جديدًا",
  exponent = 2,
  currencyCode = "EGP",
  id,
}: Props) {
  const [open, setOpen] = useState(false);
  const [items, setItems] = useState<ProductSuggestion[]>([]);
  const [active, setActive] = useState(0);
  const [loading, setLoading] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const lastEmitted = useRef<string>("");
  const listId = useId();

  // Debounced search against the catalog.
  useEffect(() => {
    let cancelled = false;
    const handle = window.setTimeout(async () => {
      setLoading(true);
      try {
        const results = await api.suggestProducts(value, 10);
        if (!cancelled) {
          setItems(results);
          setActive(0);
        }
      } catch {
        if (!cancelled) setItems([]);
      } finally {
        if (!cancelled) setLoading(false);
      }
    }, 120);
    return () => {
      cancelled = true;
      window.clearTimeout(handle);
    };
  }, [value]);

  // Close the list when clicking elsewhere.
  useEffect(() => {
    const onPointerDown = (event: MouseEvent) => {
      if (!containerRef.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onPointerDown);
    return () => document.removeEventListener("mousedown", onPointerDown);
  }, []);

  const commitFreeText = () => {
    const trimmed = value.trim();
    // Only emit when the value actually changed since the last emission.
    // Otherwise a late blur would re-apply a stale line and wipe edits the
    // user made in the meantime (for example the unit price).
    if (trimmed === lastEmitted.current.trim()) return;
    lastEmitted.current = trimmed;
    onSelect(trimmed, null);
    setOpen(false);
  };

  const choose = (product: ProductSuggestion) => {
    lastEmitted.current = product.name;
    onSelect(product.name, product);
    onPick?.(product);
    setOpen(false);
  };

  return (
    <div className="combobox" ref={containerRef}>
      <input
        id={id}
        ref={inputRef}
        className="input"
        value={value}
        placeholder={placeholder}
        role="combobox"
        aria-expanded={open}
        aria-controls={listId}
        aria-autocomplete="list"
        onFocus={() => setOpen(true)}
        onChange={(event) => {
          lastEmitted.current = event.target.value;
          onSelect(event.target.value, null);
          setOpen(true);
        }}
        onKeyDown={(event) => {
          if (event.key === "ArrowDown") {
            event.preventDefault();
            setOpen(true);
            setActive((index) => Math.min(index + 1, items.length - 1));
          } else if (event.key === "ArrowUp") {
            event.preventDefault();
            setActive((index) => Math.max(index - 1, 0));
          } else if (event.key === "Enter") {
            event.preventDefault();
            if (open && items[active]) {
              choose(items[active]);
            } else {
              commitFreeText();
            }
          } else if (event.key === "Escape") {
            setOpen(false);
          }
        }}
        onBlur={() => {
          // A short delay lets a click on an option land first.
          window.setTimeout(() => {
            setOpen(false);
            commitFreeText();
          }, 120);
        }}
      />

      {open && (items.length > 0 || loading) ? (
        <ul className="combobox__list" id={listId} role="listbox">
          {loading && items.length === 0 ? <li className="combobox__empty">جارٍ البحث…</li> : null}
          {items.map((product, index) => (
            <li key={product.id}>
              <button
                type="button"
                role="option"
                aria-selected={index === active}
                className={`combobox__option ${index === active ? "combobox__option--active" : ""}`}
                onMouseEnter={() => setActive(index)}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => choose(product)}
              >
                <span className="combobox__name">{product.name}</span>
                {product.sku ? <span className="combobox__sku">{product.sku}</span> : null}
                <span className="combobox__price ltr">
                  {formatMinor(product.basePriceMinor, exponent)} {currencyCode}
                </span>
              </button>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
