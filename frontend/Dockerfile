FROM node:26-alpine AS base
ENV PNPM_HOME="/pnpm"
ENV PATH="$PNPM_HOME:$PATH"
# Corepack is no longer bundled with Node since 25: install it from npm, pinned.
RUN npm install --global corepack@0.36.0 && corepack enable

FROM base AS deps
WORKDIR /app
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
RUN pnpm ci

FROM base AS builder
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules
COPY . .
RUN NUXT_BACKEND_HOSTNAME=backend pnpm build

FROM node:26-alpine AS runner
WORKDIR /app
COPY --from=builder /app/.output ./.output
ENV NODE_ENV=production
ENV HOST=0.0.0.0
ENV PORT=3000
EXPOSE 3000
CMD ["node", ".output/server/index.mjs"]
