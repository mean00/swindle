# The probe ↔ host RPC

Swindle splits a debug session in two:

- the **probe** (RP2040 / GD32F303 / CH32V307 firmware) only does what needs to be
  close to the pins — SWD/RVSWD/SDI bit-banging, DP/AP transfers, target voltage,
  reset, frequency, SPI/JTAG scan;
- the **host** (`hosted/build/swindle_hosted`) runs everything CPU-specific — the
  ARM and RISC-V debug drivers, flash algorithms, the GDB server.

The two halves talk over the probe's GDB CDC with a small request/reply protocol,
the **RPC**. This document describes that protocol, where it is defined, and how to
change it.

Nothing in the protocol is hand-written twice: both sides (and the shared
constants) are generated from one file, so the probe and the host cannot drift
apart.

---

## 1. Source of truth: `rpc_protocol.toml`

`swindle/rs/rs_swindle/rpc_protocol.toml` is the only place the protocol is
defined. One `[class.X]` table per packet class, one `[class.X.command.Y]` table
per command:

```toml
[class.GEN]
id = "G"                     # class byte on the wire
handler = "rpc_gen_impl"     # module in src/native/rpc_target_impl/
description = "Generic probe operations (voltage, reset, frequency, pins)"

[class.GEN.command.FREQ_SET]
id = "F"                     # command byte on the wire
old_name = "RPC_FREQ_SET"
description = "Set SWD frequency"
params = [ { name = "freq_khz", type = "u32" } ]
reply = "ok"
```

`params` is an ordered list — order _is_ the CBOR array order. A `reply` is either the
bare string `"ok"` or a table:

| `reply`                                  | target sends         | host function returns        |
| ---------------------------------------- | -------------------- | ---------------------------- |
| `"ok"`                                   | `&K`, no payload    | `bool`                       |
| `{ type = "string" }`                    | `&K` + text          | `&'static [u8]`              |
| `{ type = "u8" }`                        | `&K` + `[n]`        | `u8`                         |
| `{ type = "u32" }`                       | `&K` + `[n]`        | `u32`                        |
| `{ type = "u32", on_error = "ok_bool" }` | `&K` / `&E…`         | `(bool, u32)`                |
| `{ type = "u32", on_error = "fault" }`   | `&K` / `&E…`         | `(i32, u32)`                 |
| `{ type = "hex_block", max_length = N }` | `&K` + `[bytes]`    | `(i32, &'static [u8])`       |
| `{ type = "compound", fields = [...] }`  | `&K` + `[f, …]`     | tuple, one element per field |

`on_error` decides what a failing command reports — `"fault"` gives an `i32`
fault code, `"ok_bool"` a `false`, and a `compound` reply with `on_error =
"fault"` returns `(1, 0)`.

### Payload encoding — CBOR

