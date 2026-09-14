# wtop - Windows Terminal Resource Monitor

> Un monitor delle risorse TUI per Windows con l'estetica e la densità visiva di `btop`, progettato per avere un'impronta sulle risorse di sistema **letteralmente minima**.

---

## Caratteristiche Principali

- **Visualizzazione Completa dei Componenti Hardware**:
  - **CPU**: Utilizzo globale con grafico storico Braille ad alta densità + barre di carico per singolo core logico (es. fino a 128 core) con gradiente verde → ambra → rosso corallo. Modello CPU estratto direttamente da CPUID (zero overhead).
  - **RAM & Swap/Commit**: Memoria fisica usata, disponibile, totale, limite di commit e grafico storico dedicato.
  - **Dischi & I/O**: Volumi montati (C:, D:, ecc.) con spazio libero/totale e throughput in tempo reale (MB/s o KB/s) sia in lettura che in scrittura tramite contatori ad alte prestazioni.
  - **GPU**: Percentuale di utilizzo dei motori 3D/Compute (WDDM DirectX) + VRAM dedicata allocata + nome adattatore grafico.
  - **Rete**: Monitoraggio in tempo reale del traffico di Download e Upload (KB/s, MB/s) tramite tabella interfaccia IP Helper nativa + totale sessione scaricato/inviato + nome adattatore attivo.
  - **Processi**: Gestione avanzata dei processi di sistema ordinabili per CPU%, Memoria RSS, PID o Nome, con ricerca/filtro istantaneo (`/`) e terminazione del processo (`x` / `Delete`).
- **Motore Grafico Braille Sub-Pixel**:
  - Utilizzo dei caratteri Unicode Braille (`U+2800`–`U+28FF`) per una risoluzione di 2x4 sub-pixel per cella di testo, per curve fluide e dense identiche a `btop`.
- **Doppio Tema TrueColor 24-bit**:
  - **Tema Scuro**: Sfondo carbone scuro, accenti ciano/neon e gradienti RGB.
  - **Tema Chiaro**: Sfondo alabastro/carta, accenti scuri ad alto contrasto per ambienti luminosi.
  - Tasto rapido `t` per commutare all'istante tra i temi (salvato in configurazione).
- **Frequenza di Campionamento Dinamica**:
  - Regolabile al volo con `+` e `-` (250ms, 500ms, 1000ms, 2000ms, 5000ms).
- **Zero-WMI Architecture**:
  - Nessun uso di WMI (`WmiPrvSE.exe`), zero serializzazione COM lenta.
  - Tutte le metriche sono raccolte tramite chiamate dirette Win32 / NTDLL kernel (`NtQuerySystemInformation`, `GlobalMemoryStatusEx`, `GetIfTable2`, PDH nativo).
- **Thread Collector Isolato**:
  - Un thread worker dedicato campiona l'hardware a basso livello e passa snapshot immutabili al thread grafico; la UI risponde istantaneamente a 60 FPS senza mai bloccarsi.

---

## Prestazioni Misurate su Windows 11

| Metrica | Risultato Misurato |
| :--- | :--- |
| **Impronta RAM (Working Set)** | **~18 MB** |
| **Consumo CPU (attivo)** | **< 0.05%** (~0.3s tempo CPU su 5s di campionamento) |
| **Dimensione Binario (.exe)** | **2.6 MB** (singolo file statico, zero dipendenze esterne) |
| **Flickering del Terminale** | **0%** (Double-buffering VT100 / Virtual Terminal Processing) |

---

## Piattaforme Supportate

Le release GitHub pubblicano un binario nativo per ciascuna architettura:

| Architettura | Asset | Note |
| :--- | :--- | :--- |
| **x86_64** (Intel/AMD) | `wtop.exe` | Build primaria, testata direttamente |
| **ARM64** (Windows on ARM, es. Snapdragon X) | `wtop-arm64.exe` | Build nativa cross-compilata in CI |

L'auto-updater in-app riconosce automaticamente l'architettura del binario in esecuzione e scarica sempre l'asset corretto.

---

## Scorciatoie da Tastiera

| Tasto | Azione |
| :--- | :--- |
| `Tab` / `Shift-Tab` | Cambia il pannello attivo (focus) |
| `↑` / `↓` oppure `k` / `j` | Scorri l'elenco dei processi |
| `PgUp` / `PgDn` | Salta 10 processi in alto / in basso |
| `Home` / `End` | Vai all'inizio / fine della lista processi |
| `c` | Ordina processi per **CPU %** |
| `m` | Ordina processi per **Memoria RAM** |
| `p` | Ordina processi per **PID** |
| `n` | Ordina processi per **Nome** |
| `d` | Inverte la direzione di ordinamento (Crescente / Decrescente) |
| `/` | Attiva la ricerca/filtro dei processi in tempo reale |
| `x` oppure `Delete` | Termina il processo selezionato (con richiesta di conferma `y`/`n`) |
| `t` | Commuta tema (**Dark** ↔ **Light**) |
| `+` / `-` | Aumenta o diminuisce la frequenza di campionamento |
| `u` | Applica l'aggiornamento se una nuova versione è disponibile |
| `?` oppure `h` | Mostra la finestra di aiuto con tutte le scorciatoie |
| `q` oppure `Esc` | Chiudi wtop / Cancella filtro attivo |

---

## Compilazione ed Esecuzione

### Esecuzione Diretta
```powershell
.\target\release\wtop.exe
```

### Compilazione da Sorgenti
```powershell
cargo build --release
```

La configurazione utente (tema preferito, frequenza di campionamento, ordinamento) viene salvata automaticamente in `%APPDATA%\wtop\config.json`.
