import { reactive } from "vue";
import { settingGet, settingSet } from "../../lib/api";

export interface Prefs {
  font: "serif" | "mono";
  theme: "papel" | "escuro";
  focus: "on" | "off";
  keysound: "on" | "off";
  machine: "on" | "off";
}

export const defaults: Prefs = { font: "serif", theme: "papel", focus: "off", keysound: "off", machine: "on" };
export const prefs = reactive<Prefs>({ ...defaults });

function apply() {
  document.documentElement.dataset.theme = prefs.theme;
  document.documentElement.dataset.font = prefs.font;
}

export async function loadPrefs() {
  for (const k of Object.keys(defaults) as (keyof Prefs)[]) {
    const v = await settingGet(k);
    if (v) (prefs as Record<string, string>)[k] = v;
  }
  apply();
}

export async function setPref<K extends keyof Prefs>(k: K, v: Prefs[K]) {
  prefs[k] = v;
  apply();
  await settingSet(k, v);
}
