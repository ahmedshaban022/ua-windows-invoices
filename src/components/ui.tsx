/** The shared UI kit. Every screen is built from these primitives. */
import {
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
} from "react";
import { createPortal } from "react-dom";
import { Check, ChevronDown } from "lucide-react";

import { dismissToast, useToasts } from "../state/toast";

/* ----------------------------------------------------------------- buttons */

type ButtonVariant = "primary" | "default" | "soft" | "ghost" | "danger";

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: "md" | "sm";
  block?: boolean;
  loading?: boolean;
}

export function Button({
  variant = "default",
  size = "md",
  block = false,
  loading = false,
  disabled,
  children,
  className = "",
  ...rest
}: ButtonProps) {
  const classes = [
    "btn",
    variant !== "default" ? `btn--${variant}` : "",
    size === "sm" ? "btn--sm" : "",
    block ? "btn--block" : "",
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <button className={classes} disabled={disabled || loading} {...rest}>
      {loading ? <span className="spinner" aria-hidden="true" /> : null}
      {children}
    </button>
  );
}

interface IconButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  label: string;
}

export function IconButton({ label, children, ...rest }: IconButtonProps) {
  return (
    <button className="btn btn--icon" title={label} aria-label={label} {...rest}>
      {children}
    </button>
  );
}

/* ------------------------------------------------------------------ fields */

interface FieldProps {
  label: string;
  hint?: string;
  error?: string | null;
  children: ReactNode;
  htmlFor?: string;
}

export function Field({ label, hint, error, children, htmlFor }: FieldProps) {
  return (
    <div className="field">
      <label className="field__label" htmlFor={htmlFor}>
        {label}
      </label>
      {children}
      {error ? <span className="field__error">{error}</span> : null}
      {!error && hint ? <span className="field__hint">{hint}</span> : null}
    </div>
  );
}

/* ----------------------------------------------------------------- selects */

export interface SelectOption {
  value: string;
  label: string;
}

interface SelectProps {
  /** The value of the selected option. */
  value: string;
  /** The options, in the order they are listed in. */
  options: SelectOption[];
  /** Called with the value of the picked option. */
  onChange: (value: string) => void;
  id?: string;
  className?: string;
  disabled?: boolean;
  /** Shown when the value matches no option. */
  placeholder?: string;
  ariaLabel?: string;
}

/**
 * The dropdown (listbox) of the application.
 *
 * A native `<select>` cannot be styled: Windows draws its popup itself, so
 * the list always looks like the operating system instead of like the app.
 * This component draws the whole thing — the trigger and the popup — with the
 * application's own surfaces, and keeps the keyboard and screen-reader
 * behaviour a listbox is expected to have.
 */
