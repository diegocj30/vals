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

## H6: render instanciado — el antes y el despues

Mismo escenario, misma semilla, mismo dia y misma maquina; lo unico que cambia
es el camino de render. `--legacy-render` conserva el viejo justo para que esta
comparacion se pueda repetir dentro de un ano.

| Balas | draw (macroquad) | draw (instanciado) | mejora | fps antes | fps despues |
|---|---|---|---|---|---|
| 1 000 | 1,114 ms | **0,077 ms** | 14x | 144 | 144 |
| 5 000 | 4,682 ms | **0,142 ms** | 33x | 144 | 144 |
| 10 000 | 8,321 ms | **0,247 ms** | 34x | 92 | **144** |
| 20 000 | 16,489 ms | **0,471 ms** | 35x | 47 | **144** |
| 32 000 | 26,345 ms | **0,716 ms** | 37x | 30 | **144** |

Frame time completo (p50 / p99):

| Balas | macroquad | instanciado |
|---|---|---|
| 10 000 | 10,85 / 12,02 ms | 6,95 / 7,40 ms |
| 20 000 | 21,31 / 21,99 ms | 6,96 / 7,62 ms |
| 32 000 | 33,90 / 34,76 ms | 6,95 / 7,87 ms |

**A 32.000 balas el juego va a 144 fps clavados**, limitado por vsync y no por
el render. Sim 0,207 ms mas draw 0,716 ms: menos del **6% del presupuesto de un
frame a 60 Hz**. El cuello de botella ya no es dibujar, es el tamano del pool.

Un aviso para no leer mal la tabla: `sim` **no es comparable entre las dos
filas**. Se mide por frame, no por tick, y con el render viejo a 30 fps caben
mas ticks de simulacion en cada frame que con el nuevo a 144. Para el coste de
simulacion aislado estan los benchmarks de criterion, que no dependen de los
fps.

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

## Tirones: frames sueltos, no fps bajos

En la maquina de referencia el juego iba perfecto, y en otras salia "algun
tironcin". No era la media sino frames sueltos, asi que no los veia ni el p50
ni la escena de stress.

**Como se buscan.** Con `--pruebas`, cada frame de mas de 20 ms sale por la
salida de error con su escena, lo que paso en el y en que se fue el tiempo
(`stats::Tirones`):

```
[tiron]  23.3 ms  menu     pasos 1  sim 0.01  musica 0.00  dibujo 13.47  resto 0.04  fuera 9.80  |
```

Tras la barra van los sucesos del frame: cambio de escena o de musica,
figura, super, jefe abajo, aparicion de un enemigo del paseo... `fuera` es
`next_frame`: las corrutinas, el envio a la GPU y la espera del
vsync. Para medir sin tener que jugar, un arnes temporal (no se commitea)
recorrio en release los cinco paseos de punta a punta y los cinco bailes con
todas sus figuras, supers cada 500 ticks y la caida del jefe, cronometrando la
CPU de simulacion y de dibujo de cada frame. La web se midio con el mismo
build en el navegador del escritorio, en la misma maquina.

### Lo que habia, de mayor a menor

| Fecha | Commit | Que | Escena | Antes | Despues |
|---|---|---|---|---|---|
| 2026-10-18 | tirones | Sintetizar la sala nueva al volver a la pista tras ganar un baile | pista | **92 / 98 / 109 / 118 / 157 ms** en un frame (ganando el 1.º, 2.º... 5.º baile); en web la sintesis sola, 138 / 194 / 282 / 358 ms (1.º, 2.º, 4.º y 5.º) | **21 / 18 / 18 / 17 / 17 ms**, con el telon cerrado; en web, 139 trozos de 3 ms como mucho |
| 2026-10-18 | tirones | El rotulo del super entra con zoom: un tamano de letra por frame | combate, paseo | **9 frames seguidos** de 6,2 a 8,5 ms de dibujo (lo normal, 1,2) | 1 frame de 8-9 ms; con la letra precalentada, ninguno por encima de 3,5 |
| 2026-10-18 | tirones | Letra nueva en mitad del baile: la cartela y el HUD de la figura nueva | combate | 1 frame de **8,5 a 12,7 ms** de dibujo en cada cambio de figura | ninguno por encima de 3,5 ms |

