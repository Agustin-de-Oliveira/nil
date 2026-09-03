<div align="center">

# nil

**Suite minimalista de herramientas nativas para Linux.**

nil es un ecosistema de aplicaciones de escritorio ligeras, independientes y con foco en el teclado, construidas con **Rust**, **Tauri 2**, **Leptos (WASM)** y **Tailwind CSS**. Diseñadas para iniciar al instante, integrarse de forma fluida con Wayland y X11, y mantenerse fuera de tu camino.

[Aplicaciones](#aplicaciones) · [Stack Tecnológico](#stack-tecnológico) · [Instalación](#instalación) · [Guía de Desarrollo](CONTRIBUTING.md)

</div>

---

## ¿Qué es nil?

La mayoría de las herramientas de escritorio modernas sufren de sobrecarga: aplicaciones lentas montadas sobre runtimes pesados que consumen cientos de megabytes de memoria para tareas puntuales del día a día.

nil propone una alternativa: **utilidades de un solo propósito, rápidas, visualmente coherentes y con consumo ocioso despreciable**. Cada herramienta de la suite opera de forma autónoma con interfaces limpias, soporte nativo de atajos de teclado y una arquitectura modular compartida.

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
- **Flujo orientado a teclado**: navegación rápida con flechas y copiado inmediato al presionar Enter.

### nil-notes
Bloc de notas flotante y editor sin distracciones.
- **Jerarquía fluida de título**: el encabezado inicial se desvanece suavemente hacia arriba al presionar Enter, cediendo toda la pantalla al contenido.
- **Recuperación intuitiva**: volver hacia arriba con las flechas o tecla de borrado restaura el título para editarlo.
- **Formato rico y listas**: compatibilidad con negrita, cursiva, subrayado, tachado y casillas de verificación interactivas.
- **Sincronización en tiempo real**: títulos de pestañas reactivos derivados de la primera línea y guardado automático transparente.

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

Los paquetes ejecutables precompilados (AppImage, deb, rpm) y binarios independientes están disponibles en la sección de **[Releases](https://github.com/Agustin-de-Oliveira/nil/releases)** del repositorio.

---

## Desarrollo y Contribución

Para instrucciones sobre compilación desde el código fuente, dependencias del sistema, uso del `Makefile` y estándares de código, consultá la **[Guía de Contribución](CONTRIBUTING.md)**.

---

## Licencia

Este proyecto se distribuye bajo los términos de la Licencia MIT.
