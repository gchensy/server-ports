import type { ErrorInfo, TrayLabels } from "../api/types";
import { readPref, writePref } from "../state/prefs";
import { en, type MessageKey } from "./en";
import { zhCN } from "./zh-CN";

export type Locale = "en" | "zh-CN";
export type Language = "system" | Locale;
type Vars = Record<string, string | number>;
/** `x` for which both `x.one` and `x.other` exist. */
export type PluralKey = Extract<MessageKey, `${string}.one`> extends `${infer K}.one` ? K : never;

const catalogs: Record<Locale, Record<MessageKey, string>> = { en, "zh-CN": zhCN };
const languages: Language[] = ["system", "en", "zh-CN"];

function systemLocale(): Locale {
  const wanted = typeof navigator === "undefined" ? "en" : navigator.language;
  return wanted.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
}

/** The language of the UI, and everything that has to be said in it. */
export class I18n {
  private preference: Language;
  private version = 0;
  private readonly listeners = new Set<() => void>();

  constructor() {
    const saved = readPref("language");
    this.preference = languages.find((l) => l === saved) ?? "system";
  }

  readonly subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };
  /** Changes whenever the language does: what a component re-renders on. */
  readonly getVersion = (): number => this.version;

  get language(): Language {
    return this.preference;
  }

  get locale(): Locale {
    return this.preference === "system" ? systemLocale() : this.preference;
  }

  setLanguage(language: Language): void {
    this.preference = language;
    writePref("language", language === "system" ? null : language);
    this.version++;
    document.documentElement.lang = this.locale;
    this.listeners.forEach((listener) => listener());
  }

  readonly t = (key: MessageKey, vars?: Vars): string => {
    const text = catalogs[this.locale][key];
    return vars ? text.replace(/\{(\w+)\}/g, (_, name) => String(vars[name] ?? `{${name}}`)) : text;
  };

  /** "1 port" / "2 ports", by the plural rules of the language. */
  readonly plural = (key: PluralKey, n: number): string => {
    const form = new Intl.PluralRules(this.locale).select(n) === "one" ? "one" : "other";
    return this.t(`${key}.${form}` as MessageKey, { n });
  };

  /** The headline for an error code; unknown codes get a generic one. */
  readonly errorTitle = (code: string): string =>
    this.t((`error.${code}.title` in en ? `error.${code}.title` : "error.unknown.title") as MessageKey);

  /** The sentence under the headline, if there is one. */
  readonly errorHint = (code: string): string | null =>
    `error.${code}.hint` in en ? this.t(`error.${code}.hint` as MessageKey) : null;

  readonly describe = (error: ErrorInfo) => ({
    title: this.errorTitle(error.code),
    hint: this.errorHint(error.code),
  });

  readonly clock = (ts: number): string =>
    new Date(ts).toLocaleTimeString(this.locale, { hour12: false });

  /** "3h 5m", "4m 12s", "9s". */
  readonly duration = (ms: number): string => {
    const total = Math.max(0, Math.floor(ms / 1000));
    const [d, h, m, s] = [
      Math.floor(total / 86400),
      Math.floor((total % 86400) / 3600),
      Math.floor((total % 3600) / 60),
      total % 60,
    ];
    const part = (n: number, unit: "d" | "h" | "m" | "s") => `${n}${this.t(`unit.${unit}`)}`;
    if (d > 0) return [part(d, "d"), part(h, "h")].join(" ");
    if (h > 0) return [part(h, "h"), part(m, "m")].join(" ");
    if (m > 0) return [part(m, "m"), part(s, "s")].join(" ");
    return part(s, "s");
  };

  trayLabels(): TrayLabels {
    return {
      show: this.t("tray.show"),
      noProfiles: this.t("tray.noProfiles"),
      connectAll: this.t("tray.connectAll"),
      disconnectAll: this.t("tray.disconnectAll"),
      quit: this.t("tray.quit"),
      tooltip: this.t("tray.tooltip"),
    };
  }
}

export type { MessageKey };
