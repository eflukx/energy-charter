# --- STAGE 1: Builder ---
# We gebruiken de officiële Rust image als basis om onze applicatie te compileren.
FROM rust:1.88 as builder

# Installeer de 'musl' target om een statisch gelinkte binary te kunnen bouwen.
RUN rustup target add x86_64-unknown-linux-musl

# Maak een werkdirectory aan.
WORKDIR /app

# Kopieer de dependency-bestanden en bouw de dependencies apart.
# Dit maakt gebruik van Docker's layer caching voor snellere builds.
COPY Cargo.toml Cargo.lock ./
# Maak een dummy src/main.rs aan om dependencies te kunnen bouwen
RUN mkdir src && echo "fn main(){}" > src/main.rs
RUN cargo build --release --target x86_64-unknown-linux-musl --locked

# Kopieer de rest van de source code en de HTML file.
COPY src ./src
COPY index.html ./index.html

# Bouw de uiteindelijke applicatie.
# De --touch stap zorgt ervoor dat Cargo de wijzigingen detecteert.
RUN touch src/main.rs && cargo build --release --target x86_64-unknown-linux-musl --locked


# --- STAGE 2: Final ---
# We beginnen met een volledig lege image voor maximale efficiëntie en veiligheid.
FROM scratch

# Kopieer de gecompileerde, statische binary van de builder stage.
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/energy-proxy /energy-proxy

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
ENTRYPOINT ["/energy-proxy"]
