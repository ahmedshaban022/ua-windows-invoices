/** The shared UI kit. Every screen is built from these primitives. */
import { useEffect, useRef, type ReactNode } from "react";

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