export function Select({
  value,
  options,
  onChange,
  id,
  className = "",
  disabled = false,
  placeholder = "—",
  ariaLabel,
}: SelectProps) {
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const [placement, setPlacement] = useState<CSSProperties | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const buttonRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const listId = useId();

  const selectedIndex = options.findIndex((option) => option.value === value);
  const selected = selectedIndex >= 0 ? options[selectedIndex] : null;

  // Close the list when clicking elsewhere. The list itself lives at the body
  // level, so it has to be excluded as well.
  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: MouseEvent) => {
      const target = event.target as Node;
      if (containerRef.current?.contains(target)) return;
      if (listRef.current?.contains(target)) return;
      setOpen(false);
    };
    document.addEventListener("mousedown", onPointerDown);
    return () => document.removeEventListener("mousedown", onPointerDown);
  }, [open]);

  /**
   * The list is drawn at the body level, so that no scrolling container (the
   * body of a dialog, for instance) can cut it off. That means its position
   * has to be measured and kept in sync with the field.
   */
  const place = useCallback(() => {
    const trigger = buttonRef.current;
    if (!trigger) return;
    const rect = trigger.getBoundingClientRect();
    const below = window.innerHeight - rect.bottom;
    const needed = Math.min(280, options.length * 38 + 8);
    const up = below < needed && rect.top > below;
    setPlacement(
      up
        ? { bottom: window.innerHeight - rect.top + 4, left: rect.left, width: rect.width }
        : { top: rect.bottom + 4, left: rect.left, width: rect.width },
    );
  }, [options.length]);

  useEffect(() => {
    if (!open) return;
    place();
    const onViewportChange = () => place();
    window.addEventListener("resize", onViewportChange);
    window.addEventListener("scroll", onViewportChange, true);
    return () => {
      window.removeEventListener("resize", onViewportChange);
      window.removeEventListener("scroll", onViewportChange, true);
    };
  }, [open, place]);

  // Keep the highlighted option inside the scroll area.
  useEffect(() => {
    if (!open) return;
    const list = listRef.current;
    const option = list?.children[active] as HTMLElement | undefined;
    option?.scrollIntoView({ block: "nearest" });
  }, [open, active]);

  const openList = () => {
    if (disabled) return;
    setActive(selectedIndex >= 0 ? selectedIndex : 0);
    setOpen(true);
  };

  const choose = (option: SelectOption) => {
    onChange(option.value);
    setOpen(false);
    buttonRef.current?.focus();
  };

  const onKeyDown = (event: React.KeyboardEvent<HTMLButtonElement>) => {
    if (!open) {
      if (event.key === "ArrowDown" || event.key === "ArrowUp" || event.key === "Enter") {
        event.preventDefault();
        openList();
      }
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActive((index) => Math.min(index + 1, options.length - 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActive((index) => Math.max(index - 1, 0));
    } else if (event.key === "Home") {
      event.preventDefault();
      setActive(0);
    } else if (event.key === "End") {
      event.preventDefault();
      setActive(options.length - 1);
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      if (options[active]) choose(options[active]);
    } else if (event.key === "Escape") {
      event.preventDefault();
      setOpen(false);
      buttonRef.current?.focus();
    } else if (event.key === "Tab") {
      setOpen(false);
    }
  };

  return (
    <div className={`select ${className}`.trim()} ref={containerRef}>
      <button
        type="button"
        id={id}
        ref={buttonRef}
        className="select__trigger"
        role="combobox"
        aria-expanded={open}
        aria-controls={listId}
        aria-haspopup="listbox"
        aria-label={ariaLabel}
        disabled={disabled}
        onClick={() => (open ? setOpen(false) : openList())}
        onKeyDown={onKeyDown}
      >
        <span className="select__value">{selected ? selected.label : placeholder}</span>
        <ChevronDown className="select__chevron" size={16} aria-hidden="true" />
      </button>

      {open && placement
        ? createPortal(
            <ul
              className="select__list"
              style={placement}
              id={listId}
              role="listbox"
              ref={listRef}
              aria-activedescendant={`${listId}-${active}`}
            >
              {options.map((option, index) => (
                <li key={option.value} id={`${listId}-${index}`}>
                  <button
                    type="button"
                    role="option"
                    aria-selected={option.value === value}
                    className={`select__option ${index === active ? "select__option--active" : ""} ${
                      option.value === value ? "select__option--selected" : ""
                    }`.trim()}
                    onMouseEnter={() => setActive(index)}
                    onClick={() => choose(option)}
                  >
                    <span className="select__option-label">{option.label}</span>
                    {option.value === value ? (
                      <Check className="select__check" size={15} aria-hidden="true" />
                    ) : null}
                  </button>
                </li>
              ))}
            </ul>,
            document.body,
          )
        : null}
    </div>
  );
}

/* ------------------------------------------------------------------- cards */

export function Card({
  title,
  actions,
  children,
  footer,
  flush = false,
  className = "",
}: {
  title?: ReactNode;
  actions?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
  flush?: boolean;
  className?: string;
}) {
  return (
    <section className={`card ${className}`}>
      {title || actions ? (
        <header className="card__header">
          {title ? <h2 className="card__title">{title}</h2> : null}
          {actions ? <div className="card__actions">{actions}</div> : null}
        </header>
      ) : null}
      <div className={flush ? "card__body card__body--flush" : "card__body"}>{children}</div>
      {footer ? <footer className="card__footer">{footer}</footer> : null}
    </section>
  );
}

/* ------------------------------------------------------------------ alerts */

export function Alert({
  kind = "info",
  children,
}: {
  kind?: "info" | "error" | "success" | "warning";
  children: ReactNode;
}) {
  return (
    <div className={`alert alert--${kind}`} role={kind === "error" ? "alert" : undefined}>
      {children}
    </div>
  );
}

/* ------------------------------------------------------------------ states */

export function EmptyState({
  title,
  description,
  action,
}: {
  title: string;
  description?: string;
  action?: ReactNode;
}) {
  return (
    <div className="empty">
      <div className="empty__title">{title}</div>
      {description ? <div>{description}</div> : null}
      {action}
    </div>
  );
}

export function Loading({ label = "جارٍ التحميل…" }: { label?: string }) {
  return (
    <div className="empty">
      <span className="spinner" aria-hidden="true" />
      <div>{label}</div>
    </div>
  );
}

/* ------------------------------------------------------------------- modal */

interface ModalProps {
  open: boolean;
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  size?: "narrow" | "default" | "wide";
}

export function Modal({ open, title, onClose, children, footer, size = "default" }: ModalProps) {
  const panelRef = useRef<HTMLDivElement>(null);

  // `onClose` is usually an inline arrow, so it changes identity on every
  // render. Keeping it in a ref lets the effect below depend on `open` alone.
  const closeRef = useRef(onClose);
  closeRef.current = onClose;

  // Tracks whether the dialog has already been opened, so the panel is only
  // focused once per opening.
  const openedRef = useRef(false);

  useEffect(() => {
    if (!open) {
      openedRef.current = false;
      return;
    }
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") closeRef.current();
    };
    window.addEventListener("keydown", onKey);

    if (!openedRef.current) {
      openedRef.current = true;
      const panel = panelRef.current;
      // Only take the focus when nothing inside the dialog claimed it (an
      // input with autoFocus, for instance). Focusing the panel on every
      // render would pull the caret out of the field the user is typing in.
      if (panel && !panel.contains(document.activeElement)) {
        panel.focus();
      }
    }

    return () => window.removeEventListener("keydown", onKey);
  }, [open]);

  if (!open) return null;

  return (
    <div
      className="modal-backdrop"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        className={`modal ${size === "wide" ? "modal--wide" : ""} ${size === "narrow" ? "modal--narrow" : ""}`}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
        ref={panelRef}
      >
        <header className="modal__header">
          <h2 className="modal__title">{title}</h2>
          <div className="spacer" />
          <IconButton label="إغلاق" onClick={onClose}>
            ✕
          </IconButton>
        </header>
        <div className="modal__body">{children}</div>
        {footer ? <footer className="modal__footer">{footer}</footer> : null}
      </div>
    </div>
  );
}

