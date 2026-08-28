# VALS

Boss-rush danmaku escrito en Rust. Jugable en nativo y en navegador.

Los patrones de balas de este genero son coreografia: de ahi el nombre.

## Estructura

| Crate | Que hace |
|---|---|
| `crates/vals-core` | Simulacion determinista. **Sin dependencias graficas, a proposito.** |
| `crates/vals-app`  | Ventana, render, input y audio (macroquad). |

La separacion no es decorativa. `vals-core` no importa macroquad nunca, y eso
es lo que permite testear el gameplay en headless, medirlo con `criterion` sin
ruido de GPU, y cambiar de renderer mas adelante sin reescribir el juego.

## Ejecutar

```bash
cargo run -p vals-app            # nativo, perfil dev
cargo run -p vals-app --profile fast   # nativo, optimizado, compila rapido
cargo test                       # tests, incluido el de determinismo
```

Controles:

| Tecla | |
|---|---|
| Flechas / WASD | mover |
| `Z` | disparar |
| `X` | dash con i-frames |
| `C` | **parry**: neutraliza las balas rosas que tengas cerca y llena el medidor |
| `ESPACIO` | **super**: con el medidor lleno, limpia la pantalla y pega fuerte |
| `SHIFT` | focus: lento, con la hitbox y el radio de roce marcados |
| `F1` / `R` | overlay de debug / reiniciar |

Rozar balas sin que te den tambien llena el medidor, poco a poco. La idea es
que acercarse compense.

## Disenar un jefe

Los jefes se definen en `assets/patterns/*.ron`, **no en codigo**. El fichero
lleva en la cabecera la guia de los pasos del lenguaje (`Wait`, `Fire`, `Turn`,
`Repeat`, `Forever`, `Parallel`, `MoveTo`).

Con el juego abierto en nativo hay **hot-reload**: guarda el RON y el cambio
entra sin recompilar. Si el fichero tiene un error de sintaxis se avisa y se
sigue jugando con la ultima version buena.

## Medir

```bash
cargo bench -p vals-core --bench sim                                  # simulacion aislada
cargo run -p vals-app --release -- --bench-scene 20000 --frames 400   # frame completo
```

El segundo imprime el resumen y sale solo. Los resultados van a
[`docs/PERF.md`](docs/PERF.md), que es el historico contra el que se comparan
las optimizaciones futuras.

## Build web

```powershell
powershell -File scripts/build-web.ps1
```

Deja en `web/` un `.wasm` autocontenido. Sirvelo con cualquier servidor
estatico:

```bash
python -m http.server 8080 --directory web
```

## Documentacion

- [`docs/PERF.md`](docs/PERF.md) — historico de mediciones.