The payload is [CBOR](https://cbor.io) (`minicbor`, `no_std`): one array whose
elements are the `params` — or, for a reply, the reply fields — in order.

| type        | CBOR item               | `0x1234` on the wire |
| ----------- | ----------------------- | -------------------- |
| `u8`        | unsigned integer        | `19 12 34`           |
| `u16`       | unsigned integer        | `19 12 34`           |
| `u32`       | unsigned integer        | `19 12 34`           |
| `bool`      | `false` / `true`        | `f4` / `f5`          |
| `hex_block` | byte string (raw bytes) | `42 12 34`           |
| `string`    | text string             | `62 …`               |

An integer uses the shortest CBOR head, so small values are a single byte and
`0x1234` needs three. Nothing is byte-swapped or hex-expanded: a 1 KiB memory
block costs 1 KiB on the wire plus a 3-byte head.

Do not invent new spellings here; add a type to `TYPE_INFO` in `generate_rpc.py`
instead. Both halves go through `src/rpc_wire.rs` and the generated code, so an
integer is just an integer.

---

## 2. Generation

```bash
cd swindle/rs/rs_swindle
python3 generate_rpc.py rpc_protocol.toml --out-dir src/
```

Three files are produced (all tracked in git, all marked _DO NOT EDIT_):

| file                                 | consumer | content                                                                                                          |
| ------------------------------------ | -------- | ---------------------------------------------------------------------------------------------------------------- |
| `src/rpc_common_generated.rs`        | both     | `RpcClass` / `<Class>Command` enums, `RPC_*` byte constants, `old_name` aliases                                  |
| `src/native/rpc_target_generated.rs` | probe    | `rpc_dispatch()` + one module per class that parses params and calls `crate::native::rpc_target_impl::<handler>` |
| `src/hosted/rpc_host_generated.rs`   | host     | one `remote_<class>_<command>()` client function per command                                                     |

Rebuild both halves after regenerating — an old probe with new host code will
mis-decode, because nothing checks the version automatically (see §5).

---

## 3. Wire format

### Request (host → probe)

```
'!' <escaped payload> '#' <checksum, 2 hex digits>
payload = <class byte> <command byte> <CBOR array of the params>
```

- escaping: `!`, `#`, `$`, `{` and `}` inside the payload are sent as `}` followed
  by the byte XOR `0x20`; both bytes are summed into the checksum. The payload is
  binary, so this is load-bearing: without it the probe's parser would end the
  frame at the first `0x23`;
- the checksum is the 8-bit sum of the payload bytes as sent;
- decoded by `src/decoder.rs` (the same automaton that reads GDB packets), which
  un-escapes, verifies the checksum and hands the payload to `rpc_dispatch()`
  (`src/swindle.rs`).

### Reply (probe → host)

```
'&' <status> <escaped CBOR payload> '#'
```

No checksum. `status` is `RPC_RESP_OK` `'K'`, `RPC_RESP_ERR` `'E'`,
`RPC_RESP_PARITY` `'P'` or `RPC_RESP_NOTSUP` `'N'`. The hosted transport
(`hosted/src/bmp_qtserial.cpp::platform_buffer_read`) drains up to the `&` and
stops at the `#`, `src/hosted/transport.rs` un-escapes the payload in place, and
the generated callers decode it, e.g.

```rust
match reply_u32(reply) {
    Some(value) => (0, value),
    None => (reply_fault(reply), 0),
}
```

An error frame carries the fault code (a CBOR integer) instead of the reply
fields: `reply_fault()` returns it, or `1` when the frame carries nothing. The
hosted side never checks a length any more — CBOR says where a field ends.

### Worked example — `ADIV5.MEM_READ`

```
host ->  "!" "A" "m"          class ADIV5, command MEM_READ
             0x85             array(5)
             0x00             device_index (u8) 0
             0x00             ap_selection (u8) 0
             0x1a 23 00 00 12 csw 0x23000012   (0x23 is escaped as "}\x03")
             0x1a 20 00 e1 00 address 0x2000e100
             0x04             length 4
         "#<checksum>"

probe -> "&K" 0x81 0x44 <4 data bytes> "#"      (or "&E" + fault code)
```

`0x81` is `array(1)`, `0x44` is `bytes(4)`, so the block travels verbatim.

---

## 4. Classes

| class     | id  | handler (`src/native/rpc_target_impl/`) | purpose                                                              |
| --------- | --- | --------------------------------------- | -------------------------------------------------------------------- |
| `GEN`     | `G` | `rpc_gen_impl`                          | session start, voltage, nRST, clock, frequency, power, GPIO reset    |
| `SWDP`    | `S` | `rpc_swdp_impl`                         | SWD init                                                             |
| `HL`      | `H` | `rpc_hl_impl`                           | protocol version (`C`) and acceleration bitmap (`A`)                 |
| `SDI`     | `I` | `rpc_sdi_impl`                          | WCH SDI: reset, Debug Module read/write                              |
| `RV`      | `B` | `rpc_rv_impl`                           | RISC-V DM read/write (RVSWD) + bulk memory blocks                    |
| `LNADIV`  | `L` | `rpc_lnadiv_impl`                       | LN-specific ADIv5 helpers                                            |
| `ADIV5`   | `A` | `rpc_adiv5_impl`                        | DP/AP raw access, memory read/write — the hot path of an ARM session |
| `SWINDLE` | `Z` | `rpc_swindle_impl`                      | probe extras (frequency, pins, RTT, logging)                         |
| `JTAG`    | `J` | `rpc_jtag_impl`                         | JTAG scan                                                            |
| `SPI`     | `s` | `rpc_spi_impl`                          | SPI transfers (flash)                                                |

A hosted `mon rvswdp_scan` / `mon sdi_scan` drives the `RV` / `SDI` classes
directly (`hosted/dmi_scan.rs`, `riscv_extra_sdi.rs`); an ARM session drives
`ADIV5` through the generated `remote_adiv5_*` callers.

### Bulk RISC-V memory access (`RV.MEM_READ` / `RV.MEM_WRITE`)

A DM memory access costs ~11 DMI transactions per word (DATA0, DATA1, command
write, busy polls), and one frame per transaction is what capped a hosted `load`
at 2 KB/s — 16 KiB cost ~45k round trips. These two leaves carry a whole block
per frame and let the **probe** run the access sequence
(`native/rpc_target_impl/rpc_rv_impl.rs` → `RiscvMem`), so the wire cost is the
payload:

| leg | before | after |
| --- | --- | --- |
| 16 KiB `load` (hosted) | 2.6 KB/s | 6.2 KB/s |
| 16 KiB `dump binary memory` (hosted) | 5.2 s (per-op) | 1.2 s |

* `method` is `MemoryAccessMethod::wire_id()` (`riscv/hart.rs`, shared by both
  halves): `0` abstract, `1` abstauto, `2` progbuf, `3` sysbus. The hosted side
  sends the method its scan selected, so the probe runs the same sequence it
  would have run op by op.
* `leg` is `DmiLeg::wire_id()` (`riscv/dmi.rs`): `0` RVSWD (`bmp_rv_*`), `1` SDI
  (`riscv_extra::bmp_sdi_*`). The per-op leaves split the legs into two classes;
  the bulk leaf carries the choice as a parameter, and the scan that attached the
  target records it (`bulk_mem::set_leg`).
* Blocks are at most 1024 bytes on the wire and the host sends 256 per frame
  (`hosted/bulk_mem.rs`), which fits both the probe reply buffer (1 KiB) and the
  hosted writer buffer (512 B) with the CBOR header on top.
* The probe side needs no hart: it maps the leg's DM entry points onto a
  `DmiTransport` and calls the shared `RiscvMem` helpers, so the two ends cannot
  drift. Re-flash the probe with the matching build — an older probe answers
  `RV.MEM_*` with an error frame, i.e. every hosted memory access fails loudly
  instead of silently falling back.
* `swindle_hosted --raw-dmi` puts the sequence back on the host (one round trip
  per DM op, the pre-bulk behaviour) — for a probe that does not know the leaves,
  and for comparing the two paths on the same target. The default is the block
  mode; the startup banner says which one is active. Every memory access goes
  through `RiscvHart::mem_read` / `mem_write` for that reason, including the
  target driver (`riscv/driver.rs`), so `load`, `dump binary memory` and `x` all
  follow the same choice.

---

## 5. Protocol version

`HL.CHECK` (`'H' 'C'`) returns the version this probe speaks. `0..=4` are the
upstream Black Magic remote protocol numbers; the probe used to answer `3`
because its ADIv5 payloads matched the upstream v3 layout (big-endian ASCII hex,
32-bit addresses), then `5` when the payloads moved to little-endian ASCII hex,
which matched neither v3 nor v4 (v4 is 64-bit / ADIv6). The payloads are CBOR
now, which matches no upstream version at all, so the probe answers **6**
(`native/rpc_target_impl/rpc_hl_impl.rs::check()`).

Nothing in Swindle reads that number back — the handshake exists for stock
`bmp_hosted` clients, which then fail with _"Unknown remote protocol version 6"_
instead of silently mis-decoding addresses. Bump it again whenever a payload
layout changes, and remember that after such a change the probe firmware must be
re-flashed before a new host binary may talk to it.

---

## 6. Adding or changing a command

1. Edit `rpc_protocol.toml`: pick an unused `id` **within the class** (ids are a
   single, case-sensitive byte), list the params in wire order, choose a reply.
2. Regenerate (§2).
3. Implement the function in `src/native/rpc_target_impl/<handler>.rs`. The
   generator dictates the signature: `u8`/`u16`/`u32` all arrive as `u32`,
   `bool` as `bool`, `hex_block` as `&[u8]`; the return type must match the reply
   (`bool` for `ok*`, `(i32, u32)` for `u32`, `(i32, &[u8])` for `hex_block`, a
   tuple for `compound`).
4. Call `remote_<class>_<command>()` from the host code (`src/hosted/...`).
5. Build both halves; if a payload changed, bump the version (§5) and re-flash
   the probe.
6. For SDI/RVSWD work keep the `*_riscv_extra*` separation described in
   `.clinerules`: a new SDI leaf goes in the seam, never in a generic file.

---

## 7. Where the code lives

| role                               | path                                                       |
| ---------------------------------- | ---------------------------------------------------------- |
| protocol definition                | `swindle/rs/rs_swindle/rpc_protocol.toml`                  |
| generator                          | `swindle/rs/rs_swindle/generate_rpc.py`                    |
| shared constants                   | `swindle/rs/rs_swindle/src/rpc_common_generated.rs`        |
| framing + escaping + CBOR payload  | `swindle/rs/rs_swindle/src/rpc_wire.rs`                    |
| probe dispatch                     | `swindle/rs/rs_swindle/src/native/rpc_target_generated.rs` |
| probe param parser / reply encoder | `src/native/rpc_target/rpc_parser.rs`, `rpc_reply.rs`      |
| probe command bodies               | `swindle/rs/rs_swindle/src/native/rpc_target_impl/*.rs`    |
| host client                        | `swindle/rs/rs_swindle/src/hosted/rpc_host_generated.rs`   |
| host request encoder               | `src/hosted/rpc_host/remote_encoder.rs`                    |
| host reply reader                  | `src/hosted/transport.rs`                                  |
| serial framing (host)              | `hosted/src/bmp_qtserial.cpp`                              |
| packet automaton (probe)           | `swindle/rs/rs_swindle/src/decoder.rs`                     |

## 8. Validating a change

```bash
cd swindle/rs/rs_swindle && cargo test --features hosted,std --lib   # host half
cd build && make -j$(nproc)                                          # probe half
cd build_nosdi && make -j$(nproc)                                    # SDI off still builds
```

Then exercise it for real: run `hosted/build/swindle_hosted` against a probe (see
the "hosted debug" section of `.clinerules`) and run a scan that uses the class
you touched — `mon rvswdp_scan` for `RV`, `mon sdi_scan` for `SDI`, a plain ARM
session for `ADIV5`. Generated-code diffs are worth reviewing like any other
code: `git diff swindle/rs/rs_swindle/src`.
