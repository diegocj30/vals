# Historico de rendimiento

VALS esta pensado para seguir optimizandose durante mucho tiempo. Eso solo
funciona si hay contra que comparar, asi que **cada optimizacion anade una fila
aqui, con su medicion previa y posterior**.

## Regla

> Ninguna optimizacion se aplica sin una fila previa que la justifique.

Nada de SIMD, hilos, grid espacial ni GPU compute "por si acaso". Primero se
mide, y el numero decide.

## Protocolo de medicion

1. Perfil `release` (LTO gordo, `codegen-units = 1`). **Nunca** `dev` ni `fast`:
   esos perfiles existen para iterar, no para medir.
2. Escena de stress reproducible: `cargo run -p vals-app --release -- --bench-scene`
   (llega en H2). Semilla fija y patron fijo, para que la medicion de hoy y la
   de dentro de un ano sean la misma medicion.
3. Dejar correr 10 s y anotar el **p99**, no la media. La fluidez percibida la
   arruinan los tirones, no el promedio.
4. Para el coste de simulacion aislado, sin ruido de GPU: `cargo bench -p vals-core`.

## Maquina de referencia

| | |
|---|---|
| CPU | *(pendiente de anotar)* |
| GPU | NVIDIA GeForce GTX 1660 SUPER |
| SO | Windows 10 Pro 19045 |
| Rust | 1.98.0, `x86_64-pc-windows-msvc` |

Si se mide en otra maquina, se indica en la fila.

## Tamano del build web

| Fecha | Commit | Tamano | Nota |
|---|---|---|---|
| 2026-08-27 | H0 | **474 KB** | macroquad 0.4.16, release con LTO y `strip`. Referencia: un proyecto equivalente en Bevy ronda los 20 MB. |

## Frame time

*(Vacio hasta H2: sin sistema de balas no hay nada que medir que signifique
algo. La primera fila real sale de la escena de stress.)*

| Fecha | Commit | Balas | p50 | p99 | sim | draw | Cambio |
|---|---|---|---|---|---|---|---|

## Palancas pendientes

En orden aproximado de coste/beneficio. Ver el plan para el detalle.

**Fluidez**
- [x] Interpolacion de render entre ticks *(H0)*
- [ ] Desacoplar tasa de simulacion de la de render (sim 60 Hz, render 144 Hz)
- [ ] Cero allocations y cero I/O dentro del bucle
- [ ] Simulacion a 120 Hz (colisiones mas finas, menos tunneling)
- [ ] Colision continua (swept) solo para las balas mas rapidas
- [ ] Opciones en runtime: cap de fps, vsync, densidad de particulas, bloom

**Throughput**
- [ ] Render instanciado, un solo draw call *(H6, el salto grande)*
- [ ] Culling fuera de pantalla antes de subir el buffer de instancias
- [ ] Arrays densos (`swap_remove`) en vez de free-list con huecos
- [ ] SIMD explicito sobre los arrays de posicion y velocidad
- [ ] Grid uniforme espacial — solo si hay muchos enemigos
- [ ] `rayon` sobre chunks, detras de feature flag para no romper WASM
- [ ] Movimiento de balas en GPU compute
