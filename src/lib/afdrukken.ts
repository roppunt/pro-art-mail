// Drukt één bericht af: kopregels (van, aan, datum, onderwerp) + inhoud, zonder de rest van de app.
// Het vel staat als #afdruk direct in <body> en is alleen bij het afdrukken zichtbaar; de mailinhoud
// zit weer in een shadow root, zodat de opmaak van een nieuwsbrief niets van de app overneemt.
import type { Bericht } from "./api";
import { langeDatum, personen } from "./opmaak";

const STIJL = `
#afdruk { display: none; }
@media print {
  html, body { height: auto !important; overflow: visible !important; background: #fff !important; }
  body > :not(#afdruk) { display: none !important; }
  #afdruk { display: block; color: #000; font: 11pt/1.5 system-ui, sans-serif; }
  #afdruk h1 { font-size: 15pt; margin: 0 0 8pt; }
  #afdruk table { border-collapse: collapse; margin-bottom: 10pt; }
  #afdruk th { text-align: left; font-weight: 600; padding: 1pt 10pt 1pt 0; vertical-align: top; white-space: nowrap; }
  #afdruk td { padding: 1pt 0; overflow-wrap: anywhere; }
  #afdruk hr { border: 0; border-top: 1px solid #999; margin: 0 0 12pt; }
}`;

function vel(): HTMLDivElement {
  let el = document.getElementById("afdruk") as HTMLDivElement | null;
  if (!el) {
    const stijl = document.createElement("style");
    stijl.textContent = STIJL;
    document.head.appendChild(stijl);
    el = document.createElement("div");
    el.id = "afdruk";
    document.body.appendChild(el);
  }
  return el;
}

/** Wacht tot de afbeeldingen geladen zijn (hooguit 3 s), anders komen ze leeg op papier. */
function afbeeldingenKlaar(wortel: ShadowRoot): Promise<unknown> {
  const wachten = [...wortel.querySelectorAll("img")]
    .filter((img) => !img.complete)
    .map((img) => new Promise((klaar) => { img.onload = img.onerror = klaar; }));
  return Promise.race([Promise.all(wachten), new Promise((klaar) => setTimeout(klaar, 3000))]);
}

export async function afdrukken(b: Bericht) {
  const el = vel();
  el.replaceChildren();

  const h1 = document.createElement("h1");
  h1.textContent = b.onderwerp || "(geen onderwerp)";
  const tabel = document.createElement("table");
  const regels: [string, string][] = [["Van", personen(b.van)], ["Aan", personen(b.aan)]];
  if (b.cc.length) regels.push(["Cc", personen(b.cc)]);
  regels.push(["Datum", langeDatum(b.datum)]);
  if (b.bijlagen.length) regels.push(["Bijlagen", b.bijlagen.map((x) => x.naam).join(", ")]);
  for (const [kop, waarde] of regels) {
    const tr = tabel.insertRow();
    const th = document.createElement("th");
    th.textContent = kop;
    tr.appendChild(th);
    tr.insertCell().textContent = waarde;
  }
  const inhoud = document.createElement("div");
  el.append(h1, tabel, document.createElement("hr"), inhoud);

  const wortel = inhoud.attachShadow({ mode: "open" });
  wortel.innerHTML = `<style>:host{display:block;color:#000;font:11pt/1.5 system-ui,sans-serif;overflow-wrap:anywhere}
    img{max-width:100%;height:auto} table{max-width:100%} pre{white-space:pre-wrap;font:inherit;margin:0}</style>`;
  if (b.html) {
    wortel.innerHTML += b.html;
  } else {
    const pre = document.createElement("pre");
    pre.textContent = b.tekst; // als tekst, nooit als HTML
    wortel.appendChild(pre);
  }

  await afbeeldingenKlaar(wortel);
  window.print();
}