**La sala.** `Tema::Sala` lleva los instrumentos de los bailes ganados, asi
que cambia al ganar uno, y se sintetizaba entera en una corrutina... que no
tiene ningun `await` antes de sintetizar, asi que corria entera en el frame en
que se llegaba a la pista: 134 ms de sintesis con los cinco bailes, mas 15-20
de decodificacion. En web la sintesis va de 2 a 2,7 veces mas lenta. Ahora
`audio::Mezcla` la hace a trozos de 2 ms por frame desde que se gana el baile,
mientras caen el jefe y su tarjeta (`CAIDA` + 1,6 s como poco), y sale
**muestra a muestra lo mismo** que de una vez (hay test). Lo que queda, la
decodificacion de macroquad en nativo, cae en el frame de entrar a la pista,
con el telon cerrado; en web la hace el navegador aparte.

**La letra.** macroquad rasteriza los glifos de cada tamano entero la primera
vez que salen, y cada vez que rasteriza alguno sube el atlas **entero** a la
GPU. En nativo eso cuesta de 5 a 7 ms por texto nuevo, casi todo espera del
driver; en web, de 0 a 2 ms de CPU. El rotulo del super pasaba por unos 60 tamanos
en su zoom de entrada. Ahora se rasteriza a su tamano de reposo y se estira
(`fuentes::centrado_escalado`), y la letra que solo sale en mitad del baile
—el rotulo, la cartela y el HUD de cada figura— se rasteriza al entrar, con
el telon cerrado (`precalentar`).

### Medido y descartado

Nada de esto se toca: medido, no da tirones.

| Sospechoso | Medida | Por que no |
|---|---|---|
| Dibujo de los paseos | CPU p50 0,8-1,4 ms, p99 1,5-2,0 ms; Triana, la mas cara | Estable de punta a punta, sin picos propios |
| Dibujo de los bailes | CPU p50 0,8-1,7 ms, p99 1,3-2,3 ms | Igual |
| Paso fijo y su recuperacion | Simulacion p99 ≤ 0,02 ms por tick; el tope de 5 pasos son ≤ 0,1 ms | Tras un frame lento no hay espiral: recuperar cuesta nada |
| Reservas de memoria por frame (`foto`, los `collect` del dibujo, el HUD) | No se ven en el p99 | Microsegundos; no hay fila que lo justifique |
| `Guardado::guardar` | p50 0,38 ms, el peor de 50 8,6 ms | Solo nativo (en web es `localStorage`) y en transiciones |
| Recarga en caliente | 0,33 ms cada 20 frames | Solo nativo; en web no hace nada |

Con los doce hilos de la CPU ocupados, la sintesis de una vez de la sala con
los cinco bailes sube de 134 a 192 ms; a trozos el coste por frame lo sigue
poniendo el presupuesto.

**Queda**: un frame de unos 8 ms al salir la tarjeta del final de un baile o
de un paseo (texto nuevo, cuando el juego ya ha acabado), el primer frame de
cada escena (con el telon cerrado) y los 0,8 s de sintetizar todo al arrancar
(en nativo), que son pantalla de carga y no tiron.

## Tamano del build web

| Fecha | Commit | Tamano | Nota |
|---|---|---|---|
| 2026-08-27 | H0 | 474 KB | macroquad 0.4.16, release con LTO y `strip`. Referencia: un proyecto equivalente en Bevy ronda los 20 MB. |
| 2026-08-27 | H2 | 492 KB | +18 KB por el sistema de balas y los emisores. |
| 2026-08-28 | H8 | 738 KB | +4 KB: el modo plataformas es fisica, no dependencias. |
| 2026-08-28 | H7 | 734 KB | +33 KB: dos jefes mas y el sintetizador de audio. Los sonidos no ocupan nada en disco porque no existen hasta que arranca el juego. |
| 2026-08-28 | H6 | 701 KB | +9 KB sobre H5. El pipeline propio y los shaders son texto y codigo, no dependencias. |
| 2026-08-27 | H4 | 674 KB | +3 KB. El parry, el graze y el super son logica, no dependencias. |
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
- [x] **Render instanciado, un solo draw call** *(H6)* — hecho: 37x mas rapido
      a 32.000 balas, y el render deja de ser el cuello de botella
- [x] Culling fuera de pantalla antes de subir el buffer de instancias
- [ ] Arrays densos (`swap_remove`) en vez de free-list con huecos
- [ ] ~~Grid uniforme espacial~~ — **descartado por ahora**: medido en 24,6 µs
      a 20k balas, no hay nada que ganar
- [ ] ~~SIMD sobre posicion y velocidad~~ — **aplazado**: optimizaria el 1,5%
      del frame
- [ ] ~~`rayon` sobre chunks~~ — **aplazado**, mismo motivo
- [ ] Movimiento de balas en GPU compute — solo tendria sentido despues de que
      el render deje de ser el cuello de botella
