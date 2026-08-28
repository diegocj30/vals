# VALS

Boss-rush danmaku escrito en Rust. Jugable en nativo y en navegador.

Los patrones de balas de este genero son coreografia: de ahi el nombre.

Tres jefes —El Vals, El Espejo y La Coda—, tres vidas, y un parry que te empuja
a meterte donde estan las balas en vez de huir de todas.

Dos modos: **volar** por la arena, como el danmaku clasico, o **plataformas**
con gravedad y salto, mas cerca de Cuphead.

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
| Flechas / WASD | mover (en plataformas, arriba salta) |
| `Z` | disparar |
| `X` | dash con i-frames |
| `C` | **parry**: neutraliza las balas rosas que tengas cerca y llena el medidor |
| `ESPACIO` | **super**: con el medidor lleno, limpia la pantalla y pega fuerte |
| `SHIFT` | focus: lento, con la hitbox y el radio de roce marcados |
| `M` | silenciar |
| `F1` / `R` | overlay de debug / reiniciar |

Parriar las balas **rosas** llena la barra `SUPER`; cuando esta llena, `ESPACIO`
limpia la pantalla y hace mucho dano. Rozar balas sin que te den tambien la
llena, poco a poco. La idea es que acercarse compense.

Al perder se reintenta **ese** jefe, no la carrera entera.

No hay ni un asset en el repositorio: los graficos los dibuja un shader a partir
de la distancia al centro, y los sonidos los sintetiza un par de osciladores al
arrancar.

## Disenar un jefe

Los tres jefes se definen en `assets/patterns/boss1.ron`, `boss2.ron` y
`boss3.ron`, **no en codigo**. Anadir uno nuevo es escribir el fichero y meterlo
en `DEFAULT_BOSS_RONS`: esa unica linea es toda la logica que hace falta. El fichero
lleva en la cabecera la guia de los pasos del lenguaje (`Wait`, `Fire`, `Turn`,
`Repeat`, `Forever`, `Parallel`, `MoveTo`).

Con el juego abierto en nativo hay **hot-reload**: guarda el RON y el cambio
entra sin recompilar. Si el fichero tiene un error de sintaxis se avisa y se
sigue jugando con la ultima version buena.

## Replays

Un replay es **una semilla mas la lista de inputs**: como la simulacion es
determinista, con eso se recrea la partida entera. Treinta segundos ocupan
4 KB.

Se graba siempre mientras juegas; `F2` guarda la partida en `replays/`.

```bash
cargo run -p vals-app -- --replay replays/1234567890.valsrpl   # reproducir
cargo run -p vals-core --example replay_tool -- verify <fichero>
cargo run -p vals-core --example replay_tool -- info <fichero>
```

`assets/replays/golden.valsrpl` es el **replay dorado**: 30 segundos versionados
que el CI reproduce en cada push comparando huellas de estado. Es la red que
permite reescribir el bucle caliente demostrando que el comportamiento no
cambia. Si tocas la jugabilidad a proposito, falla — y hay que regrabarlo:

```bash
cargo run -p vals-core --example replay_tool -- record-golden
```

## Medir

```bash
cargo bench -p vals-core --bench sim                                  # simulacion aislada
cargo run -p vals-app --release -- --bench-scene 20000 --frames 400   # frame completo
cargo run -p vals-app --release -- --bench-scene 20000 --frames 400 --legacy-render
```

El ultimo usa el render anterior a H6 (primitivas de macroquad, dos circulos
por bala) en vez del instanciado. Se conserva para que la comparacion de
`docs/PERF.md` se pueda repetir: **a 32.000 balas van 30 fps contra 144**.

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
