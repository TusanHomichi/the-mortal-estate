import fs from "node:fs";
import path from "node:path";
import { defineConfig, type Plugin } from "vite";

const ASSET_PREFIX = "/feel-assets/";

function feelAssetsPlugin(): Plugin {
  const configured = process.env.TME_FEEL_ASSETS?.trim() ?? "";
  const repositoryRoot = fs.realpathSync(path.resolve(import.meta.dirname, ".."));
  let usableRoot: string | null = null;
  if (configured && path.isAbsolute(configured) && fs.existsSync(configured)) {
    const candidateRoot = fs.realpathSync(configured);
    if (
      fs.statSync(candidateRoot).isDirectory() &&
      candidateRoot !== repositoryRoot &&
      !candidateRoot.startsWith(`${repositoryRoot}${path.sep}`)
    ) {
      usableRoot = candidateRoot;
    }
  }

  return {
    name: "tme-private-feel-assets",
    configureServer(server) {
      server.middlewares.use((request, response, next) => {
        const requestPath = (request.url ?? "/").split("?", 1)[0]!;
        if (!requestPath.startsWith(ASSET_PREFIX)) {
          next();
          return;
        }
        if (request.method !== "GET" && request.method !== "HEAD") {
          response.statusCode = 405;
          response.setHeader("Allow", "GET, HEAD");
          response.end("method not allowed");
          return;
        }
        if (usableRoot === null) {
          response.statusCode = 404;
          response.setHeader("Content-Type", "text/plain; charset=utf-8");
          response.end("candidate feel assets are unavailable");
          return;
        }

        let relativePath: string;
        try {
          relativePath = decodeURIComponent(requestPath.slice(ASSET_PREFIX.length));
        } catch {
          response.statusCode = 400;
          response.end("invalid asset path");
          return;
        }
        const resolved = path.resolve(usableRoot, relativePath);
        if (
          relativePath.length === 0 ||
          resolved === usableRoot ||
          !resolved.startsWith(`${usableRoot}${path.sep}`) ||
          !fs.existsSync(resolved) ||
          !fs.statSync(resolved).isFile()
        ) {
          response.statusCode = 404;
          response.end("candidate feel asset not found");
          return;
        }
        const extension = path.extname(resolved).toLowerCase();
        const contentType = extension === ".json" ? "application/json" : "image/png";
        response.statusCode = 200;
        response.setHeader("Content-Type", contentType);
        response.setHeader("Cache-Control", "no-store");
        if (request.method === "HEAD") {
          response.end();
          return;
        }
        fs.createReadStream(resolved).pipe(response);
      });
    },
  };
}

export default defineConfig({
  plugins: [feelAssetsPlugin()],
  server: {
    host: "127.0.0.1",
  },
  preview: {
    host: "127.0.0.1",
  },
});
