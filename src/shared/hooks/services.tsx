// Service lookup for components: the app registers its service instances once (app/boot) and
// components ask for one by its class, so features never import the app or each other's
// internals.
import { createContext, useContext, useSyncExternalStore, type ReactNode } from 'react';
import type { Store } from '../lib/Store.ts';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type Constructor<T> = abstract new (...args: any[]) => T;

export class ServiceRegistry {
  private readonly services = new Map<Constructor<unknown>, unknown>();

  add<T>(cls: Constructor<T>, instance: T): T {
    this.services.set(cls, instance);
    return instance;
  }

  get<T>(cls: Constructor<T>): T {
    const service = this.services.get(cls);
    if (!service) throw new Error(`Service ${cls.name} is not registered`);
    return service as T;
  }
}

const ServicesContext = createContext<ServiceRegistry | null>(null);

export function ServicesProvider({ registry, children }: { registry: ServiceRegistry; children: ReactNode }) {
  return <ServicesContext.Provider value={registry}>{children}</ServicesContext.Provider>;
}

export function useService<T>(cls: Constructor<T>): T {
  const registry = useContext(ServicesContext);
  if (!registry) throw new Error('useService outside ServicesProvider');
  return registry.get(cls);
}

// The current state of a store; the component re-renders when it changes.
export function useStore<T extends object>(store: Store<T>): T {
  return useSyncExternalStore(store.subscribe, store.get);
}
