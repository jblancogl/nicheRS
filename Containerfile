# NicheRS container image: the same single executable, with the web UI embedded.
# Build: buildah bud -f Containerfile --build-arg VCS_REF=$(git rev-parse HEAD) -t niche .

FROM docker.io/library/node:22-bookworm AS web
WORKDIR /src/web
COPY web/package.json web/package-lock.json ./
RUN npm ci --no-audit --no-fund
COPY web/ ./
ARG VCS_REF=dev
RUN VITE_BUILD_ID="$VCS_REF" npm run build

FROM docker.io/library/rust:1.96-bookworm AS build
WORKDIR /src
COPY . .
COPY --from=web /src/web/dist web/dist
ARG VCS_REF=dev
RUN NICHE_BUILD_ID="$VCS_REF" cargo build --locked --profile dist -p niche-cli

FROM gcr.io/distroless/cc-debian12:nonroot
ARG VCS_REF=dev
COPY --from=build /src/target/dist/niche /niche
LABEL org.opencontainers.image.title="NicheRS" \
      org.opencontainers.image.description="Species distribution modelling (MaxEnt) with an embedded web UI" \
      org.opencontainers.image.licenses="MIT" \
      org.opencontainers.image.revision="$VCS_REF"
# Server mode: listen on all interfaces inside the pod; access control is done by the ingress
# (oauth2-proxy + Authentik). Data and caches live on the mounted volume.
ENV NICHE_HOST=0.0.0.0 \
    NICHE_PORT=8080 \
    NICHE_DATA_DIR=/data \
    NICHE_CACHE_DIR=/data/cache \
    NICHE_LOG_FORMAT=json
EXPOSE 8080
VOLUME ["/data"]
USER nonroot:nonroot
ENTRYPOINT ["/niche"]
CMD ["serve"]
