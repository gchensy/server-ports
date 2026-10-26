import type { ErrorInfo, TrayLabels } from "../api/types";
import { readPref, writePref } from "../state/prefs";
import { de } from "./de";
import { en, type Catalog, type MessageKey, type PluralKey } from "./en";
import { es } from "./es";
import { fr } from "./fr";
import { ja } from "./ja";
import { ko } from "./ko";
import { ru } from "./ru";
import { zhCN } from "./zh-CN";
import { zhTW } from "./zh-TW";

/** What each language calls itself, in the order the picker lists them. */
export const LANGUAGE_NAMES = {
  en: "English",
  "zh-CN": "简体中文",
  "zh-TW": "繁體中文",
  ja: "日本語",
  ko: "한국어",
  es: "Español",
  fr: "Français",
  de: "Deutsch",
  ru: "Русский",
} as const;

export type Locale = keyof typeof LANGUAGE_NAMES;
export type Language = "system" | Locale;
type Vars = Record<string, string | number>;

export const LOCALES = Object.keys(LANGUAGE_NAMES) as Locale[];
const catalogs: Record<Locale, Catalog> = { en, "zh-CN": zhCN, "zh-TW": zhTW, ja, ko, es, fr, de, ru };
const languages: readonly Language[] = ["system", ...LOCALES];

/** The supported locale closest to a language tag such as "de-AT" or "zh-Hant-HK", if any. */
export function matchLocale(tag: string): Locale | null {
  const [language, ...rest] = tag.toLowerCase().split("-");
  if (language !== "zh") return LOCALES.find((locale) => locale === language) ?? null;
  const script = rest.find((part) => part === "hans" || part === "hant");
  const traditional = script ? script === "hant" : rest.some((part) => ["tw", "hk", "mo"].includes(part));
  return traditional ? "zh-TW" : "zh-CN";
}

function systemLocale(): Locale {
  const tags = typeof navigator === "undefined" ? [] : navigator.languages;
  return tags.map(matchLocale).find((locale) => locale !== null) ?? "en";
}

function fill(text: string, vars?: Vars): string {
  return vars ? text.replace(/\{(\w+)\}/g, (_, name) => String(vars[name] ?? `{${name}}`)) : text;
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

  readonly t = (key: MessageKey, vars?: Vars): string => fill(catalogs[this.locale][key], vars);

  /** "1 port" / "2 ports", by the plural rules of the language. */
  readonly plural = (key: PluralKey, n: number): string => {
    const forms: Partial<Record<string, string>> = catalogs[this.locale];
    const form = forms[`${key}.${new Intl.PluralRules(this.locale).select(n)}`] ?? forms[`${key}.other`];
    return fill(form ?? key, { n });
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

  /** "02:58:56" in every language (some would drop the leading zero). */
  readonly clock = (ts: number): string =>
    new Date(ts).toLocaleTimeString(this.locale, {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
      hourCycle: "h23",
    });

  /** "3h 5m", "4m 12s", "9s". */
  readonly duration = (ms: number): string => {
    const total = Math.max(0, Math.floor(ms / 1000));
    const [d, h, m, s] = [
      Math.floor(total / 86400),
      Math.floor((total % 86400) / 3600),
      Math.floor((total % 3600) / 60),
      total % 60,
    ];
    const part = (n: number, unit: "d" | "h" | "m" | "s") => this.t(`duration.${unit}`, { n });
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

export type { MessageKey, PluralKey };
