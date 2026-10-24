# RVSWD — the WCH two-wire RISC-V DMI protocol

How to clock a WCH RVSWD debug module (CH32V2/3/4, CH32X035… RISC-V parts) over a plain
two-wire bit-banged link: what the waveform is, where every edge falls, and what the DM
expects. Read this and you can write a working master from scratch.

Everything below is taken from implementations that are proven on real silicon:

| Source | What it is |
| :--- | :--- |
| `swindle/src/platform/ln/bmp_rvTap_ln.cpp` | bit-banged GPIO master (GD32F303 / CH32V307 *probes*) |
| `swindle/src/platform/rp2040/bmp_rvTap_rp2040.cpp` + `rvswd.pio` | RP2040 PIO master |
| `swindle/src/template/rvswd_template.h` | frame assembly, status decode, retries, DM init |
| `doc/rvswd.drawio` | annotated capture of a real WCH-Link talking to a real DM |

Verified configurations: CH32V307 and GD32F303 probes ↔ CH32V303 / CH32V307 targets
(`rvswdp_scan`, `attach`, memory read/write, flash program/erase).

Terminology: **probe** = the master (us), **DM** = the target's RISC-V Debug Module,
**request** = what the probe clocks out, **response** = what the DM clocks back.

---

## Contents

1. [TL;DR cheat sheet](#1-tldr-cheat-sheet)
2. [Physical layer](#2-physical-layer)
3. [The four edge rules](#3-the-four-edge-rules)
4. [Frame anatomy](#4-frame-anatomy)
5. [Parity](#5-parity)
6. [The status nibble](#6-the-status-nibble)
7. [Timing requirements](#7-timing-requirements)
8. [Bring-up: line reset and DM init](#8-bring-up-line-reset-and-dm-init)
9. [Talking to the DM: register map and transactions](#9-talking-to-the-dm-register-map-and-transactions)
10. [Error handling and retries](#10-error-handling-and-retries)
11. [Reference implementation map](#11-reference-implementation-map)
12. [Minimal master, in pseudo-code](#12-minimal-master-in-pseudo-code)
13. [Gotchas](#13-gotchas)

---

## 1. TL;DR cheat sheet

```
wires        SWCLK (probe -> target only)   SWDIO (one wire: probe drives the request,
                                            DM drives the response)
idle         both high
bit order    MSB first, everywhere
sample point CLK rising edge  (data changes on the falling edge)
start        SWDIO falls while CLK is HIGH      (an edge, *not* a clocked bit)
stop         SWDIO rises while CLK is HIGH      (an edge, *not* a clocked bit)
request      START | addr[6:0] | W/R | P P | pad[3:0] | (write only) data[31:0] | P P | STOP
address      7 bits, sent as (addr << 1) | rw ; rw = 1 write, rw = 0 read
parity       one bit = XOR of the covered field, transmitted twice (even parity)
pad          4 bits: 0000 on a write frame, 0001 on a read frame (the DM ignores them)
write resp   4 status bits (probe clocks a 16-clock window, keeps the first 4)
read resp    data[31:0] | P P | status[3:0]      (38 clocks)
status       0x3 = OK, 0x7 = also accepted, 0xF = nothing driven (retry)
```

Frame cost: **write = 134 half clock periods**, **read = 110 half clock periods**
(67 / 55 SCK cycles). See [§7](#7-timing-requirements).

---

## 2. Physical layer

```
     probe                                   target (CH32Vxxx)
   +--------+      SWCLK  ---------------->  +----------+
   |  GPIO  |                                |          |
   |  SWDIO |<-------------- SWDIO --------->|    DM    |
   +--------+      (one wire, two drivers)   +----------+
```

* **SWCLK** is driven by the probe only, and only by the probe. The DM never clocks anything:
  it is a pure slave that waits for the next clock edge.
* **SWDIO** is a single wire with two drivers: the probe drives the request, then *releases*
  it (tri-state / floating input, not "drive high") and the DM drives the response.
* **Idle state is CLK high, IO high.** Both lines have pull-ups on the target side, so a
  released IO reads as 1 — which is why "nothing drove the line" is indistinguishable from
  an all-ones status (0xF).
* When the board has a level shifter (all LN probes do, via a direction pin), the probe must
  flip the shifter's DIR pin together with its own pin mode: driving the pin hi-Z while the
  shifter still points outwards leaves the DM unable to answer.
  Reference: `SwdDirectionPin::dir_input()` / `dir_output()` in
  `swindle/src/platform/ln/bmp_swdio_ln.h`.
* There is **no target-generated clock and no clock recovery**: a gap of any length between
  two clock edges (a USB interrupt, an OS preemption, a breakpoint in your bit-bang loop)
  does not corrupt the frame. This is the big difference to WCH's *one-wire* SDI protocol
  (CH32V003), where the cell length *is* the encoding and interrupts must be masked. Over
  RVSWD, do not mask interrupts for a frame: it only stalls your own USB ISR.
* One DM per bus. There is no ID/scan phase and no multidrop: you talk to "the" DM at
  addresses 0x00–0x7f.

---

## 3. The four edge rules

Everything else follows from these four lines:

```
 1. sample      SWDIO is sampled by the DM on the CLK RISING edge.
 2. change      SWDIO changes on the CLK FALLING edge (i.e. at the start of the low phase),
                so the low phase is the data setup time and the high phase is the hold time.
 3. start bit   SWDIO falls WHILE CLK IS HIGH      -> frame begins (falling data edge)
 4. stop bit    SWDIO rises WHILE CLK IS HIGH      -> frame ends   (rising data edge)
```

Rules 3 and 4 are what makes start/stop distinguishable from a data bit: a data bit never
moves while CLK is high, so a transition during the high phase can only be framing.

One bit cell. Every drawing here uses 8 columns per bit: falling edge, 3 columns of low
phase, rising edge, 3 columns of high phase:

```
             tLOW                  tHIGH
        <------------->      <------------->
CLK  ____/‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾\_______________/‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾
IO   ‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾\_______________________
                         ^               ^
                         |               |
             SWDIO changes with        SWDIO is sampled on
             the CLK falling edge       the CLK rising edge
```

The reference implementation stores the data bit *immediately after* the clock falls and then
waits: `off(); set(bit); wait(); on(); wait();`. The wait **is** the setup time — no clock
stretch needed. Do **not** write `off(); on(); set(bit)` (data edge at the *end* of the low
phase): on a 144 MHz probe the GPIO store lands ~20 ns before the clock edge and the DM
latches the previous bit. That exact race was the cause of a whole class of "random DMI
failures"; fixing it is a reordering, not a slowdown (see [§11](#11-reference-implementation-map)).

### Start bit, zoomed

```
CLK  ‾‾‾‾‾‾‾‾\___/‾‾‾
IO   \________________
     ^       ^
     |       +-- first falling clock edge: bit A6 starts here
     +---------- IO falls while CLK is high  =>  START
```

The probe must hold CLK high for at least ~one half period between the start edge and the
first falling clock edge (the reference waits exactly one half period). This gives the DM time
to see the framing edge before it has to latch A6.

### Stop bit, zoomed

Last data bit = 1 (the line still has to be pulled low before it may rise):

```
CLK  \___/‾‾‾\___/‾‾‾‾‾‾‾‾‾
IO   ‾‾‾‾‾‾‾‾\_______/‾‾‾‾‾
             ^   ^   ^
             |   |   +-- IO rises while CLK is high  =>  STOP
             |   +------ CLK is high, IO still low
             +---------- CLK falls, IO is pulled low (data edge, no glitch)
```

Last data bit = 0 (the line is already low, only the rise is new):

```
CLK  \___/‾‾‾\___/‾‾‾‾‾‾‾‾‾
IO   ________________/‾‾‾‾‾
                     ^
                     +-- IO rises while CLK is high  =>  STOP
```

The order matters: `CLK low -> drive IO low -> wait -> CLK high -> wait -> IO rises`. Driving
the IO high at the same instant as the clock edge (the naive version, `clockOn(); set(1);`)
creates a double edge when the GPIO store lands after the clock edge, and the DM then sees a
*start* bit instead of a stop. The reference spends 4 half periods here: one low phase with
IO low, one high phase with IO low, then the rise.


---

## 4. Frame anatomy

Every DMI access is exactly one frame: start bit, request, optional response, stop bit.
Nothing is pipelined and nothing spans frames — with one exception, the DM's answer to the
*previous* write, which is allowed to be "busy" (see [§6](#6-the-status-nibble)).

```
WRITE  ┌───────┬────────────┬─────┬───────┬──────┬───────────────┬───────┬──────────────┬───────┐
       │ START │ A6 … A0    │ W/R │ P1 P1 │ pad  │ DATA[31:0]    │ P2 P2 │ 16-clock     │ STOP  │
       │ edge  │ 7 bits MSB │  =1 │ even  │ 0000 │ MSB first     │ even  │ window       │ edge  │
       └───────┴────────────┴─────┴───────┴──────┴───────────────┴───────┴──────────────┴───────┘
       48 request clocks + 16 response clocks, i.e. 1 + 96 + 1 + 32 + 4 = 134 half periods
```

```
READ   ┌───────┬────────────┬─────┬───────┬──────┬─────────────────┬───────┬────────┬───────┐
       │ START │ A6 … A0    │ W/R │ P1 P1 │ pad  │ DATA[31:0]      │ P2 P2 │ STATUS │ STOP  │
       │ edge  │ 7 bits MSB │  =0 │ even  │ 0001 │ MSB first       │ even  │ 4 bits │ edge  │
       └───────┴────────────┴─────┴───────┴──────┴─────────────────┴───────┴────────┴───────┘
       14 request clocks + 38 response clocks, i.e. 1 + 28 + 1 + 76 + 4 = 110 half periods
```

In a write frame the probe drives everything up to and including `P2 P2`, then releases the
line and only clocks; in a read frame the probe drives 14 bits and then only clocks. During
the 16-clock response window of a write frame the DM drives only the first four bits
(`S3 … S0`); the rest of the window idles high. The per-phase driver breakdown is in
[§4.4](#44-who-drives-what).

Note the two asymmetries that trip up every new implementation:

1. **The status is in a different place.** In a write frame the DM answers first (4 status
   bits, immediately after the request); in a read frame the answer is *data first*, and the
   status nibble comes last.
2. **The read request needs the pad bit** (`0001` instead of `0000`). Real masters disagree
   here — a WCH-Link capture shows `1111`, the swindle implementation sends `0001`, and both
   work — so the field is a don't-care as far as the DM is concerned; keep `0001`/`0000` and
   you are compatible with both.

The address field is 7 bits because the WCH DTM has a 7-bit DMI address (`dmi->address_width
= 7`), and the request header is built as one byte:

```c
uint8_t  header  = (addr << 1) | (write ? 1 : 0);   /* addr = 0x00 … 0x7f */
uint32_t parity1 = xor_of_bits(header);             /* even parity, see §5 */
uint32_t frame   = header;
frame = (frame << 2) | (parity1 ? 3 : 0);           /* P1 P1 */
frame = (frame << 4) | (write ? 0 : 1);             /* pad 0000 / 0001 */
if (write)
{
    frame = (frame << 32) | data;                   /* D31 … D0 */
    frame = (frame << 2) | (xor_of_bits(data) ? 3 : 0);  /* P2 P2 */
}
```

Transmitted MSB-first, that is `start, a6, a5, a4, a3, a2, a1, a0, w, p, p, pad3..pad0, d31,
d30, …, d0, p2, p2, stop`.

### 4.1 Request header (real waveform)

Read of `dmstatus` (0x11):

```
CLK  ‾‾‾‾‾‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾
IO   \_______________________/‾‾‾‾‾‾‾\_______________________/‾‾‾‾‾‾‾\_______________________________________________/‾‾‾‾‾‾‾
        A6      A5      A4      A3      A2      A1      A0      R       P1      P1      0       0       0       1
     header byte 0x22 | parity bit = XOR(header) = 0 (sent twice) | pad = 0001
```

Write of `dmcontrol` (0x10):

```
CLK  ‾‾‾‾‾‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾
IO   \_______________________/‾‾‾‾‾‾‾\_______________________________/‾‾‾‾‾‾‾\_______________________________________________
        A6      A5      A4      A3      A2      A1      A0      W       P1      P1      0       0       0       0
     header byte 0x21 | parity bit = XOR(header) = 0 (sent twice) | pad = 0000
```


### 4.2 Write frame: data, turn-around, status

`dmcontrol <= 0x80000001`, showing the last two data bits, the data parity, the turn-around
and the response:

```
CLK  \___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾‾‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾‾‾‾‾‾‾
IO   \_______/‾‾‾‾‾‾‾\_______________/‾‾‾\_______________/‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾\_______/‾‾‾‾‾
        D1      D0      P2      P2    z     S3      S2      S1      S0     STOP
     data 0x80000001, parity2 = XOR(data) = 0, status sampled = 0x3 (S3..S0 = 0 0 1 1)
```

* `D1 D0 P2 P2` are the last two data bits and the two data-parity bits, still driven by the
  probe.
* `z` is the **turn-around**: the probe tri-states SWDIO and CLK simply stays high for one
  extra half period (the reference calls `dir_input(); wait();`). Without it the DM's first
  status bit can collide with the probe still driving the line.
* `S3 … S0` are the four status bits, MSB first, driven by the DM. The probe clocks
  **16** response clocks and keeps the first four sampled bits; the rest of the window is
  idle high, because the DM has released the line by then.

### 4.3 Read frame: turn-around and response

`data0` reads back `0xA5A55A5A`, again showing only the tail:

```
CLK  \___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾‾‾‾‾‾‾
IO   ‾‾‾‾‾‾‾‾\_______________________________________/‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾\_______/‾‾‾‾‾
        D1      D0      P2      P2      S3      S2      S1      S0     STOP
     data 0xa5a55a5a, parity2 = XOR(data) = 0, status = 0x3
```

Reading it from left to right: the last two data bits (`D1 D0`, still driven by the DM), the
two data-parity bits (`P2 P2`), then the four status bits, then the stop bit that the probe
drives.

The response is therefore **38 clocks**: `DATA[31:0]`, `P2 P2`, `STATUS[3:0]`. A master that
samples all 38 bits into a 64-bit word can decode it as

```c
data   = (uint32_t)(rx >> 6);      /* bits 37..6 = DATA[31:0] */
status = (uint32_t)(rx & 0xF);     /* last four sampled bits */
```

which is exactly what `rv_dm_read()` does. The two parity bits in the middle are *not*
verified by the reference implementation (they are cheap to check: `xor_of_bits(data) ==
(rx >> 4) & 1` and the same bit again in bit 5).

### 4.4 Who drives what

| phase | CLK | SWDIO | clocks |
| :--- | :--- | :--- | :--- |
| idle between frames | probe, high | nobody (pull-up = 1) | — |
| start bit | probe, held high | probe drives the falling edge | 1 half period of CLK high |
| header: addr[6:0] + W/R + P1 P1 + pad[3:0] | probe | probe | 14 clocks |
| write data + P2 P2 | probe | probe | 34 clocks |
| write response | probe | DM (probe tri-states) | 16 clocks, first 4 meaningful |
| read response | probe | DM (probe tri-states) | 38 clocks |
| stop bit | probe (low, then high) | probe drives 0, then releases/drives 1 | 4 half periods |

So a DMI write is **14 + 34 = 48 request clocks + 16 response clocks**, and a DMI read is
**14 request clocks + 38 response clocks**.


---

## 5. Parity

Both parity fields are one bit computed as the **XOR of the covered bits** (so the field plus
its parity bit contains an even number of ones — "even parity"), and both are transmitted
**twice, back to back**. Sending them twice is not a typo: the reference and the WCH-Link
capture both do it and the DM is happy with it.

| field | covers | position |
| :--- | :--- | :--- |
| P1 | the 8 header bits (`addr[6:0]`, W/R) | 2 identical bits right after W/R |
| P2 | the 32 data bits | 2 identical bits right after the data, in both directions |

Worked examples (`p = XOR of the bits`):

| what | value | ones | parity |
| :--- | :--- | :--- | :--- |
| header, write dmcontrol (0x10) | 0x21 = `0010 0001` | 2 | 0 |
| header, read dmstatus (0x11) | 0x22 = `0010 0010` | 2 | 0 |
| header, read data0 (0x04) | 0x08 = `0000 1000` | 1 | 1 |
| header, write data0 (0x04) | 0x09 = `0000 1001` | 2 | 0 |
| data | 0x80000001 | 2 | 0 |
| data | 0xA5A55A5A | 16 | 0 |
| data | 0x00000001 | 1 | 1 |
| data | 0xFFFFFFFF | 32 | 0 |

Implementation detail: `lnOddParity()` in `swindle/private_include/lnbmp_parity.h` is the
helper used for this *and* for SWD. Despite the name it returns the plain XOR of the bits
(the name is inherited from the SWD request builder); for RVSWD, that is exactly the value you
want. WCH's own SDI documentation is explicit on the convention — "the debug slave supports
CRC8 and even parity; the current version only supports even parity" — so treat it as even
parity, not the odd parity used by ARM's SWD.

The reference implementation never *checks* `P2` on the way back, and no frame has ever been
seen to be rejected because of a parity bit — so do not build an error detector on it. Verify
a data path by writing a pattern and reading it back, not by parity.

---

## 6. The status nibble

The status is transmitted MSB first as four bits, in a *different place* depending on the
direction (see [§4](#4-frame-anatomy)).

| status | seen when | reference action |
| :--- | :--- | :--- |
| `0x3` (`0011`) | normal answer, DM accepted the access | success |
| `0x7` (`0111`) | also returned for accepted accesses | accepted as success |
| `0xF` (`1111`) | nothing drove the wire (line idles high via its pull-up) | treat as failure and retry |
| anything else | not observed on healthy silicon | treat as failure and retry |

Reading the nibble as `{S3,S2,S1,S0}`, the reference implementation accepts
`status == 3 || status == 7` and fails everything else (`rv_dm_write()` / `rv_dm_read()` in
`rvswd_template.h`). That policy is inherited from the original lnBMP driver and is what runs
on the verified CH32V303/CH32V307 targets.

The RISC-V DMI specification defines a **2-bit** response status (`0` success, `1` reserved,
`2` failed, `3` busy), which is what an SDI or JTAG DMI returns. WCH's 4-bit field is not
documented. The most consistent reading of the observations is that the meaningful status is
in the low bits and the trailing 1s are simply the released line idling high:

```
   S3 S2 S1 S0
    .  .  1  1     <- "1 1" = the DM has released SWDIO (pull-up)
    \___/  \__/
   ?  ?    status
```

...which is why "accepted" shows up as `0b0011`. Which of `S3`/`S2` flips to give `0b0111` has
not been pinned down; both nibbles occur on working accesses. Do not depend on the upper two
bits: branch on "is it 3 or 7", like the reference does, and keep the raw nibble in your logs
so you can classify a new value if you ever see one.

Consequences worth knowing:

* The status can be "not accepted yet" even though the DM will do the work anyway (a write
  that starts a system-bus or abstract access launches as soon as the DM has it). Retry, do
  not abort the session.
* A `0xF` on a *read* means the response window produced nothing: check power, the SWDIO
  direction/level-shifter, the reset sequence and the clock speed before assuming a protocol
  bug.
* A mistimed data edge (see [§3](#3-the-four-edge-rules)) usually shows up as `0xF` or a
  misplaced status, not as a parity error — the DM silently latched the wrong bits.



---

## 7. Timing requirements

The DM publishes no timing diagram. What follows is (a) what the working implementations do,
(b) the derived absolute numbers, (c) which of those numbers is a *requirement* and which is
just margin.

| symbol | meaning | reference value | hard requirement? |
| :--- | :--- | :--- | :--- |
| `tBIT` | CLK period (one bit cell) | 540 ns at the default 1.85 MHz | ≥ ~300 ns (see below) |
| `tLOW` | CLK low phase | `tBIT / 2` | data must be stable *before* the next rise |
| `tHIGH` | CLK high phase | `tBIT / 2` | ≥ ~150 ns |
| `tSU` | data valid before the rising edge | ≈ `tLOW` (a full half period) | ≥ ~100 ns — *this is the one that bites* |
| `tH` | data valid after the rising edge | ≈ `tHIGH` | any value ≥ 0 works if you change on the falling edge |
| `tS1` | CLK high after the START edge | `tBIT / 2` | ≥ several hundred ns |
| `tZ` | turn-around (probe released, CLK high) | one extra half period | ≥ 1 half period |
| `tRESP` | end of request → DM starts driving | > 2 µs, ≤ 8.6 µs | size the window in **time**, not clocks |
| `tRST` | idle after the reset sequence | 10 ms | ≥ 10 ms |
| `tGAP` | gap between frames | 0 … anything | none — the DM waits for the next clock |

### 7.1 Clock frequency of the reference

`mon fq <Hz>` (or `bmp_set_frequency_c()`) converts a requested frequency into a NOP count
(`swd_delay_cnt`) with a per-vendor calibration:

```
CH32V3 probe (144 MHz):  SCK = 16.668e6 / (ws + 5)     ws=0 -> 3.33 MHz, ws=4 -> 1.85 MHz
GD32F303 probe (96 MHz): SCK =  7.681e6 / (ws + 4.5)   ws=0 -> 1.71 MHz, ws=4 -> 0.90 MHz
```

`rv_dm_start()` clamps `ws` to ≥ 1 (`BMP_MIN_WS`), and the default `swd_delay_cnt` is 4. A
half period is one `swait()` = `ws` NOPs plus the GPIO stores, i.e. `tBIT / 2`.

There is no documented maximum. 1–2 MHz is a sane default; the DM's sampling margin and your
own edge placement are what run out first, and throughput is dominated by the *number* of DMI
transactions, not by SCK (see [§9](#9-talking-to-the-dm-register-map-and-transactions)).

### 7.2 What a frame costs — exact half-period budget

Count from the reference loop (`off(); set(); wait(); on(); wait();` = 2 half periods per bit):

| segment | write | read |
| :--- | ---: | ---: |
| CLK high after the start edge | 1 | 1 |
| request bits (header 14 / data 34) | 48 × 2 = 96 | 14 × 2 = 28 |
| turn-around (`dir_input()` + wait, CLK high) | 1 | 1 |
| response clocks × 2 | 16 × 2 = 32 | 38 × 2 = 76 |
| stop bit (low, high, low, high — 4 waits) | 4 | 4 |
| **total half periods** | **134** | **110** |
| **total CLK periods** | **67** | **55** |

Absolute durations (`67 / f` and `55 / f`):

| SCK | write frame | read frame |
| ---: | ---: | ---: |
| 3.3 MHz (CH32V3, ws=0) | 20 µs | 17 µs |
| 1.85 MHz (CH32V3, ws=4, default) | 36 µs | 30 µs |
| 1.0 MHz (`mon fq 1000000`) | 67 µs | 55 µs |
| 0.90 MHz (GD32F303, ws=4) | 74 µs | 61 µs |

These are model values from the wait-state calibration above (±20 %); the *ratios* (134 vs
110 half periods) are exact, which is what matters when you compare two implementations.

### 7.3 The response window is a time budget, not a bit count

This is the subtlest part of the protocol. The DM does not answer on the very next clock: it
needs a few microseconds after the last request clock before it drives the status. With a
4-clock response window (≈2.2 µs at 1.85 MHz) a 144 MHz probe got `0xF` back — "nothing
driven" — on most writes; with **16 clocks (≈8.6 µs)** the same hardware answers every time.

So:

```
response_window_clocks >= 8 us * SCK          (16 clocks at 1.85 MHz, 25 at 3 MHz, 8 at 1 MHz)
```

and the status is still the **first four sampled bits** of that window. If you change the
clock, recompute the window: a fixed "16" is only correct for the clock it was tuned at.
(`RVSWD_WRITE_RESPONSE_CLOCKS` in `rvswd_template.h`.)

### 7.4 Gaps, interrupts and preemption are free

Because the DM is a pure slave clocked by the probe, there is no maximum inter-bit or
inter-frame delay. Practical consequence: **do not mask interrupts for a frame.** The bit-bang
loop only needs consistent edge *ordering*; a USB ISR or a scheduler tick that lands between
two bits merely makes the frame longer. (SDI, the one-wire protocol of the CH32V003, is the
opposite: there the cell width *is* the encoding, and a wrapper must mask interrupts — see
`bmp_sdiTap_ln.cpp`'s critical sections. Do not copy that pattern into RVSWD.)


---

## 8. Bring-up: line reset and DM init

### 8.1 Line reset

Before the first frame (and after a target reset), clock the DM with SWDIO held high, then emit
a stop bit and wait:

```
CLK  ‾‾\___/‾‾‾\___/‾‾‾\___/‾‾‾  ... 100 pulses ...  \___/‾‾‾‾‾‾‾‾‾‾‾‾‾
IO   ‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾‾\______/‾‾‾‾‾‾‾‾‾‾‾‾‾
                 probe drives 1, nothing is transmitted        stop      then idle 10 ms
```

```c
io_dir_output();
io_set(1);
for (int i = 0; i < 100; i++) { clk_off(); clk_on(); }   /* 100 idle clocks */
io_set(0); io_set(1);                                   /* stop edge, CLK high */
delay_ms(10);
```

The 100 clocks do not carry data (SWDIO stays high); they are the "get in sync / flush" phase
the DM wants. The reference implementation gets away with a *very short* low pulse for the
stop bit here (two back-to-back GPIO stores, no waits) because in this one place the DM only
needs to see the rising edge — but a properly timed stop bit (as in [§3](#3-the-four-edge-rules))
is always safe.

### 8.2 DM init, as captured from a WCH-Link

This is the sequence in `rv_dm_probe()` (`rvswd_template.h`). It is reverse-engineered from a
WCH-Link capture, not from documentation, but it is what works. `DELAY()` = 1 ms.

| # | access | value | why |
| ---: | :--- | :--- | :--- |
| — | line reset (§8.1) | — | sync |
| 1 | write `0x10` dmcontrol | `0x80000001` | `dmactive=1`, `hartsel=0` |
| 2 | write `0x10` dmcontrol | `0x80000001` | repeated, as captured |
| 3 | read `0x11` dmstatus | — | must not be all ones |
| 4 | read `0x7f` | → chip id | WCH extension: `0x203xxxx`, `0x303xxxx`, `0x305xxxx`… |
| 5 | write `0x05` data1 | `0x1ffff704` | as captured |
| 6 | write `0x17` command | `0x02200000` | abstract: access *register*, 32-bit, read, regno 0 |
| 7 | read `0x04` data0 | — | |
| 8 | read `0x05` data1 | — | |

Success criterion used by the scan: `(chip_id & 0x7fff) != 0x7fff` — i.e. the DM answered
something that looks like a WCH part id. On a CH32V303 the reference prints
`WCH : found 0x30310514 device`.

If step 3 or 4 returns `0xFFFFFFFF` (or the status is `0xF`), the DM did not answer: check the
line reset, the pin direction, the target's power and its `nRST`, and try a lower SCK.

---

## 9. Talking to the DM: register map and transactions

Addresses are 7 bits. The DMI register map is the standard RISC-V one (WCH adds `0x7f` for the
chip id):

| addr | register | notes |
| ---: | :--- | :--- |
| `0x04`–`0x07` | `data0` … `data3` | abstract-command arguments / results |
| `0x10` | `dmcontrol` | bit 31 `dmactive`, `hartsel`, `resumereq`, `ndmreset` |
| `0x11` | `dmstatus` | `allhalted`/`allrunning`/`authenticated`… |
| `0x16` | `abstractcs` | `busy`, `cmderr` |
| `0x17` | `command` | abstract command |
| `0x18` | `abstractauto` | `autoexecdata0` — the streaming trick used by `AbstAuto` |
| `0x20`… | `progbuf0`… | program buffer |
| `0x38`/`0x39`/`0x3a` | `sbcs` / `sbaddress0` / `sbaddress1` | system bus access |
| `0x3c`/`0x3d` | `sbdata0` / `sbdata1` | |
| `0x7f` | WCH chip id | non-standard |

A DMI access is a single frame — there is no separate "read data" frame:

```c
/* read dmstatus */
ok  = rv_dm_read(0x11, &status);        /* one frame: request 14 clocks, answer 38 clocks */

/* write dmcontrol */
ok  = rv_dm_write(0x10, 0x80000001);    /* one frame: request 48 clocks, answer 4 status bits */

/* abstract memory read of one word: several frames, because the DM is a state machine */
rv_dm_write(0x04, address);             /* data0 = address     */
rv_dm_write(0x17, 0x02200000);          /* command = access mem, 32-bit, read */
/* poll abstractcs (0x16) until busy == 0 */
rv_dm_read(0x04, &value);               /* data0 = result      */
```

The `AbstAuto` streaming mode (`abstractauto.autoexecdata0 = 1`) is what makes bulk transfers
cheap: issue `command` once, then move only `data0` for the following words (write `data0` to
write memory, read `data0` to read it back). Measured on CH32V303 over RVSWD: 10 DMI
transactions for one word, 142 for 64 words — about 2.2 transactions per word, against 4 in
non-streaming mode. At ~36 µs per frame that is the difference between "usable" and
"sluggish", so if you write a master, get the streaming modes right rather than the clock rate.


---

## 10. Error handling and retries

A DMI access either answers `0x3`/`0x7` (accepted) or something else (retry). The reference
policy, `ch32_riscv_dmi_read()` / `ch32_riscv_dmi_write()`:

```
attempts   = 6
spacing    = 100 us between attempts
on success = clear fault
on failure = keep the last bad status + the frame duration, log once when the budget is out
```

Why the *spacing*: a DM that is still finishing a system-bus or abstract access answers "busy",
and two back-to-back frames are much closer together than that busy window, so without the
delay all six attempts land inside the same busy period. Retrying after a human-scale pause is
what turns "busy" into a success.

Rules that keep a session healthy:

* Retry any non-`0x3`/`0x7` answer, read *and* write, with a bounded budget — never loop
  forever on a dead bus.
* After a write that triggers work (`command`, `sbcs`), the next read may legitimately come
  back busy: poll `abstractcs.busy` (0x16) rather than re-issuing the write.
* Drop back to a lower SCK when the failure rate rises; the response-window budget is in µs
  ([§7.3](#73-the-response-window-is-a-time-budget-not-a-bit-count)), so a faster clock shrinks
  the margin twice over.
* Log the *status value* on failure, not just "failed". `0xF` (nothing driven), `0x3` (fine)
  and "some other nibble" (protocol/edge problem) point at completely different bugs.

---

## 11. Reference implementation map

| file | what lives there |
| :--- | :--- |
| `swindle/include/bmp_rvTap.h` | the transport API: `rvswd_write_then_read(tx_data, tx_bits, rx_bits)`, `rv_dm_reset()`, `rvswd_scan()` |
| `swindle/src/platform/ln/bmp_rvTap_ln.cpp` | bit-banged physics for the LN probes (GD32F303, CH32V307): the edge ordering, the turn-around, the stop bit, `rv_dm_reset()` |
| `swindle/src/platform/rp2040/bmp_rvTap_rp2040.cpp` + `swindle/src/platform/rp2040/rvswd.pio` | same contract, PIO backend: one instruction per half period, `side_set` = {direction, CLK} |
| `swindle/src/template/rvswd_template.h` | frame assembly (`rv_dm_write()`, `rv_dm_read()`), status decode, retry budget, DM init/scan, the `riscv_dmi_s` callbacks |
| `swindle/src/platform/ln/bmp_tap_ln.cpp` | `mon fq` → wait states (`bmp_set_frequency_c()`), pin mode ownership |
| `swindle/src/platform/ln/bmp_swdio_ln.h` | `SwdDirectionPin` (pin + shifter DIR), `SwdWaitPin` (`clockOn/clockOff/wait` = store + `swait()`) |
| `swindle/private_include/lnbmp_parity.h` | `lnOddParity()` — the XOR-parity helper used for `P1`/`P2` |
| `blackmagic/src/platforms/hosted/wchlink_riscv_dtm.c` | the same DMI spoken over a WCH-Link's USB protocol — useful as a cross-check of the register layer |
| `doc/rvswd.drawio` | annotated capture of a real WCH-Link frame: start bit, 7-bit address, W/R, parity, pad, data, parity, 4-bit status, stop |

### 11.1 Why the LN backend looks the way it does

The three details that were expensive to find, and that a new implementation should copy:

**1. The data edge comes first, the clock edge second.**

```c
rSWCLK->off();                        /* low phase starts                    */
rSWDIO->set(bit);                     /* data settles during the low phase   */
rSWCLK->wait();                       /* ... and this wait is the setup time */
rSWCLK->on();                         /* rising edge, data already stable    */
rSWCLK->wait();                       /* high phase                          */
```

The tempting variant `off(); on(); set(bit);` puts the GPIO store ~20 ns before the clock
edge on a 144 MHz core and loses the race — the DM latches the previous bit. A per-bit extra
`wait()` also fixes it, but costs +39 % frame time; the reordering costs nothing.

**2. The stop bit is `CLK low → drive 0 → CLK high → drive 1`.** Driving the line high in the
same store sequence as the clock rise can produce a stop edge *and* a start edge (the DM sees
the falling edge of the same store). Two half periods of "CLK high, line low" in between make
it unambiguous. Cost: 4 half periods instead of 2, once per frame.

**3. The response window is `RVSWD_WRITE_RESPONSE_CLOCKS = 16`,** not 4: see
[§7.3](#73-the-response-window-is-a-time-budget-not-a-bit-count). The status is decoded from
the *first* four sampled bits (`rx >> 12`), the remaining clocks are idle high.

**No critical section.** `rvswd_write_then_read()` deliberately does *not* mask interrupts,
unlike its SDI sibling: RVSWD is probe-clocked, so preemption only stretches the frame.


---

## 12. Minimal master, in pseudo-code

Everything the protocol needs, in one frame function plus two wrappers. `half_delay()` is one
half clock period (a NOP loop on a bit-banged GPIO); the hooks are whatever your platform
provides (`uint64_t`/`bool` come from `<stdint.h>`/`<stdbool.h>`). This has been compiled and
compared bit-for-bit against `rvswd_template.h` for all 128 addresses.

```c
/* ---- platform hooks ---------------------------------------------------------- */
static void clk_lo(void);       /* SWCLK low                                        */
static void clk_hi(void);       /* SWCLK high                                       */
static void io_drive(int v);    /* drive SWDIO (and point the level shifter outwards) */
static void io_release(void);   /* tri-state SWDIO (and point the level shifter inwards) */
static int  io_read(void);      /* read SWDIO                                       */
static void half_delay(void);   /* one half clock period                            */

/* ---- one frame: start, request, optional response, stop --------------------- */
static uint64_t rvswd_frame(uint64_t tx, int tx_bits, int rx_bits)
{
    uint64_t rx = 0;

    /* START: SWDIO falls while CLK is high (one half period of CLK high first) */
    io_drive(0);
    half_delay();

    /* request, MSB first.  Data changes on the falling edge, is sampled on the
     * rising edge; the wait after the store is the setup time. */
    for (int i = tx_bits - 1; i >= 0; i--)
    {
        clk_lo();
        io_drive((int)((tx >> i) & 1));
        half_delay();               /* low phase == data setup time */
        clk_hi();
        half_delay();               /* high phase                   */
    }

    /* turn-around and response, MSB first */
    if (rx_bits)
    {
        io_release();
        half_delay();               /* probe released, CLK high     */
        for (int i = 0; i < rx_bits; i++)
        {
            clk_lo();
            half_delay();
            clk_hi();
            half_delay();
            rx = (rx << 1) | (uint64_t)io_read();   /* sample during the high phase */
        }
    }

    /* STOP: pull low while CLK is low, raise CLK, then let the line rise */
    clk_lo();
    io_drive(0);
    half_delay();
    clk_hi();
    half_delay();
    io_drive(1);
    return rx;
}

/* ---- helpers ---------------------------------------------------------------- */
static int xor_parity(uint32_t v)        /* 1 if the bit count is odd */
{
    v ^= v >> 16; v ^= v >> 8; v ^= v >> 4; v &= 0xF;
    return (0x6996 >> v) & 1;
}

#define RESPONSE_WINDOW_CLOCKS 16        /* >= 8 us worth of clocks! */

static bool dm_write(uint8_t addr, uint32_t value)
{
    uint32_t hdr = (uint32_t)(addr << 1) | 1u;          /* 1 = write */
    uint64_t t   = hdr;
    t = (t << 2) | (xor_parity(hdr) ? 3u : 0u);         /* P1 P1     */
    t = (t << 4);                                       /* pad 0000  */
    t = (t << 32) | value;                              /* D31 … D0  */
    t = (t << 2) | (xor_parity(value) ? 3u : 0u);       /* P2 P2     */
    uint32_t status = (uint32_t)(rvswd_frame(t, 48, RESPONSE_WINDOW_CLOCKS) >> 12) & 0xFu;
    return status == 3u || status == 7u;
}

static bool dm_read(uint8_t addr, uint32_t *out)
{
    uint32_t hdr = (uint32_t)(addr << 1);               /* 0 = read  */
    uint64_t t   = hdr;
    t = (t << 2) | (xor_parity(hdr) ? 3u : 0u);         /* P1 P1     */
    t = (t << 4) | 1u;                                  /* pad 0001  */
    uint64_t rx  = rvswd_frame(t, 14, 38);              /* 32 data + P2 P2 + status */
    uint32_t status = (uint32_t)(rx & 0xFu);
    *out = (uint32_t)(rx >> 6);
    return status == 3u || status == 7u;
}

/* ---- line reset + bring-up --------------------------------------------------- */
static void dm_reset(void)
{
    io_drive(1);
    for (int i = 0; i < 100; i++) { clk_lo(); clk_hi(); }
    io_drive(0); io_drive(1);       /* stop edge (a short pulse is tolerated here) */
    delay_ms(10);
}

static bool dm_probe(uint32_t *chip_id)
{
    uint32_t ignored;
    dm_reset();
    dm_write(0x10, 0x80000001u); delay_ms(1);   /* dmcontrol: dmactive=1 */
    dm_write(0x10, 0x80000001u); delay_ms(1);
    dm_read(0x11, &ignored);     delay_ms(1);   /* dmstatus              */
    dm_read(0x7f, chip_id);      delay_ms(1);   /* WCH chip id           */
    dm_write(0x05, 0x1ffff704u); delay_ms(1);
    dm_write(0x17, 0x02200000u); delay_ms(1);   /* abstract: read x0     */
    dm_read(0x04, &ignored);     delay_ms(1);
    dm_read(0x05, &ignored);
    return (*chip_id & 0x7fffu) != 0x7fffu;
}
```

### Checklist for a new implementation

1. Idle: CLK high, SWDIO released or driven high. Direction/level-shifter pointing out.
2. Line reset: 100 clocks with SWDIO high, stop edge, 10 ms idle.
3. Frame: START edge → request MSB-first with **data before clock** → release → response →
   STOP edge.
4. Write frame: 48 request clocks, 16 response clocks, status = first 4 bits.
5. Read frame: 14 request clocks, 38 response clocks, data = bits 37..6, status = bits 3..0.
6. Retry non-3/7 statuses 6 times, 100 µs apart; log the status value on give-up.
7. Do not mask interrupts; do not add tBIT padding between frames.


---

## 13. Gotchas

Things that have cost time on this hardware, or that are easy to get wrong.

**`0xF` is ambiguous.** The line idles high, so "the DM never drove anything" and "the DM
drove four ones" look identical. Before blaming the protocol, check: target powered? SWDIO
level-shifter direction? line reset done? `dmactive` set? SCK too fast?

**`dmactive` is not the default.** A DM that was never sent `dmcontrol = 0x80000001` does not
answer DMI traffic. That is the first thing [§8.2](#82-dm-init-as-captured-from-a-wch-link)
fixes.

**Data edge before clock edge, always.** `off(); set(); wait(); on(); wait();` — if you write
`off(); on(); set();` the last bit of the frame depends on whether your GPIO store beat the
clock store. On a 144 MHz probe it does not.

**Only framing edges may coincide with CLK high.** A data bit that changes while CLK is high
is indistinguishable from a start or stop bit. That is why the stop bit needs its own low
phase.

**The response window is measured in microseconds.** `RESPONSE_WINDOW_CLOCKS = 16` is tuned
for 1.85 MHz. At 3 MHz that is 5.3 µs and it gets marginal; at 1 MHz it is 16 µs and you waste
time. Scale it with the clock: `clocks ≈ 8 µs × f_SCK`.

**Status position depends on the direction.** Write: *first* four response bits. Read: *last*
four response bits, after 32 data + 2 parity bits. Mixing the two up yields plausible but
wrong "data", and a "status" that is really the top four bits of the data word (which look
like a constant once the target's addresses settle).

**Busy needs a pause.** Retrying a busy DM back-to-back just burns the budget; space the
attempts (100 µs in the reference).

**Do not mask interrupts.** RVSWD is probe-clocked; gaps are free. The critical sections in
`bmp_sdiTap_ln.cpp` belong to SDI, where the cell timing *is* the encoding — copying them into
the RVSWD path only stalls the USB ISR for a whole frame.

**Parity is even, and sent twice.** A single odd-parity bit (the ARM SWD convention) is a
different field, and a single even bit is a different length. Both mistakes shift the frame.

**The address is 7 bits, and the header is a byte.** `(addr << 1) | rw`. Address 0x7f exists
(WCH chip id); 0x80 and above do not.

**No scan, no multidrop.** `rvswdp_scan` is "try to talk to the DM at 0x10/0x7f"; there is no
ID register to enumerate. (WCH's *one-wire* SDI protocol does have a scan handshake; do not
confuse the two.)

**Verify with readback, not with parity.** The DM was never observed to reject a frame on a
bad `P2`. If you need confidence, write a pattern and read it back (`compare-sections` in a GDB
session does exactly that).

### Debugging the wire itself

Two cheap tricks that resolve most questions:

* Put a logic analyzer on SWCLK/SWDIO and decode with 8 samples per bit cell, MSB first,
  rising-edge sampling. Start/stop bits then pop out as "data edges during CLK high", and a
  mis-ordered `set()` shows up immediately as data changing *after* the rising edge.
* Count clocks per frame in the capture: 48 (write request), 16 (write response), 14 (read
  request), 38 (read response). A frame that is off by one bit is a framing bug, not a DM bug.

