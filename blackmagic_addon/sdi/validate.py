#!/usr/bin/env python3
"""
Validate SDI flash writing on a Swindle/LN probe.

The probe is driven the way a session drives it: gdb over the probe's GDB CDC, with
`mon sdi_scan` to attach over the single-wire interface and then a `load` of a small image
at the flash base. Each try writes the image and reads it back in a *fresh* session - a
same-session dump cannot catch "the block verified, then got corrupted later", which is the
failure mode the CH32V00x hold exists for.

Usage:
    python3 blackmagic_addon/sdi/validate.py /dev/ttySwindleGdb2
    python3 blackmagic_addon/sdi/validate.py /dev/ttySwindleGdb2 --tries 5 --size 8192
    python3 blackmagic_addon/sdi/validate.py /dev/ttySwindleGdb2 --hold 0   # mon stub_hold 0

The region at --address is overwritten and then put back: it is dumped before the first
write and restored (and verified) at the end, unless --no-restore is given.
"""

from __future__ import annotations

import argparse
import re
import shutil
import struct
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass, field
from pathlib import Path

DEFAULT_GDB = "/riscv/xpack-14.2.0-2/bin/riscv-none-elf-gdb"
DEFAULT_ADDRESS = 0x08000000
DEFAULT_SIZE = 2048
DEFAULT_TRIES = 5
DEFAULT_MAX_ATTEMPTS = 3
DEFAULT_TIMEOUT = 180

# What gdb/the probe says when a write did not happen. Matched case-insensitively.
LOAD_ERRORS = (
    "error erasing flash",
    "error writing data to flash",
    "load failed",
    "cannot access memory",
    "remote failure reply",
)

RE_ACCESS = re.compile(r"Selected memory access method:\s*(\w+)")
RE_LOAD_SIZE = re.compile(r"load size (\d+)")
RE_RATE = re.compile(r"Transfer rate:\s*([\d.]+)\s*(KB|bytes)/sec")
# `mon riscv_benchmark`: which accessors this probe/DM actually completes a round trip with.
RE_BENCH = re.compile(r"^\s*(Sysbus|Abstract|AbstAuto|Progbuf)\s*:\s*(OK|FAILED|UNSUPPORTED)", re.M)
# `mon riscv_confirm`: the counters behind the memory path's own retries and fallbacks.
RE_CONFIRM = re.compile(
    r"\b(windows|disagreeing|elements|faults|latched_cmderr|autoexec_retries)=(\d+)")
RE_CONFIRM_LAST = re.compile(
    r"last @ 0x([0-9a-fA-F]+)\s+pass1=0x([0-9a-fA-F]+)\s+pass2=0x([0-9a-fA-F]+)")
# `mon stub_stats`: what the flash-stub runner and the drivers actually did.
RE_STUB = re.compile(r"\b(runs|halted|repairs|wait_ms|hold_ms)=(\d+)")
RE_STUB_UNRULY = re.compile(r"last unruly run pc=0x([0-9a-fA-F]+).*code=0x([0-9a-fA-F]+),0x([0-9a-fA-F]+)")


def log(message: str) -> None:
    print(message, flush=True)


# --------------------------------------------------------------------------------------
# Test image
# --------------------------------------------------------------------------------------

