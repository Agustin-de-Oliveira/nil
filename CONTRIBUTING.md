<div align="center">

# Guía de Contribución

**Estándares de desarrollo, arquitectura y flujos de trabajo para la suite nil.**

Esta guía describe cómo configurar el entorno local, la organización del monorepo, los comandos de desarrollo y los principios de código requeridos para colaborar en el proyecto.

[Arquitectura](#1-arquitectura-del-monorepo) · [Requisitos](#2-requisitos-del-sistema) · [Comandos](#3-flujo-de-trabajo-y-comandos) · [Estándares](#4-estándares-y-filosofía-de-código) · [Pull Requests](#5-proceso-de-pull-requests)

</div>

---

## 1. Arquitectura del Monorepo

El proyecto está organizado como un workspace modular de Cargo que desacopla la lógica de sistema en Rust (Tauri v2) de las interfaces de usuario reactivas en WebAssembly (Leptos 0.7 + Tailwind CSS 4 + Trunk).

```text
nil/
├── Cargo.toml          Manifiesto del workspace
├── Makefile            Automatización de compilación y desarrollo
├── bin/                Binarios locales de dev (trunk, tailwindcss)
├── crates/             Backends en Rust y lógica de sistema
│   ├── nil-core/       Biblioteca compartida (OCR, portapapeles, utilidades)
│   ├── nil-shot/       Backend de captura y anotación
│   ├── nil-clip/       Backend del gestor de portapapeles
│   └── nil-notes/      Backend del editor de notas
└── frontend/           Frontends en Leptos compilados a WASM
    ├── nil-shot/       UI de capturas (puerto :1420)
    ├── nil-clip/       UI de portapapeles (puerto :1421)
    └── nil-notes/      UI de notas (puerto :1422)
```

### Componentes y Responsabilidades

| Componente | Tipo | Responsabilidad |
| :--- | :--- | :--- |
| `crates/nil-core` | Biblioteca Rust | Abstracciones compartidas de Wayland/X11, OCR interactivo y persistencia. |
| `crates/nil-shot` | Binario Tauri | Gestión de ventanas transparentes, shortcuts globales y puente IPC de capturas. |
| `frontend/nil-shot` | Leptos WASM | Renderizado de anotaciones vectoriales sobre canvas, blur y selección de área. |
| `crates/nil-clip` | Binario Tauri | Listener en segundo plano del portapapeles del sistema y almacenamiento SQLite/JSON. |
| `frontend/nil-clip` | Leptos WASM | Lista virtualizada, búsqueda fuzzy en tiempo real y vistas previas tipadas. |
| `crates/nil-notes` | Binario Tauri | Ventana flotante estilo HUD, control de ciclo de vida y persistencia en disco. |
| `frontend/nil-notes` | Leptos WASM | Editor de texto enriquecido con transiciones dinámicas de encabezado y checkboxes. |

---

## 2. Requisitos del Sistema

### Entorno de Compilación

1. **Rust (Cadena estable)** con target WebAssembly:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```
2. **Trunk y Tailwind CSS CLI**:
   El proyecto incluye binarios locales listos para usar dentro del directorio `bin/`, por lo que no es estrictamente necesario instalarlos de forma global.

### Dependencias de Sistema en Tiempo de Ejecución

nil interactúa directamente con utilidades del sistema para capturas, portapapeles y notificaciones.

#### Arch Linux / Manjaro
```bash
sudo pacman -S grim slurp wl-clipboard tesseract tesseract-data-eng tesseract-data-spa libnotify
```

#### Debian / Ubuntu / Pop!_OS
```bash
sudo apt install grim slurp wl-clipboard tesseract-ocr tesseract-ocr-eng tesseract-ocr-spa libnotify-bin
```

#### Fedora / RHEL
```bash
sudo dnf install grim slurp wl-clipboard tesseract tesseract-langpack-eng tesseract-langpack-spa libnotify
```

---

## 3. Flujo de Trabajo y Comandos

El archivo `Makefile` en la raíz centraliza la orquestación de Trunk y Cargo para evitar comandos manuales extensos.

### Resumen de Comandos

| Comando | Acción | Descripción |
| :--- | :--- | :--- |
| `make shot` | Desarrollo | Inicia Trunk en `frontend/nil-shot` (:1420) y corre `nil-shot`. |
| `make clip` | Desarrollo | Inicia Trunk en `frontend/nil-clip` (:1421) y corre `nil-clip`. |
| `make notes` | Desarrollo | Inicia Trunk en `frontend/nil-notes` (:1422) y corre `nil-notes`. |
| `make check` | Verificación | Ejecuta validación estricta de tipos (`cargo check`) en todo el monorepo. |
| `make build` | Compilación | Compila los 3 frontends minificados y los binarios release en paralelo. |
| `make build-shot` | Compilación | Compila frontend y binario release exclusivo de `nil-shot`. |
| `make build-clip` | Compilación | Compila frontend y binario release exclusivo de `nil-clip`. |
| `make build-notes` | Compilación | Compila frontend y binario release exclusivo de `nil-notes`. |
| `make clean` | Mantenimiento | Remueve artefactos temporales de `target/` y `dist/`. |

### Modo Desarrollo

Al ejecutar cualquiera de las aplicaciones en modo dev:
```bash
make notes
```
El `Makefile` inicia Trunk en segundo plano para habilitar recarga en caliente (hot-reload) del código WASM y Tailwind. Al cerrar la ventana de la aplicación o presionar `Ctrl+C`, el proceso en segundo plano se limpia automáticamente sin dejar procesos huérfanos.

### Verificación de Código

Antes de confirmar cualquier cambio, verificá que no existan errores de compilación ni advertencias críticas:
```bash
make check
```

---

## 4. Estándares y Filosofía de Código

### Simplicidad ante todo
Los cambios deben tender a simplificar el sistema, no a complejizarlo. Se prefiere consolidar o eliminar código antes que añadir nuevas capas de abstracción, flags innecesarios o casos especiales. Si una solución hace crecer la superficie de la aplicación, buscá la variante que la reduzca.

### Cero comentarios en el código
No se admiten comentarios explicativos, notas `TODO`/`FIXME`, código comentado ni directivas de supresión de linter. La intención se expresa mediante:
- Nombres de funciones y variables claros y específicos.
- Estructura y modularidad natural.
- Mensajes de commit informativos.

### Dependencias mínimas
No agregues dependencias nuevas a los archivos `Cargo.toml` sin evaluar previamente si el problema puede resolverse de forma eficiente con lo que ya forma parte del workspace.

### Simetría arquitectónica
Cada herramienta de la suite debe mantener una organización simétrica en su par `crates/nil-<app>` y `frontend/nil-<app>`, reutilizando tipos base y convenciones de IPC transparentes.

---

## 5. Proceso de Pull Requests

1. **Crear una rama de trabajo**:
   Partí siempre desde `main` con un nombre representativo:
   ```bash
   git checkout -b fix/editor-caret-jump
   ```
2. **Validar localmente**:
   Asegurate de que `make check` finalice sin errores:
   ```bash
   make check
   ```
3. **Commits semánticos y atómicos**:
   Escribí mensajes de commit directos que expliquen qué cambió y por qué:
   ```bash
   git commit -m "fix(notes): preserve selection range during toolbar format toggle"
   ```
4. **Abrir el Pull Request**:
   Detallá el resumen de cambios, comportamiento anterior y validaciones realizadas.
