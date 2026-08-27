import { Resvg } from "@resvg/resvg-js";
import type { APIRoute } from "astro";
import { root } from "astro:config/server";
import { fileURLToPath } from "node:url";

const fontFile = fileURLToPath(new URL("src/assets/fonts/PirataOne-Regular.ttf", root));

const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630">
  <rect width="1200" height="630" fill="#0b0a0a"/>
  <rect x="24" y="24" width="1152" height="582" fill="none" stroke="#ece7de" stroke-opacity="0.16"/>
  <path d="M24 24h14v1H25v13h-1z M1176 24v14h-1V25h-13v-1z M24 606v-14h1v13h13v1z M1176 606h-14v-1h13v-13h1z" fill="#e23127"/>
  <text x="600" y="345" text-anchor="middle" font-family="Pirata One" font-size="280" fill="#e23127">blasphem</text>
  <text x="600" y="455" text-anchor="middle" font-family="Pirata One" font-size="38" fill="#ece7de">hostile messages, judged in the browser before they send</text>
  <text x="600" y="560" text-anchor="middle" font-family="Pirata One" font-size="24" fill="#8a7f70">fifteen languages, deterministic, no request after load</text>
</svg>`;

export const GET: APIRoute = () => {
  const renderer = new Resvg(svg, {
    fitTo: { mode: "width", value: 1200 },
    font: { loadSystemFonts: false, fontFiles: [fontFile], defaultFontFamily: "Pirata One" },
  });
  const png = new Uint8Array(renderer.render().asPng());
  return new Response(png, { headers: { "Content-Type": "image/png" } });
};