/* ------------------------------------------------------------------ toasts */

export function ToastStack() {
  const toasts = useToasts();
  if (toasts.length === 0) return null;
  return (
    <div className="toast-stack" aria-live="polite">
      {toasts.map((item) => (
        <div
          key={item.id}
          className={`toast ${item.kind !== "info" ? `toast--${item.kind}` : ""}`}
          onClick={() => dismissToast(item.id)}
        >
          {item.message}
        </div>
      ))}
    </div>
  );
}

/* --------------------------------------------------------------- pagination */

export function Pagination({
  page,
  pageSize,
  total,
  onPage,
}: {
  page: number;
  pageSize: number;
  total: number;
  onPage: (page: number) => void;
}) {
  const pages = Math.max(1, Math.ceil(total / Math.max(1, pageSize)));
  if (total === 0) return null;
  return (
    <div className="pagination">
      <span>
        صفحة {page} من {pages} — {total} عنصر
      </span>
      <Button size="sm" disabled={page <= 1} onClick={() => onPage(page - 1)}>
        السابق
      </Button>
      <Button size="sm" disabled={page >= pages} onClick={() => onPage(page + 1)}>
        التالي
      </Button>
    </div>
  );
}

/* ------------------------------------------------------------------ badges */

export function Badge({
  kind = "default",
  children,
}: {
  kind?: "default" | "brand" | "success" | "danger" | "warning";
  children: ReactNode;
}) {
  return <span className={`badge ${kind !== "default" ? `badge--${kind}` : ""}`}>{children}</span>;
}
