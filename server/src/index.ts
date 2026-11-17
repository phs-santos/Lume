import { serve } from "@hono/node-server";
import { serveStatic } from "@hono/node-server/serve-static";
import { mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { createApp } from "./app.js";
import { openDb } from "./db.js";

const invite = process.env.INVITE_CODE;
if (!invite || invite.length < 12) {
  console.error("Defina INVITE_CODE (12+ caracteres) no .env antes de subir o servidor do Lume.");
  process.exit(1);
}
const port = Number(process.env.PORT ?? 3020);
const dbPath = process.env.DB_PATH ?? "./data/lume.db";
const site = resolve(process.env.SITE_DIR ?? "../capture/dist");
mkdirSync(dirname(dbPath), { recursive: true });

const app = createApp(openDb(dbPath), { inviteCode: invite });
app.use("/*", serveStatic({ root: site }));
app.get("*", serveStatic({ path: `${site}/index.html` })); // app de uma página

serve({ fetch: app.fetch, port, hostname: process.env.HOST ?? "127.0.0.1" }, () =>
  console.log(`Lume ouvindo em ${port} (site: ${site}, banco: ${dbPath})`),
);
