# Contexto de build: a raiz do repositório (a página do celular reaproveita a cena do app).
FROM node:24-slim AS capture
WORKDIR /app
COPY src/components/DeskScene.vue src/components/
COPY src/lib/timeOfDay.ts src/lib/
COPY test-fixtures test-fixtures
COPY capture/package.json capture/package-lock.json capture/
RUN cd capture && npm ci --no-audit --no-fund
COPY capture capture
RUN cd capture && npx vite build

FROM node:24-slim AS server
WORKDIR /app/server
COPY server/package.json server/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY server/tsconfig.json ./
COPY server/src src
RUN npx tsc -p . && npm prune --omit=dev

FROM node:24-slim
ENV NODE_ENV=production PORT=3020 HOST=0.0.0.0 DB_PATH=/data/lume.db SITE_DIR=/app/site
WORKDIR /app/server
COPY --from=server /app/server/node_modules node_modules
COPY --from=server /app/server/dist dist
COPY --from=server /app/server/package.json .
COPY --from=capture /app/capture/dist /app/site
RUN mkdir -p /data && chown node:node /data
USER node
EXPOSE 3020
HEALTHCHECK --interval=30s --timeout=5s CMD node -e "fetch('http://127.0.0.1:3020/api/health').then(r=>process.exit(r.ok?0:1)).catch(()=>process.exit(1))"
CMD ["node", "dist/index.js"]
