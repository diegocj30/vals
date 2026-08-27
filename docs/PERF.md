# Historico de rendimiento

VALS esta pensado para seguir optimizandose durante mucho tiempo. Eso solo
funciona si hay contra que comparar, asi que **cada optimizacion anade una fila
aqui, con su medicion previa y posterior**.

## Regla

> Ninguna optimizacion se aplica sin una fila previa que la justifique.

Nada de SIMD, hilos, grid espacial ni GPU compute "por si acaso". Primero se
mide, y el numero decide. Ver el hallazgo de H2 mas abajo: esta regla ya ha
evitado optimizar lo que no tocaba.

## Protocolo de medicion

Perfil `release` siempre (LTO gordo, `codegen-units = 1`). **Nunca** `dev` ni
`fast`: esos existen para iterar, no para medir.

**Coste de simulacion aislado**, sin GPU de por medio:

```
cargo bench -p vals-core --bench sim
```

**Coste real con render**, que imprime el resumen y sale solo:

```
cargo run -p vals-app --release -- --bench-scene 20000 --frames 400
```

Ambos usan la misma escena de `vals-core::bench`, con semilla fija
(`STRESS_SEED`) y secuencia de disparos fija. **Esa semilla y las poblaciones
medidas no se tocan**: cambiarlas invalida todo lo que hay debajo. Los primeros
60 frames se descartan (compilacion de shaders y llenado del pool).

Se anota el **p99**, no la media: la fluidez la arruinan los tirones, no el
promedio.

## Maquina de referencia

| | |
|---|---|
| GPU | NVIDIA GeForce GTX 1660 SUPER |
| SO | Windows 10 Pro 19045 |
| Rust | 1.98.0, `x86_64-pc-windows-msvc` |
| Monitor | 144 Hz con vsync |

Si se mide en otra maquina, se indica en la fila.

## Simulacion aislada (criterion)

`bullets_update` incluye el relleno del pool ademas del movimiento; en regimen
el relleno es solo lo que murio ese tick, asi que domina el movimiento.

| Fecha | Commit | Balas | update | colision (peor caso) | Nota |
|---|---|---|---|---|---|
| 2026-08-27 | H2 | 1 000 | 8,45 µs | 1,24 µs | |
| 2026-08-27 | H2 | 5 000 | 45,0 µs | 6,15 µs | |
| 2026-08-27 | H2 | 10 000 | 84,8 µs | 12,3 µs | |
| 2026-08-27 | H2 | 20 000 | **167,9 µs** | **24,6 µs** | 1,0% del presupuesto de frame |

Escala **lineal perfecta**: 118 Melem/s en las cuatro poblaciones. Es lo que se
espera de un layout SoA que no falla cache.

La colision es el peor caso posible: un punto donde no hay nada, para que el
barrido no pueda salir antes de tiempo. Aun asi, 813 Melem/s.

## Frame completo, con render

| Fecha | Commit | Balas | p50 | p99 | sim | draw | fps |
|---|---|---|---|---|---|---|---|
| 2026-08-27 | H2 | 1 000 | 6,95 ms | 7,48 ms | 0,004 ms | 1,06 ms | 144 (vsync) |
| 2026-08-27 | H2 | 5 000 | 6,90 ms | 8,73 ms | 0,027 ms | 5,02 ms | 145 (vsync) |
| 2026-08-27 | H2 | 10 000 | 11,18 ms | 12,21 ms | 0,062 ms | 8,62 ms | 89 |
| 2026-08-27 | H2 | 20 000 | 22,05 ms | 22,80 ms | **0,245 ms** | **17,11 ms** | 45 |

A 1 000 y 5 000 el p50 esta limitado por vsync, asi que ahi el numero que
significa algo es `draw`, no el frame time.

## Hallazgo de H2: el cuello de botella no es donde se suponia

**El render cuesta 70 veces mas que la simulacion.** A 20 000 balas, mover y
colisionar todo cuesta 0,245 ms mientras que emitir los draw calls cuesta
17,11 ms. La simulacion consume el 1,5% del presupuesto de un frame a 60 Hz.

Consecuencias directas, y por esto se mide antes de tocar nada:

- **SIMD, `rayon` y GPU compute sobre las balas no sirven de nada ahora
  mismo.** Optimizarian el 1,5% del frame. Bajan al final de la lista.
- **El grid espacial no hace falta.** La colision por fuerza bruta cuesta
  24,6 µs en el peor caso; una estructura de aceleracion solo anadiria
  complejidad y bugs a cambio de nada medible.
- **La unica optimizacion que importa ahora es el render instanciado (H6).**
  Dos `draw_circle` por bala con las primitivas de macroquad es justo la
  version lenta, y era el plan desde el principio: esto es el "antes".

## Tamano del build web

| Fecha | Commit | Tamano | Nota |
|---|---|---|---|
| 2026-08-27 | H0 | 474 KB | macroquad 0.4.16, release con LTO y `strip`. Referencia: un proyecto equivalente en Bevy ronda los 20 MB. |
| 2026-08-27 | H2 | 492 KB | +18 KB por el sistema de balas y los emisores. |
| 2026-08-27 | H3 | 671 KB | +179 KB: el interprete de patrones y, sobre todo, serde y el parser de RON. Es el precio de que los jefes se disenen en un fichero en vez de en codigo, y a este tamano sale a cuenta. |

## Palancas pendientes

**Fluidez**
- [x] Interpolacion de render entre ticks *(H0)*
- [ ] Desacoplar tasa de simulacion de la de render (sim 60 Hz, render 144 Hz)
- [ ] Cero allocations y cero I/O dentro del bucle
- [ ] Simulacion a 120 Hz (colisiones mas finas, menos tunneling)
- [ ] Colision continua (swept) solo para las balas mas rapidas
- [ ] Opciones en runtime: cap de fps, vsync, densidad de particulas, bloom

**Throughput** — reordenado segun lo medido en H2
- [ ] **Render instanciado, un solo draw call** *(H6)* — es el unico que
      importa ahora: se lleva el 98,5% del coste
- [ ] Culling fuera de pantalla antes de subir el buffer de instancias
- [ ] Arrays densos (`swap_remove`) en vez de free-list con huecos
- [ ] ~~Grid uniforme espacial~~ — **descartado por ahora**: medido en 24,6 µs
      a 20k balas, no hay nada que ganar
- [ ] ~~SIMD sobre posicion y velocidad~~ — **aplazado**: optimizaria el 1,5%
      del frame
- [ ] ~~`rayon` sobre chunks~~ — **aplazado**, mismo motivo
- [ ] Movimiento de balas en GPU compute — solo tendria sentido despues de que
      el render deje de ser el cuello de botella
