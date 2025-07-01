# Energy Price Viewer

Dit project is een webapplicatie voor het visualiseren van energieprijzen (stroom en gas) van de ANWB Energie API. Het bestaat uit een robuuste backend-service geschreven in Rust met het Axum framework, en een dynamische frontend gebouwd met HTML, Tailwind CSS en Chart.js.

De backend fungeert als een slimme proxy die de data van de ANWB API ophaalt, deze cachet voor betere prestaties en de CORS-problematiek oplost.

## Features

-   **Interactieve Grafieken:** Visualiseer energieprijzen per uur, dag of maand.
-   **Slimme Caching:** Een in-memory cache (met configureerbare TTL) vermindert de laadtijd en het aantal aanvragen naar de externe API.
-   **Parallelle Cache Warming:** Bij het opstarten wordt de cache voor de afgelopen 7 dagen op de achtergrond gevuld voor een snelle eerste ervaring.
-   **Dynamische Frontend:** De backend serveert een moderne, single-page frontend die communiceert via een interne API.
-   **Volledig Configureerbaar:** Pas het luisteradres, cache-instellingen en log-levels aan via environment variabelen.
-   **Sterk Gevetypeerde Backend:** Gebruik van Rust's enums en `DateTime` types voor robuuste en veilige code.
-   **Klaar voor Deployment:** Inclusief een multi-stage `Dockerfile` voor het bouwen van een minimale, statisch gelinkte container (`FROM scratch`).

## Projectstructuur

Voor een correcte werking moet de projectmap als volgt zijn opgebouwd:

```
energy-proxy/
├── Cargo.toml
├── Cargo.lock
├── Dockerfile
├── index.html          <-- Het frontend HTML-bestand
└── src/
    └── main.rs         <-- De backend Rust-code
```

## Lokaal Draaien

### Vereisten

-   [Rust](https://rustup.rs/) (laatste stabiele versie)

### Stappen

1.  Navigeer naar de root van het project.
2.  Start de applicatie met Cargo:
    ```bash
    cargo run
    ```
3.  De server zal nu draaien. Open je browser en ga naar `http://127.0.0.1:3000` (of het adres dat je hebt geconfigureerd).

## Configuratie (Environment Variabelen)

Je kunt de applicatie configureren door de volgende environment variabelen in te stellen voordat je `cargo run` uitvoert.

| Variabele                  | Beschrijving                                              | Standaardwaarde     | Voorbeeld (PowerShell)                    |
| -------------------------- | --------------------------------------------------------- | ------------------- | ----------------------------------------- |
| `LISTEN_ADDR`              | Het IP-adres en de poort waarop de server luistert.       | `127.0.0.1:3000`    | `$env:LISTEN_ADDR="0.0.0.0:8080"`         |
| `STATIC_FILE_PATH`         | Het pad naar het te serveren `index.html` bestand.        | `index.html`        | `$env:STATIC_FILE_PATH="static/app.html"` |
| `CACHE_TTL_SECONDS`        | De levensduur van een cache-item in seconden.             | `3600` (1 uur)      | `$env:CACHE_TTL_SECONDS="600"`            |
| `CACHE_WARMUP_CONCURRENCY` | Het max. aantal parallelle requests tijdens het opwarmen. | `10`                | `$env:CACHE_WARMUP_CONCURRENCY="4"`       |
| `RUST_LOG`                 | Bepaalt het log-niveau.                                   | `energy_proxy=info` | `$env:RUST_LOG="debug"`                   |

**Voorbeeld (PowerShell):**

```powershell
$env:LISTEN_ADDR="0.0.0.0:8080"; $env:RUST_LOG="debug"; cargo run
```

## Draaien met Docker

### Vereisten

-   [Docker](https://www.docker.com/products/docker-desktop/)

### Stappen

1.  **Bouw de Docker Image:** Zorg ervoor dat je in de root van het project bent en voer uit:
    ```bash
    docker build -t energy-proxy-app .
    ```
2.  **Start de Container:** Zodra de build voltooid is, start je de container. De poort `3000` van de container wordt gekoppeld aan poort `3000` op je machine.

    ```bash
    docker run -p 3000:3000 --rm --name my-energy-app energy-proxy-app
    ```
3.  **Open de Applicatie:** Navigeer in je browser naar `http://localhost:3000`. De volledige applicatie wordt nu geserveerd vanuit de Docker container.
