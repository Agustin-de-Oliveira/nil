# Glint

Herramienta ligera, rápida y moderna de captura y edición de pantalla para Linux, construida con **Rust**, **Tauri 2**, **Leptos (WASM)** y **Tailwind CSS**.

Compatible de forma nativa tanto con entornos Wayland basados en wlroots (**Hyprland**, **Sway**) como con **GNOME** y sesiones **X11**.

---

## Características

- **Captura rápida e interactiva**:
  - Toque rápido en la tecla (`< 400ms`): Captura la pantalla enfocada actual directamente al portapapeles y muestra una notificación con miniatura y botón para editar.
  - Pulsación mantenida (`400ms`): Abre el selector de área y el editor completo.
- **Herramientas de Anotación**:
  - Lápiz de trazo libre suave y Resaltador translúcido.
  - Formas geométricas: Rectángulos, Flechas y Círculos.
  - Herramienta de desenfoque/censura (Blur) para ocultar información sensible.
  - Selector de color con lupa magnificadora (Pixel Loupe).
  - Múltiples paletas de color y grosores de trazo.
- **OCR Integrado (Reconocimiento de Texto)**:
  - Extracción de texto desde la imagen con un clic mediante Tesseract.
  - Detección automática de URLs y códigos de color con apertura y copia inmediata.
- **Deshacer / Rehacer / Zoom / Pan**:
  - Historial completo de cambios (`Ctrl+Z`, `Ctrl+Y`).
  - Navegación fluida por la imagen con rueda del mouse o arrastre (`Espacio + Arrastre`).

---

## Requisitos del Sistema

### Dependencias en tiempo de ejecución:
- `grim` y `slurp` (para Hyprland / Sway / Wayland wlroots) o `gnome-screenshot` / `gdbus` (para GNOME) o `maim` / `scrot` (para X11).
- `wl-clipboard` (Wayland) o `xclip` (X11).
- `tesseract` (para la función de OCR).
- `libnotify` (`notify-send` para notificaciones con acciones).

En sistemas basados en Arch Linux:
```bash
sudo pacman -S grim slurp wl-clipboard tesseract tesseract-data-eng tesseract-data-spa libnotify
```

---

## Instalación Rápida

### Opción 1: Descargar el paquete ejecutable (Recomendado)
Dirigite a la sección de **[Releases](https://github.com/)** del repositorio y descargá el archivo según tu distribución:

- **Universal (Cualquier distribución Linux)**: Descargá `Glint_x86_64.AppImage`. Dale permisos de ejecución y abrilo directamente con doble clic:
  ```bash
  chmod +x Glint_x86_64.AppImage
  ./Glint_x86_64.AppImage
  ```
- **Ubuntu / Debian / Pop!_OS / Linux Mint**: Descargá `glint_amd64.deb` e instalalo con doble clic o con:
  ```bash
  sudo dpkg -i glint_amd64.deb
  ```
- **Fedora / openSUSE / RHEL**: Descargá `glint_x86_64.rpm` e instalalo con doble clic o con:
  ```bash
  sudo rpm -i glint_x86_64.rpm
  ```

---

## Compilación Manual (Desarrollo)

Si preferís compilarlo desde el código fuente:

```bash
# 1. Compilar el frontend en WASM
cd frontend
trunk build --release

# 2. Compilar el binario optimizado en Rust
cd ../src-tauri
cargo build --release
```

Instalar el binario generado localmente:
```bash
mkdir -p ~/.local/bin
cp src-tauri/target/release/glint ~/.local/bin/glint
chmod +x ~/.local/bin/glint
```

Crear el lanzador `.desktop`:

```bash
cat << 'EOF' > ~/.local/share/applications/glint.desktop
[Desktop Entry]
Name=Glint
GenericName=Capturador de pantalla
Comment=Captura rápida, anotación, blur y OCR
Exec=glint --area
Icon=accessories-screenshot
Terminal=false
Type=Application
Categories=Utility;Graphics;
EOF
```

---

## Configuración de Atajos de Teclado

### En Hyprland (`~/.config/hypr/hyprland/keybinds.lua`):

```lua
-- Atajos para Glint
create_bind(vars.kbScreenshot, hl.dsp.exec_cmd("glint --press"), locked)
create_bind(vars.kbScreenshot, hl.dsp.exec_cmd("glint --release"), { locked = true, release = true })
create_bind(vars.kbScreenshotFreeze, hl.dsp.exec_cmd("glint --full"), locked)
create_bind(vars.kbScreenshotRegion, hl.dsp.exec_cmd("glint --area"), locked)
```

### En GNOME:
En **Configuración** -> **Teclado** -> **Atajos personalizados**:
- **Comando**: `glint --area`
- **Atajo**: `Print` o `Super + Shift + S`
