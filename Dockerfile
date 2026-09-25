FROM rust:1 AS build

WORKDIR /app
COPY . /app

RUN SQLX_OFFLINE=true cargo build --release
# Distroless has no shell to create it: an empty folder for the card images volume, so that
# Docker initialises the volume owned by the nonroot user the backend runs as.
RUN mkdir -p /out/card-images

FROM gcr.io/distroless/cc-debian13:nonroot

COPY --from=build --chown=nonroot:nonroot /app/target/release/ae /usr/local/bin/ae
COPY --from=build --chown=nonroot:nonroot /out/card-images /data/card-images

EXPOSE 8080

CMD ["/usr/local/bin/ae"]