def pattern_bytes(size: int) -> bytes:
    """A deterministic, non-repeating image: word i is 0xA5A50000 | i (size % 4 == 0)."""
    return b"".join(struct.pack("<I", 0xA5A50000 | i) for i in range(size // 4))


def build_elf(binary: Path, address: int, work: Path, objcopy: str, ld: str) -> Path:
    """
    Wrap `binary` in a one-segment ELF at `address`, and prove the ELF really carries it.

    The two-step objcopy/ld dance has a trap: a linker script that only names `*(.text)`
    drops the payload of a `-I binary` object (it lives in `.data`), and the ELF then loads
    a page of zeros. That is checked rather than assumed - it once looked exactly like "the
    probe writes zeros".
    """
    obj = work / (binary.stem + ".o")
    elf = work / (binary.stem + ".elf")
    script = work / "image.ld"
    check = work / (binary.stem + ".check")

    script.write_text(
        f"SECTIONS\n{{\n  . = 0x{address:08x};\n  .text : {{ *(.data) *(.text) }}\n}}\n"
    )
    run([objcopy, "-I", "binary", "-O", "elf32-littleriscv", "-B", "riscv",
         str(binary), str(obj)])
    run([ld, "-T", str(script), "-o", str(elf), str(obj)])
    run([objcopy, "-O", "binary", "--only-section=.text", str(elf), str(check)])
    if check.read_bytes() != binary.read_bytes():
        raise SystemExit(
            f"refusing to run: {elf.name} does not carry the test image "
            "(the linker script lost it)"
        )
    return elf


def run(argv: list[str], timeout: float = DEFAULT_TIMEOUT) -> subprocess.CompletedProcess:
    return subprocess.run(argv, capture_output=True, text=True, errors="replace",
                          timeout=timeout)


# --------------------------------------------------------------------------------------
# gdb plumbing
# --------------------------------------------------------------------------------------

@dataclass
class GdbSession:
    gdb: str
    device: str
    timeout: float = DEFAULT_TIMEOUT

    def run(self, commands: list[str]) -> tuple[bool, str]:
        """Run gdb in batch mode with `commands`; return (ok, log text)."""
        argv = [self.gdb, "-batch",
                "-ex", "set pagination off",
                "-ex", "set confirm off",
                "-ex", "set remotetimeout 10",
                "-ex", "set mem inaccessible-by-default off",
                "-ex", f"target extended-remote {self.device}"]
        for command in commands:
            argv += ["-ex", command]
        argv += ["-ex", "detach"]
        try:
            result = run(argv, self.timeout)
        except subprocess.TimeoutExpired:
            return False, f"gdb timed out after {self.timeout:.0f} s"
        return result.returncode == 0, (result.stdout or "") + (result.stderr or "")


def load_errors_in(log_text: str) -> list[str]:
    lowered = log_text.lower()
    return [needle for needle in LOAD_ERRORS if needle in lowered]


# --------------------------------------------------------------------------------------
# One load, one verification
# --------------------------------------------------------------------------------------

@dataclass
class LoadResult:
    ok: bool
    method: str = "?"
    size: int = 0
    elapsed: float = 0.0
    gdb_rate: float | None = None
    errors: list[str] = field(default_factory=list)
    log_text: str = ""
    # `mon riscv_confirm`, read after the load in the same session: the memory path's own
    # retries. Empty when the firmware has no such command.
    confirm: dict[str, int] = field(default_factory=dict)
    last_mismatch: tuple[int, int, int] | None = None
    # `mon stub_stats`, read after the load in the same session: what the flash path did
    # underneath (stub runs, halts seen, register-repair fallbacks).
    stub: dict[str, int] = field(default_factory=dict)
    unruly: tuple[int, int, int] | None = None

    @property
    def speed(self) -> float:
        """Bytes/second for this load, measured on the wall clock."""
        return self.size / self.elapsed if self.elapsed > 0 and self.size else 0.0


def do_load(session: GdbSession, elf: Path, hold: int | None) -> LoadResult:
    # Both counter sets are reset first, so what is read back after the load is this try's.
    commands = ["monitor riscv_confirm reset", "monitor stub_stats reset"]
    if hold is not None:
        commands.append(f"monitor stub_hold {hold}")
    commands += ["monitor sdi_scan", "attach 1", f"load {elf}",
                 "monitor riscv_confirm", "monitor stub_stats"]
    started = time.monotonic()
    ok, text = session.run(commands)
    elapsed = time.monotonic() - started

    result = LoadResult(ok=False, elapsed=elapsed, log_text=text)
    if match := RE_ACCESS.search(text):
        result.method = match.group(1)
    if match := RE_LOAD_SIZE.search(text):
        result.size = int(match.group(1))
    if match := RE_RATE.search(text):
        value, unit = float(match.group(1)), match.group(2)
        result.gdb_rate = value * 1024 if unit == "KB" else value
    result.errors = load_errors_in(text)
    result.ok = ok and not result.errors and result.size > 0
    # Later occurrences win: the reset line prints the cleared counters before the load.
    result.confirm = {name: int(value) for name, value in RE_CONFIRM.findall(text)}
    if match := RE_CONFIRM_LAST.search(text):
        result.last_mismatch = tuple(int(part, 16) for part in match.groups())  # type: ignore[assignment]
    result.stub = {name: int(value) for name, value in RE_STUB.findall(text)}
    if match := RE_STUB_UNRULY.search(text):
        result.unruly = tuple(int(part, 16) for part in match.groups())  # type: ignore[assignment]
    return result


def probe_accessors(session: GdbSession) -> dict[str, str]:
    """
    Ask the probe/DM which accessors work, with `mon riscv_benchmark`.

    The scan picks one accessor and moves on; this is the only place that says whether a
    *fallback* happened, i.e. whether the one it picked was the first choice or a survivor.
    """
    ok, text = session.run(["monitor sdi_scan", "attach 1", "monitor riscv_benchmark"])
    if not ok:
        return {}
    return {name: state for name, state in RE_BENCH.findall(text)}


def do_read(session: GdbSession, address: int, size: int, out: Path) -> tuple[bool, bytes]:
    """
    Read the region back in a session of its own.

    The scan and the attach are part of it: a `dump` with no target attached reads nothing,
    which is how the first version of this script "failed to back up" without ever touching
    the target.
    """
    out.unlink(missing_ok=True)
    ok, _ = session.run([
        "monitor sdi_scan",
        "attach 1",
        f"dump binary memory {out} 0x{address:08x} 0x{address + size:08x}",
    ])
    if not ok or not out.exists():
        return False, b""
    return True, out.read_bytes()


# --------------------------------------------------------------------------------------
# The test
# --------------------------------------------------------------------------------------

def collect_confirm(results: list[LoadResult]) -> dict[str, int]:
    """Sum the `mon riscv_confirm` counters over the tries (empty when none reported them)."""
    totals: dict[str, int] = {}
    for result in results:
        for name, value in result.confirm.items():
            totals[name] = totals.get(name, 0) + value
    return totals


def collect_stub(results: list[LoadResult]) -> dict[str, int]:
    """Sum the `mon stub_stats` counters over the tries (empty when none reported them)."""
    totals: dict[str, int] = {}
    for result in results:
        for name, value in result.stub.items():
            totals[name] = totals.get(name, 0) + value
    return totals


def validate(args: argparse.Namespace) -> int:
    work = Path(tempfile.mkdtemp(prefix="sdi_validate_"))
    objcopy = args.objcopy or str(Path(args.gdb).with_name("riscv-none-elf-objcopy"))
    ld = args.ld or str(Path(args.gdb).with_name("riscv-none-elf-ld"))
    for tool in (args.gdb, objcopy, ld):
        if not shutil.which(tool):
            raise SystemExit(f"missing tool: {tool}")

    session = GdbSession(args.gdb, args.device, args.timeout)
    image = pattern_bytes(args.size)
    (work / "image.bin").write_bytes(image)
    elf = build_elf(work / "image.bin", args.address, work, objcopy, ld)

    log("SDI flash validation")
    log(f"  device        : {args.device}")
    log(f"  image         : {args.size} B at 0x{args.address:08x}")
    log(f"  tries         : {args.tries} (max {args.max_attempts} load attempts each)")
    hold = "part default" if args.hold is None else f"{args.hold} ms/page"
    log(f"  stub hold     : {hold}")
    log(f"  work dir      : {work}")
    log("")

    # Which accessors the DM completes a round trip with, before anything is written: this is
    # what turns "method=Progbuf" into "Abstract failed, Progbuf took over".
    accessors = probe_accessors(session)
    if accessors:
        table = ", ".join(f"{name}={state}" for name, state in accessors.items())
        log(f"  accessors     : {table}")
        usable = [name for name, state in accessors.items() if state == "OK"]
        log(f"  usable        : {', '.join(usable) if usable else 'none'}")
    else:
        log("  accessors     : not reported by this firmware (no 'mon riscv_benchmark')")
    log("")

    # Back up the region first: the test is destructive, and doing it through the same
    # mechanism means a target that cannot be read is refused instead of silently wiped.
    backup_elf: Path | None = None
    if not args.no_restore:
        ok, original = do_read(session, args.address, args.size, work / "backup.bin")
        if not ok:
            raise SystemExit("could not read the region to back it up; refusing to write")
        (work / "backup.bin").write_bytes(original)
        backup_elf = build_elf(work / "backup.bin", args.address, work, objcopy, ld)
        log(f"backup of {args.size} B at 0x{args.address:08x} -> {work / 'backup.bin'}")
        log("")

    methods: dict[str, int] = {}
    speeds: list[float] = []
    results: list[LoadResult] = []
    total_attempts = 0
    successes = 0
    started_all = time.monotonic()

    for index in range(1, args.tries + 1):
        attempt = 0
        result: LoadResult | None = None
        verified = False
        while attempt < args.max_attempts:
            attempt += 1
            result = do_load(session, elf, args.hold)
            total_attempts += 1
            if not result.ok:
                continue
            verified, content = do_read(session, args.address, args.size,
                                        work / f"read_{index}_{attempt}.bin")
            if verified and content == image:
                break
            # A mismatch here is late corruption *or* a lost read; either way the try is not
            # proven, so it costs an attempt rather than a verdict.
            verified = False
        assert result is not None
        results.append(result)

        methods[result.method] = methods.get(result.method, 0) + 1
        if result.ok and verified:
            successes += 1
            speeds.append(result.speed)
            state = "ok  "
        else:
            state = "FAIL"
        errors = "" if not result.errors else f"  errors={','.join(result.errors)}"
        log(f"try {index:2d}: {state} method={result.method:<9} load={result.size:5d} B  "
            f"{result.elapsed:6.2f} s  {result.speed:7.0f} B/s  attempts={attempt}{errors}")
        if result.confirm:
            keys = ("windows", "disagreeing", "faults", "latched_cmderr", "autoexec_retries")
            parts = " ".join(f"{key.split('_')[0]}={result.confirm.get(key, 0)}" for key in keys)
            note = ""
            if result.last_mismatch:
                addr, first, second = result.last_mismatch
                # `riscv_confirm reset` prints a zeroed "last" line; only a real one matters.
                if addr or first != second:
                    note = f"  last_mismatch=0x{addr:08x} {first:#010x}!={second:#010x}"
            log(f"          internals  : {parts}{note}")
        if result.stub:
            runs, halted = result.stub.get("runs", 0), result.stub.get("halted", 0)
            repairs = result.stub.get("repairs", 0)
            flag = ""
            if repairs:
                flag += "  <-- register-repair fallback used"
            if runs != halted:
                flag += "  <-- a stub run never reported its halt"
            if result.unruly:
                pc, code0, code1 = result.unruly
                flag += f"  (last unruly run pc=0x{pc:08x} code={code0:#010x},{code1:#010x})"
            log(f"          flash path : stub_runs={runs} halted={halted} repairs={repairs} "
                f"wait={result.stub.get('wait_ms', 0)} ms hold={result.stub.get('hold_ms', 0)} ms{flag}")

    total_elapsed = time.monotonic() - started_all
    retries = total_attempts - successes

    log("")
    log("summary")
    log(f"  successes      : {successes}/{args.tries}")
    if methods:
        spread = ", ".join(f"{name} x{count}" for name, count in sorted(methods.items()))
        log(f"  access method  : {spread}")
    if speeds:
        log(f"  average speed  : {sum(speeds) / len(speeds):.0f} B/s "
            f"(min {min(speeds):.0f}, max {max(speeds):.0f}, over successful loads)")
        log(f"  bytes written  : {args.size * successes} B")
    log(f"  total time     : {total_elapsed:.2f} s")
    log(f"  load attempts  : {total_attempts} for {successes} verified loads "
        f"({retries} retr{'y' if retries == 1 else 'ies'})")

    # The retries the probe did on its own, which the per-try wall clock cannot show.
    if totals := collect_confirm(results):
        log("  internal retries (read confirmation: a lost SDI read is re-read and compared)")
        log("    windows    : {}   disagreeing: {}   elements settled: {}".format(
            totals["windows"], totals["disagreeing"], totals["elements"]))
        log("    faults     : {}   latched cmderr: {}   autoexec retries: {}".format(
            totals["faults"], totals["latched_cmderr"], totals["autoexec_retries"]))
    if stub_totals := collect_stub(results):
        log("  flash path (per-session counters, summed over the tries)")
        log("    stub runs  : {}   halts seen: {}   register repairs: {}".format(
            stub_totals.get("runs", 0), stub_totals.get("halted", 0),
            stub_totals.get("repairs", 0)))
        log("    waiting    : {} ms (stub polling)   holding: {} ms".format(
            stub_totals.get("wait_ms", 0), stub_totals.get("hold_ms", 0)))
        if stub_totals.get("repairs", 0):
            log("    WARNING    : a block fell back to the register path - the stub path is broken")
    log("  not visible  : the SDI tap's own frame retries (a read whose value is all ones is")
    log("                 re-read in the tap) - nothing counts those on the probe today.")

    status = 0 if successes == args.tries else 1
    if backup_elf is not None:
        put_back = do_load(session, backup_elf, args.hold)
        ok, content = do_read(session, args.address, args.size, work / "restored.bin")
        same = ok and content == (work / "backup.bin").read_bytes()
        log("")
        log("  restore        : " + ("ok, region byte-identical" if same else
                                     "FAILED - written back, but the read-back differs"))
        if not put_back.ok or not same:
            status = 2

    if args.keep:
        log(f"  work dir kept  : {work}")
    else:
        shutil.rmtree(work, ignore_errors=True)
    return status


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Validate SDI flash writing through a Swindle/LN probe, using gdb.",
    )
    parser.add_argument("device", help="probe GDB CDC, e.g. /dev/ttySwindleGdb2")
    parser.add_argument("--tries", type=int, default=DEFAULT_TRIES,
                        help=f"number of writes to perform (default {DEFAULT_TRIES})")
    parser.add_argument("--size", type=int, default=DEFAULT_SIZE,
                        help=f"image size in bytes, multiple of 4 (default {DEFAULT_SIZE})")
    parser.add_argument("--address", default=hex(DEFAULT_ADDRESS),
                        help=f"flash address to write (default {hex(DEFAULT_ADDRESS)})")
    parser.add_argument("--hold", type=int, default=None,
                        help="send 'mon stub_hold <ms>' before each load; omit for the part's value")
    parser.add_argument("--max-attempts", type=int, default=DEFAULT_MAX_ATTEMPTS,
                        help=f"load attempts per try (default {DEFAULT_MAX_ATTEMPTS})")
    parser.add_argument("--gdb", default=DEFAULT_GDB, help=f"gdb binary (default {DEFAULT_GDB})")
    parser.add_argument("--objcopy", default=None, help="riscv objcopy (default: next to --gdb)")
    parser.add_argument("--ld", default=None, help="riscv ld (default: next to --gdb)")
    parser.add_argument("--timeout", type=float, default=DEFAULT_TIMEOUT,
                        help=f"per-gdb-call timeout, seconds (default {DEFAULT_TIMEOUT})")
    parser.add_argument("--no-restore", action="store_true",
                        help="leave the test pattern in flash (no backup/restore)")
    parser.add_argument("--keep", action="store_true", help="keep the temporary files")
    args = parser.parse_args(argv)

    if args.size % 4 or args.size < 4:
        parser.error("--size must be a multiple of 4")
    args.address = int(args.address, 0)
    return validate(args)


if __name__ == "__main__":
    sys.exit(main())
