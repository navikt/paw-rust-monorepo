// Henter nytt innhold til elementer med data-oppdater med fast intervall.
// Siden virker uten dette skriptet; det gir bare automatisk oppdatering.
"use strict";
(() => {
  const elementer = () => document.querySelectorAll("[data-oppdater]");

  const oppdater = async (el) => {
    if (document.hidden) return;
    try {
      const svar = await fetch(el.dataset.oppdater, {
        credentials: "same-origin",
        redirect: "manual",
      });
      // Utløpt sesjon: last siden på nytt, så Wonderwall logger inn igjen.
      if (svar.status === 401 || svar.type === "opaqueredirect") {
        location.reload();
        return;
      }
      if (svar.ok) el.innerHTML = await svar.text();
    } catch {
      // Nettverksfeil: prøv igjen ved neste intervall.
    }
  };

  for (const el of elementer()) {
    const sekunder = Math.max(Number(el.dataset.intervall) || 30, 10);
    setInterval(() => oppdater(el), sekunder * 1000);
  }
  document.addEventListener("visibilitychange", () => {
    if (!document.hidden) elementer().forEach(oppdater);
  });
})();
