import { createContext, useContext, useEffect, useSyncExternalStore, type ReactNode } from "react";
import type { Backend } from "../api/backend";
import type { ProfileId } from "../api/types";
import { I18n } from "../i18n";
import { Appearance } from "./Appearance";
import { AppStore, type AppState } from "./AppStore";
import { ThroughputTracker, type Sample } from "./ThroughputTracker";

/** The long-lived objects of the UI, created once at start-up. */
export class Services {
  readonly i18n = new I18n();
  readonly appearance = new Appearance();
  readonly store: AppStore;
  readonly throughput: ThroughputTracker;

  constructor(readonly backend: Backend) {
    this.store = new AppStore(backend, this.i18n);
    this.throughput = new ThroughputTracker(this.store);
  }
}

const ServicesContext = createContext<Services | null>(null);

export function ServicesProvider({ services, children }: { services: Services; children: ReactNode }) {
  useEffect(() => {
    services.throughput.start();
    return () => services.throughput.stop();
  }, [services]);
  return <ServicesContext.Provider value={services}>{children}</ServicesContext.Provider>;
}

export function useServices(): Services {
  const services = useContext(ServicesContext);
  if (!services) throw new Error("ServicesProvider is missing");
  return services;
}

/** Re-renders when what `select` picks from the state changes (by identity). */
export function useAppState<T>(select: (state: AppState) => T): T {
  const { store } = useServices();
  return useSyncExternalStore(store.subscribe, () => select(store.getState()));
}

export function useStore(): AppStore {
  return useServices().store;
}

/** The translator; its identity changes with the language, so memoised children follow. */
export function useI18n() {
  const { i18n } = useServices();
  useSyncExternalStore(i18n.subscribe, i18n.getVersion);
  return i18n;
}

export function useAppearance(): Appearance {
  const { appearance } = useServices();
  useSyncExternalStore(appearance.subscribe, appearance.getVersion);
  return appearance;
}

export function useSamples(forwardId: string): readonly Sample[] {
  const { throughput } = useServices();
  useSyncExternalStore(throughput.subscribe, throughput.getVersion);
  return throughput.samples(forwardId);
}

export function useProfile(id: ProfileId | null) {
  return useAppState((state) => state.profiles.find((p) => p.id === id));
}
