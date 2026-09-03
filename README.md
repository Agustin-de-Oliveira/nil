<div align="center">

# nil

**Suite de herramientas ligeras para Linux.**

nil es un conjunto de aplicaciones de escritorio independientes construidas con **Rust**, **Tauri 2**, **Leptos (WASM)** y **Tailwind CSS**, diseñadas para abrir rápido, consumir pocos recursos y funcionar de manera nativa tanto en entornos Wayland como en X11.

[Aplicaciones](#aplicaciones) · [Desarrollo en Vivo](#desarrollo-en-vivo) · [Stack Tecnológico](#stack-tecnológico) · [Instalación](#instalación) · [Guía de Contribución](CONTRIBUTING.md)

</div>

---

## ¿Qué es nil?

nil reúne herramientas de escritorio para tareas puntuales del día a día (captura de pantalla, gestión de portapapeles y notas rápidas).

Cada aplicación funciona de manera autónoma con su propia ventana e interfaz, pero comparten una arquitectura común: lógica de bajo nivel implementada en Rust para máxima velocidad y bajo consumo de memoria, combinada con interfaces reactivas compiladas a WebAssembly.

---

## Aplicaciones

### nil-shot
Herramienta de captura y anotación interactiva de pantalla.
- **Captura rápida**: captura directa al portapapeles con toque breve o selección de área con pulsación mantenida.
- **Herramientas de anotación**: trazo libre, resaltador translúcido, flechas y formas geométricas (rectángulos y círculos).
- **Censura de información**: desenfoque dinámico (blur) para ocultar datos sensibles antes de compartir.
- **OCR interactivo**: extracción de texto desde la imagen con un clic mediante Tesseract, con detección de URLs y códigos de color.
- **Control de lienzo**: zoom con rueda del mouse, paneo fluido (`Espacio + Arrastre`), lupa de píxeles e historial completo de deshacer/rehacer.

### nil-clip
Gestor de historial de portapapeles de baja latencia.
- **Monitoreo eficiente**: registro automático y persistente del portapapeles con bajo impacto en recursos.
- **Búsqueda difusa (fuzzy search)**: filtrado instantáneo en tiempo real sobre el historial de copiado.
- **Previsualización contextual**: detección automática y formateo de fragmentos de código, enlaces y texto plano.
- **Acceso rápido**: navegación ágil con flechas del teclado y pegado inmediato.

### nil-notes
Bloc de notas flotante y editor sin distracciones.
- **Jerarquía fluida de título**: el encabezado inicial se desvanece suavemente hacia arriba al presionar Enter, cediendo toda la pantalla al contenido.
- **Recuperación intuitiva**: volver hacia arriba con las flechas o tecla de borrado restaura el título para editarlo.
- **Formato rico y listas**: compatibilidad con negrita, cursiva, subrayado, tachado y casillas de verificación interactivas.
- **Sincronización en tiempo real**: títulos de pestañas reactivos derivados de la primera línea y guardado automático transparente.

---

## Desarrollo en Vivo

Una de las ventajas del diseño modular con Leptos y Trunk es la posibilidad de **programar y ver los cambios en vivo**.

Al ejecutar cualquiera de las aplicaciones en modo de desarrollo (`make notes`, `make shot` o `make clip`), Trunk recompila el código WebAssembly y Tailwind en segundo plano, refrescando la interfaz al instante sin necesidad de reiniciar la ventana de Tauri ni volver a compilar los binarios de Rust.

---

## Stack Tecnológico

| Capa | Tecnología |
| :--- | :--- |
| Lenguaje Principal | Rust (Edición 2021) |
| Runtime de Escritorio | Tauri v2 |
| Frontend Reactivo | Leptos 0.7 (CSR compilado a WebAssembly) |
| Motor de Estilos | Tailwind CSS 4 |
| Bundler WASM | Trunk |
| Reconocimiento de Texto (OCR) | Tesseract OCR |
| Gestión de Portapapeles | wl-clipboard (Wayland) / xclip (X11) |
| Captura de Pantalla | grim + slurp (Wayland wlroots) / GNOME / maim (X11) |
| Notificaciones de Sistema | libnotify (`notify-send`) |
| Arquitectura del Proyecto | Monorepo modular (`crates/` + `frontend/`) |

---

## Instalación

Las versiones compiladas están disponibles en la sección de **[Releases](https://github.com/Agustin-de-Oliveira/nil/releases)** del repositorio.

### AppImage (Universal)
Descargá el archivo correspondiente a la herramienta, dale permisos de ejecución y abrilo directamente:

```bash
chmod +x nil-shot_x86_64.AppImage
./nil-shot_x86_64.AppImage
```

### Paquetes `.deb` (Debian, Ubuntu, Pop!_OS, Linux Mint)
```bash
sudo dpkg -i nil-shot_amd64.deb
```

### Paquetes `.rpm` (Fedora, openSUSE, RHEL)
```bash
sudo rpm -i nil-shot_x86_64.rpm
```

### Binarios independientes
También podés descargar directamente el binario compilado de cada herramienta y ubicarlo en tu `$PATH` (por ejemplo en `~/.local/bin` o `/usr/local/bin`).

---

## Desarrollo y Contribución

Para instrucciones sobre compilación desde el código fuente, dependencias del sistema, uso del `Makefile` y estándares de código, consultá la **[Guía de Contribución](CONTRIBUTING.md)**.

---

## Licencia

Este proyecto se distribuye bajo los términos de la **Licencia Apache 2.0**.
