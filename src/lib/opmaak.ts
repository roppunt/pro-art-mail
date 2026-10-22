// Datums, namen en mappen leesbaar maken.
import type { Map, Persoon } from "./api";

const MAANDEN = ["januari", "februari", "maart", "april", "mei", "juni", "juli", "augustus", "september", "oktober", "november", "december"];

/** "14:05" vandaag, "gisteren", "ma 19 okt" deze week of eerder dit jaar, anders "19-10-2025". */
export function korteDatum(sec: number): string {
  if (!sec) return "";
  const d = new Date(sec * 1000);
  const nu = new Date();
  const dag = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const verschil = Math.round((dag(nu) - dag(d)) / 86400000);
  if (verschil === 0) return d.toLocaleTimeString("nl-NL", { hour: "2-digit", minute: "2-digit" });
  if (verschil === 1) return "gisteren";
  if (d.getFullYear() === nu.getFullYear()) return d.toLocaleDateString("nl-NL", { weekday: "short", day: "numeric", month: "short" });
  return d.toLocaleDateString("nl-NL");
}

export function langeDatum(sec: number): string {
  if (!sec) return "";
  return new Date(sec * 1000).toLocaleString("nl-NL", { weekday: "long", day: "numeric", month: "long", year: "numeric", hour: "2-digit", minute: "2-digit" });
}

/** Kopje in de lijst: Vandaag, Gisteren, Eerder deze week, of de maand. */
export function groep(sec: number): string {
  if (!sec) return "Zonder datum";
  const d = new Date(sec * 1000);
  const nu = new Date();
  const begin = new Date(nu.getFullYear(), nu.getMonth(), nu.getDate()).getTime();
  if (d.getTime() >= begin) return "Vandaag";
  if (d.getTime() >= begin - 86400000) return "Gisteren";
  if (d.getTime() >= begin - 6 * 86400000) return "Eerder deze week";
  const m = MAANDEN[d.getMonth()];
  return (m[0].toUpperCase() + m.slice(1)) + (d.getFullYear() === nu.getFullYear() ? "" : ` ${d.getFullYear()}`);
}

export const naamVan = (p: { naam?: string; adres?: string; van_naam?: string; van_adres?: string }) =>
  p.naam || p.van_naam || p.adres || p.van_adres || "(onbekend)";

export const personen = (l: Persoon[]) => l.map((p) => (p.naam ? `${p.naam} <${p.adres}>` : p.adres)).join(", ");

export function grootte(b: number): string {
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${Math.round(b / 1024)} kB`;
  return `${(b / 1024 / 1024).toFixed(1).replace(".", ",")} MB`;
}

/** Jaararchief herkennen (`Archives.2021`, `Archief/2023`): geeft het jaar of null. */
export function jaarVanMap(m: Map): number | null {
  const r = m.naam.match(/^(archives?|archief)[./](\d{4})$/i);
  return r ? Number(r[2]) : null;
}

const VOLGORDE = ["inbox", "concepten", "verzonden", "archief", "spam", "prullenbak"];

/** Verdeelt de mappen in vaste mappen, jaararchieven (nieuwste eerst) en overige. */
export function indeling(lijst: Map[]) {
  const vast = lijst.filter((m) => m.soort !== "gewoon").sort((a, b) => VOLGORDE.indexOf(a.soort) - VOLGORDE.indexOf(b.soort));
  const gewoon = lijst.filter((m) => m.soort === "gewoon");
  const jaren = gewoon
    .filter((m) => jaarVanMap(m))
    .sort((a, b) => jaarVanMap(b)! - jaarVanMap(a)!)
    .map((m) => ({ ...m, naam: String(jaarVanMap(m)) }));
  const overig = gewoon.filter((m) => !jaarVanMap(m)).sort((a, b) => a.naam.localeCompare(b.naam, "nl"));
  return { vast, jaren, overig };
}

export const initiaal = (s: string) => (s.replace(/[^\p{L}\p{N}]/gu, "")[0] || "?").toUpperCase();
