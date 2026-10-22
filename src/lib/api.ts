// Aanroepen naar de Rust-kant (src-tauri/src/lib.rs). Fouten komen als leesbare tekst terug.
import { invoke } from "@tauri-apps/api/core";

export type Account = { email: string; imap_host: string; imap_poort: number; smtp_host: string; smtp_poort: number };

export type Map = {
  pad: string;
  naam: string;
  soort: "inbox" | "concepten" | "verzonden" | "spam" | "prullenbak" | "archief" | "gewoon";
  scheiding: string;
  aantal: number;
  ongelezen: number;
};

export type Kop = {
  uid: number;
  volgnummer: number;
  van_naam: string;
  van_adres: string;
  onderwerp: string;
  datum: number;
  gelezen: boolean;
  beantwoord: boolean;
  grootte: number;
  bijlage: boolean;
};

export type Pagina = { totaal: number; berichten: Kop[] };

export type Persoon = { naam: string; adres: string };
export type Bijlage = { naam: string; grootte: number; soort: string };
export type Bericht = {
  uid: number;
  onderwerp: string;
  van: Persoon[];
  aan: Persoon[];
  cc: Persoon[];
  datum: number;
  html: string;
  tekst: string;
  bijlagen: Bijlage[];
};

export const huidigAccount = () => invoke<Account | null>("huidig_account");
export const inloggen = (email: string, wachtwoord: string) => invoke<Account>("inloggen", { email, wachtwoord });
export const afmelden = () => invoke<void>("afmelden");
export const mappen = () => invoke<Map[]>("mappen");
export const berichten = (map: string, pagina: number) => invoke<Pagina>("berichten", { map, pagina });
export const bericht = (map: string, uid: number) => invoke<Bericht>("bericht", { map, uid });
