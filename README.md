# Spore - Spotify Native Terminal UI (TUI)

**Cliente de consola nativo en Rust para Spotify con reproducción de audio de alta fidelidad (Ogg Vorbis 320 kbps), caché en disco gapless y visualizador analógico TrueColor.**

---

## 🍄 ¿Qué es Spore?

Spore es el hermano ligero de [Calyx](https://github.com/hector-horta/calyx) diseñado específicamente para el ecosistema de **Spotify**. Mientras las aplicaciones de escritorio tradicionales consumen cientos de megabytes de memoria y recursos de GPU, Spore corre directamente en tu terminal consumiendo apenas **~25 MB de RAM**, con inicio instantáneo en **0.05 segundos** y **0% de uso de GPU**.

---

## 🔧 Arquitectura del Proyecto

### Workspace Cargo (`Cargo.toml`)

```text
crates/
├── spore-core          # Motor de audio nativo (Symphonia/Rodio), cliente Spotify Facade,
│                       # soporte de audio Ogg Vorbis 320k, SQLite WAL y caché atómico LRU.
└── spore-tui           # Interfaz de consola (Ratatui), gapless lookahead prefetching,
                        # analizador de espectro de 48 bandas y letras animadas con transliteración.
```

---

## 🚀 Instalación y Ejecución

```sh
# Clonar repositorio
git clone https://github.com/hector-horta/spore.git
cd spore

# Compilar y ejecutar
cargo run -p spore-tui
```

---

## 🎵 Atajos de Teclado (TUI)

> 💡 **Inicio Rápido**: Presiona **`u`** al abrir Spore para iniciar sesión en tu cuenta de Spotify y desbloquear audio de alta fidelidad (`[Vorbis 320k]`), tu biblioteca personal y tus playlists.

| **Tecla** | **Acción** |
|:---:|:---|
| `u` | **🔑 Iniciar Sesión / Login Spotify**: Inicia sesión para desbloquear audio completo `[Vorbis 320k]`, Biblioteca y Playlists |
| `/` | **Buscar** canciones, artistas y álbumes en el catálogo de Spotify |
| `1` / `2` / `3` / `4` / `5` | **Secciones Spotify** (`1 - Recientes`, `2 - Novedades`, `3 - Radio`, `4 - Biblioteca`, `5 - Playlists`) |
| `?` | **Ayuda** de controles y guía de configuración integrada |
| `p` | **Reproducir Lista / Play Playlist** (reproduce la lista o playlist completa) |
| `L` / `l` | **Cambiar Idioma** en caliente (Español ⇄ English) |
| `<` / `>` (o `,` / `.`) | **Pista Anterior / Siguiente** en la cola |
| `c` | **Limpiar Todo** (vacía la Cola activa o el historial de Recientes) |
| `y` | **Letras Fonéticas / Pronunciación** (cicla entre: `Dual [Kanji+Romaji]` ➔ `Romaji` ➔ `Original`) |
| `d` | **Quitar Pista** seleccionada (de la Cola o de Recientes) |
| `j` / `k` o `↑` / `↓` | **Navegar** arriba/abajo en la lista activa |
| `Enter` | **Reproducir** la canción seleccionada |
| `Espacio` | **Pausar / Reanudar** la reproducción |
| `a` | **Encolar al final** (*Add to queue*) |
| `n` | **Encolar a continuación** (*Play Next*) |
| `Tab` | **Alternar vista** entre *Catálogo* y *Cola de Reproducción* |
| `s` | **Alternar Shuffle** (Aleatorio `🔀 ON` / `OFF`) |
| `r` | **Alternar Repeat** (Repetición `🔁 Off` ➔ `🔁 All` ➔ `🔂 One`) |
| `+` / `-` | **Ajustar volumen** (+5% / -5%) |
| `q` | **Salir** de Spore |

---

## 🔐 Características Principales

- **🎧 Audio Ogg Vorbis 320 kbps de Alta Calidad**: Decodificación nativa continua sin depender de navegadores externos.
- **⚡ Caché Atómico en Disco (`~/.cache/spore/audio/`)**: Almacena las pistas ya descargadas en disco con cuota máxima configurable de **2 GB** y política de desalojo LRU.
- **⚡ Lookahead Prefetching de la Pista $n+1$ (Gapless Playback)**: Descarga anticipada en segundo plano de la siguiente pista para transiciones en **0.05 segundos** sin silencios.
- **🎤 Letras Sincronizadas con Relleno por Carácter**: Resaltado letra por letra en **24-bit TrueColor** al compás del canto.
- **🈳 Motor de Transliteración Fonética Offline (`y`)**: Transliteración automática de Japonés (Kanji + Kana a Romaji con KAKASI) y Coreano (Hangul silábico).
- **🌈 Analizador de Espectro Analógico (48 Pasos)**: 12 bandas de frecuencia con barrido suave mediante *color blending* (Dorado, Aqua y Verde esmeralda).
