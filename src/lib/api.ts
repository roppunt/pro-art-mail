// Aanroepen naar de Rust-kant (src-tauri/src/lib.rs). Fouten komen als leesbare tekst terug.
import { invoke } from "@tauri-apps/api/core";

export type Account = {
  email: string;
  naam: string;
  imap_host: string;
  imap_poort: number;
  smtp_host: string;
  smtp_poort: number;
  verzonden_map: string;
};

export type Map = {
  pad: string;
  naam: string;
  soort: "inbox" | "concepten" | "verzonden" | "spam" | "prullenbak" | "archief" | "gewoon";
  scheiding: string;
  aantal: number;
  ongelezen: number;
};

export type Kop = {
  map: string;
  uid: number;
  van_naam: string;
  van_adres: string;
  onderwerp: string;
  datum: number;
  gelezen: boolean;
  beantwoord: boolean;
  grootte: number;
  bijlage: boolean;
};

export type Pagina = { totaal: number; klaar: boolean; berichten: Kop[] };
export type Voortgang = { account: string; map: string; gedaan: number; totaal: number };
export type Gebeurtenis = { account: string; map: string };

export type Persoon = { naam: string; adres: string };
export type Bijlage = { naam: string; grootte: number; soort: string };
export type Bericht = {
  uid: number;
  onderwerp: string;
  van: Persoon[];
  antwoord_aan: Persoon[];
  message_id: string;
  referenties: string;
  aan: Persoon[];
  cc: Persoon[];
  datum: number;
  html: string;
  tekst: string;
  bijlagen: Bijlage[];
};

export type Verwijzing = { map: string; uid: number };
export type Concept = {
  account: string;
  aan: string[];
  cc: string[];
  bcc: string[];
  onderwerp: string;
  html: string;
  tekst: string;
  bestanden: string[];
  bijlagen_van: Verwijzing | null;
  zonder_bijlagen: string[];
  antwoord_op: Verwijzing | null;
  in_reply_to: string | null;
  references: string | null;
  message_id: string;
};

export type Ondertekening = { naam: string; tekst: string };

// Accounts
export const accounts = () => invoke<Account[]>("accounts");
export const accountStandaard = (email: string) => invoke<Account>("account_standaard", { email });
export const accountBewaren = (acc: Account, wachtwoord: string) => invoke<Account[]>("account_bewaren", { acc, wachtwoord });
export const accountVerwijderen = (email: string) => invoke<Account[]>("account_verwijderen", { email });

// Lezen
export const mappen = (account: string) => invoke<Map[]>("mappen", { account });
export const berichten = (account: string, map: string, vanaf: number, aantal: number) =>
  invoke<Pagina>("berichten", { account, map, vanaf, aantal });
export const zoek = (account: string, zoekterm: string) => invoke<Kop[]>("zoek", { account, zoekterm });
export const zoekInTekst = (account: string, map: string, zoekterm: string) => invoke<Kop[]>("zoek_in_tekst", { account, map, zoekterm });
export const adressen = (account: string, begin: string) => invoke<string[]>("adressen", { account, begin });
export const bericht = (account: string, map: string, uid: number, markeer: boolean) =>
  invoke<Bericht>("bericht", { account, map, uid, markeer });

// Ordenen
export const markeer = (account: string, map: string, uids: number[], gelezen: boolean) => invoke<void>("markeer", { account, map, uids, gelezen });
export const verwijder = (account: string, map: string, uids: number[]) => invoke<void>("verwijder", { account, map, uids });
export const archiveer = (account: string, map: string, berichten: [number, number][]) => invoke<void>("archiveer", { account, map, berichten });
export const verplaats = (account: string, map: string, uids: number[], doel: string) => invoke<void>("verplaats", { account, map, uids, doel });

// Bijlagen
export const bijlageOpenen = (account: string, map: string, uid: number, index: number) => invoke<void>("bijlage_openen", { account, map, uid, index });
export const bijlageOpslaan = (account: string, map: string, uid: number, index: number, pad: string) =>
  invoke<void>("bijlage_opslaan", { account, map, uid, index, pad });

// Schrijven
export const verstuur = (concept: Concept) => invoke<void>("verstuur", { concept });
export const bewaarConcept = (concept: Concept) => invoke<void>("bewaar_concept", { concept });
export const ondertekeningen = () => invoke<Ondertekening[]>("ondertekeningen");
export const bewaarOndertekeningen = (lijst: Ondertekening[]) => invoke<void>("bewaar_ondertekeningen", { lijst });
