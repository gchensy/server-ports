import { Check } from "lucide-react";
import { LANGUAGE_NAMES, LOCALES, type Language } from "../i18n";
import { cx } from "../util/cx";

/**
 * Every language under its own name, so that it can be found whatever language the UI is in now.
 * "System default" comes first, in the current language.
 */
export function LanguagePicker({
  value,
  label,
  systemLabel,
  onChange,
}: {
  value: Language;
  label: string;
  systemLabel: string;
  onChange: (language: Language) => void;
}) {
  const options: { value: Language; label: string }[] = [
    { value: "system", label: systemLabel },
    ...LOCALES.map((locale) => ({ value: locale, label: LANGUAGE_NAMES[locale] })),
  ];
  return (
    <div className="language-picker" role="radiogroup" aria-label={label}>
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          role="radio"
          aria-checked={option.value === value}
          // The name is in its own language, which also picks the right CJK glyphs.
          lang={option.value === "system" ? undefined : option.value}
          className={cx(
            "language-option",
            option.value === "system" && "language-option--system",
            option.value === value && "is-on",
          )}
          onClick={() => onChange(option.value)}
        >
          <span>{option.label}</span>
          {option.value === value && <Check size={14} aria-hidden="true" />}
        </button>
      ))}
    </div>
  );
}
