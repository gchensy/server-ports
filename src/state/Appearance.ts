import { readPref, writePref } from "./prefs";

export type Theme = "system" | "light" | "dark";
const themes: Theme[] = ["system", "light", "dark"];

/** Light or dark: follows the system unless the user picked one. */
export class Appearance {
  private choice: Theme;
  private version = 0;
  private readonly listeners = new Set<() => void>();

  constructor() {
    const saved = readPref("theme");
    this.choice = themes.find((t) => t === saved) ?? "system";
    this.apply();
  }

  readonly subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };
  readonly getVersion = (): number => this.version;

  get theme(): Theme {
    return this.choice;
  }

  setTheme(theme: Theme): void {
    this.choice = theme;
    writePref("theme", theme === "system" ? null : theme);
    this.apply();
    this.version++;
    this.listeners.forEach((listener) => listener());
  }

  private apply(): void {
    const root = document.documentElement;
    if (this.choice === "system") root.removeAttribute("data-theme");
    else root.setAttribute("data-theme", this.choice);
  }
}
