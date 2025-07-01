# --- STAGE 1: Builder ---
# We gebruiken de officiële Rust image als basis om onze applicatie te compileren.
FROM --platform=$BUILDPLATFORM rust:1.88 as builder

# ARG's om de doelarchitectuur te bepalen. Deze worden automatisch door `buildx` gevuld.
ARG TARGETPLATFORM
ARG TARGETARCH

# --- UPDATE: Installeer de benodigde C-compilers voor musl cross-compilation ---
# De 'ring' crate heeft een C-compiler nodig voor de doelarchitectuur.
RUN apt-get update && apt-get install -y musl-tools gcc-aarch64-linux-gnu && rm -rf /var/lib/apt/lists/*

# Installeer de benodigde 'musl' target voor de doelarchitectuur.
# We gebruiken een CASE statement om de juiste target te kiezen.
RUN <<EOT
    set -e
    case "$TARGETARCH" in
        "amd64") RUST_TARGET="x86_64-unknown-linux-musl" ;;
        "arm64") RUST_TARGET="aarch64-unknown-linux-musl" ;;
        *) echo "Unsupported architecture: $TARGETARCH"; exit 1 ;;
    esac
    rustup target add $RUST_TARGET
EOT

# Maak een werkdirectory aan.
WORKDIR /app

# Kopieer de dependency-bestanden en bouw de dependencies apart.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main(){}" > src/main.rs
# UPDATE: Bouw de dependencies voor de specifieke target.
RUN <<EOT
    set -e
    case "$TARGETARCH" in
        "amd64") RUST_TARGET="x86_64-unknown-linux-musl" ;;
        "arm64") RUST_TARGET="aarch64-unknown-linux-musl" ;;
    esac
    cargo build --release --target $RUST_TARGET --locked
EOT

# Kopieer de rest van de source code en de HTML file.
COPY src ./src
COPY index.html ./index.html

# Bouw de uiteindelijke applicatie.
RUN <<EOT
    set -e
    case "$TARGETARCH" in
        "amd64") RUST_TARGET="x86_64-unknown-linux-musl" ;;
        "arm64") RUST_TARGET="aarch64-unknown-linux-musl" ;;
    esac
    touch src/main.rs && cargo build --release --target $RUST_TARGET --locked
EOT


# --- STAGE 2: Final ---
# We beginnen met een volledig lege image voor maximale efficiëntie en veiligheid.
FROM scratch

# Kopieer de gecompileerde, statische binary van de builder stage.
# UPDATE: Het pad is nu dynamisch gebaseerd op de target architectuur.
COPY --from=builder /app/target/*-unknown-linux-musl/release/energy-chart /energy-chart

# Kopieer het HTML-bestand van de builder stage.
COPY --from=builder /app/index.html /app/index.html

# Stel de environment variabelen in zodat de app weet waar hij moet luisteren
# en waar het HTML-bestand te vinden is.
ENV LISTEN_ADDR="0.0.0.0:3000"
ENV STATIC_FILE_PATH="/app/index.html"
ENV RUST_LOG="info"

# Exposeer de poort waarop de applicatie luistert.
EXPOSE 3000

# Het commando om de applicatie te starten wanneer de container wordt uitgevoerd.
ENTRYPOINT ["/energy-chart"]
