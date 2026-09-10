/** Original menu emblems and concise descriptions, separate from rules-owned sheets. */
const themes: Record<string, { path: string; description: string }> = {
  fighter: { path: "M12 3h8v15l-4 5-4-5ZM6 18h20M16 23v7M12 30h8", description: "Weapons, armour and the discipline of close combat." },
  martial_artist: { path: "M8 26V14h4V8h4v5h4V9h4v12l-6 8h-7ZM12 14v7M16 13v7M20 13v7", description: "Unarmed fighting, precise strikes and practiced defence." },
  thief: { path: "M7 27 23 5l4 4L11 31ZM5 22l9 7M20 9l4 3M4 4h7v7H4Z", description: "Stealth, nimble weapons and a private school of magic." },
  wizard: { path: "M6 29 22 9M18 3h8v8h-8ZM22 1v2M28 7h3M3 15h6M6 12v6M19 24h8M23 20v8", description: "Arcane study, spells and the careful application of power." },
  thaumaturge: { path: "M13 3h6v8h8v6h-8v12h-6V17H5v-6h8ZM5 26h4M23 26h4", description: "Priestly magic, restoration and strength of will." },
};

export function creationTheme(profile: string) {
  return themes[profile.split("/").at(-1)!] ?? { path: "M16 3 29 16 16 29 3 16Z", description: "Choose your path into the estate." };
}
export function classEmblem(profile: string): SVGSVGElement {
  const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  svg.setAttribute("viewBox", "0 0 32 32"); svg.setAttribute("aria-hidden", "true"); svg.classList.add("class-emblem");
  const path = document.createElementNS(svg.namespaceURI, "path");
  path.setAttribute("d", creationTheme(profile).path); svg.append(path); return svg;
}
