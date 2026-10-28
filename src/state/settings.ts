/** Global application settings, loaded once and shared by every screen. */
import { create } from "zustand";

import * as api from "../lib/api";
import type { AppSettings } from "../lib/types";

interface SettingsState {
  settings: AppSettings | null;
  loading: boolean;
  error: string | null;
  load: () => Promise<void>;
  save: (settings: AppSettings) => Promise<AppSettings>;
}

export const useSettings = create<SettingsState>((set) => ({
  settings: null,
  loading: false,
  error: null,
  load: async () => {
    set({ loading: true, error: null });
    try {
      const settings = await api.getSettings();
      set({ settings, loading: false });
    } catch (error) {
      set({ error: api.errorMessage(error), loading: false });
    }
  },
  save: async (settings: AppSettings) => {
    const saved = await api.saveSettings(settings);
    set({ settings: saved });
    return saved;
  },
}));

/** The currency exponent currently in use. */
export function currentExponent(settings: AppSettings | null): number {
  return settings?.currencyExponent ?? 2;
}
