// De drie thema's (ontwerpen A, B en C). De keuze wordt per pc onthouden.
export type ThemaId = "a" | "b" | "c";

export const THEMAS: { id: ThemaId; naam: string; uitleg: string }[] = [
  { id: "a", naam: "Rustig", uitleg: "Licht en overzichtelijk, drie kolommen. Lijkt het meest op een gewoon mailprogramma." },
  { id: "b", naam: "Groot en eenvoudig", uitleg: "Grote letters en knoppen, mappen als tabbladen bovenin." },
  { id: "c", naam: "Atelier", uitleg: "Donker, met kleuraccenten. Rustig voor de ogen." },
];

const SLEUTEL = "pro-art-mail.thema";

export function laadThema(): ThemaId {
  try {
    const t = localStorage.getItem(SLEUTEL);
    if (t === "a" || t === "b" || t === "c") return t;
  } catch {
    // geen opslag beschikbaar: standaardthema
  }
  return "a";
}

export function zetThema(t: ThemaId) {
  document.documentElement.dataset.thema = t;
  try {
    localStorage.setItem(SLEUTEL, t);
  } catch {
    // niet erg: dan geldt de keuze alleen tot de app sluit
  }
}
