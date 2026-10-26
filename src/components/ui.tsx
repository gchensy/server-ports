import { motion } from "motion/react";
import { useId, type ButtonHTMLAttributes, type ReactNode } from "react";
import { cx } from "../util/cx";
import { spring } from "./motion";

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: "primary" | "secondary" | "ghost" | "danger";
  size?: "md" | "sm";
  icon?: ReactNode;
};

export function Button({ variant = "secondary", size = "md", icon, className, children, ...rest }: ButtonProps) {
  return (
    <button type="button" className={cx("btn", `btn--${variant}`, size === "sm" && "btn--sm", className)} {...rest}>
      {icon}
      {children}
    </button>
  );
}

export function IconButton({
  label,
  danger,
  className,
  children,
  ...rest
}: ButtonHTMLAttributes<HTMLButtonElement> & { label: string; danger?: boolean }) {
  return (
    <button
      type="button"
      className={cx("icon-btn", danger && "icon-btn--danger", className)}
      aria-label={label}
      title={label}
      {...rest}
    >
      {children}
    </button>
  );
}

/** An on/off switch. `busy` shows that the thing it controls is on its way. */
export function Switch({
  checked,
  busy,
  label,
  onChange,
}: {
  checked: boolean;
  busy?: boolean;
  label: string;
  onChange: (checked: boolean) => void;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      title={label}
      className={cx("switch", checked && "is-on", busy && "is-busy")}
      onClick={() => onChange(!checked)}
    >
      <motion.span className="switch__knob" layout transition={spring} />
    </button>
  );
}

export function Segmented<T extends string>({
  value,
  options,
  label,
  onChange,
}: {
  value: T;
  options: { value: T; label: string; icon?: ReactNode }[];
  label: string;
  onChange: (value: T) => void;
}) {
  const id = useId();
  return (
    <div className="segmented" role="radiogroup" aria-label={label}>
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          role="radio"
          aria-checked={option.value === value}
          className={cx("segmented__option", option.value === value && "is-on")}
          onClick={() => onChange(option.value)}
        >
          {option.value === value && (
            <motion.span layoutId={`${id}-thumb`} className="segmented__thumb" transition={spring} />
          )}
          <span className="segmented__label">
            {option.icon}
            {option.label}
          </span>
        </button>
      ))}
    </div>
  );
}

export function Field({
  label,
  hint,
  error,
  htmlFor,
  className,
  children,
}: {
  label: string;
  hint?: string;
  error?: string | null;
  htmlFor?: string;
  className?: string;
  children: ReactNode;
}) {
  return (
    <div className={cx("field", className)}>
      <label htmlFor={htmlFor}>{label}</label>
      {children}
      {error ? (
        <p className="field__error" role="alert">
          {error}
        </p>
      ) : hint ? (
        <p className="field__hint">{hint}</p>
      ) : null}
    </div>
  );
}

/** A line of text with an on/off switch at its end. */
export function ToggleRow({
  label,
  hint,
  checked,
  onChange,
}: {
  label: string;
  hint?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <div className="toggle-row">
      <div className="toggle-row__text">
        <span className="toggle-row__label">{label}</span>
        {hint && <span className="toggle-row__hint">{hint}</span>}
      </div>
      <Switch checked={checked} label={label} onChange={onChange} />
    </div>
  );
}
