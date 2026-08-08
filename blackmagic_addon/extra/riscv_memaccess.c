/*
 * This file is part of the swindle project.
 *
 * riscv_memaccess.c — RISC-V memory access auto-selector.
 *
 * At probe time riscv32_benchmark_register() (called from riscv32_probe())
 * benchmarks the memory access methods a RISC-V debug module can offer:
 *
 *   Abstract  - word-at-a-time abstract commands (riscv32.c default path)
 *   AbstAuto  - abstract commands with AUTOEXECDATA_0 streaming (~1 DMI/word)
 *   Progbuf   - load/store code run in the program buffer (riscv32.c)
 *   Sysbus    - debug-module system-bus master (riscv32.c)
 *
 * Each method is scored by the DMI transaction count needed to round-trip a
 * 1-word and a RISCV_BENCH_WORDS-word buffer through SRAM. The cheapest
 * method that round-trips correctly is selected, and target->mem_read/write
 * are hooked to the dispatchers below.
 *
 * The dispatcher exists because riscv32_mem_read/write() only know the
 * SYSBUS/PROGBUF routing flags: if the benchmark picked AbstAuto the
 * RV_HART_FLAG_MEMORY_ABSTAUTO bit would be invisible to them and every
 * access would silently fall back to plain word-wise abstract. So while that
 * bit is set we route here: short transfers go word-wise (streaming's fixed
 * setup/teardown cost only pays off beyond RISCV_BENCH_SHORT_XFER_MAX bytes),
 * longer transfers use the streaming path, everything else defers to
 * riscv32_mem_read/write().
 *
 * `mon riscv_benchmark` (riscv32_run_benchmark()) prints the raw numbers;
 * `mon riscv_benchmark2` (riscv32_run_benchmark2()) re-runs the same round
 * trips through the generic target API only (target_mem64_write/read) so the
 * "best" path picked by the selector can be verified end to end.
 */
#include "general.h"
#include "target.h"
#include "target_internal.h"
#include "riscv_debug.h"
#include "command.h"
#include "gdb_packet.h"

extern uint32_t bmp_rv_tx_count_c(void);
extern void bmp_rv_tx_reset_c(void);
/* Rust scratch buffer accessor (1024-byte single-shot buffer) + esprit panic hook */
extern uint8_t *get_temp_buffer_c(uint32_t size);
extern void do_assert(const char *a);

/* RV_HART_FLAG_MEMORY_ABSTAUTO routing flag is shared from riscv_debug.h */

/* --- Tunables -------------------------------------------------------------- */

#define RISCV_BENCH_SCRATCH_ADDR 0x20000000U /* generic SRAM base used for the round trips */
#define RISCV_BENCH_TEST_VALUE 0xa5a55a5aU
#define RISCV_BENCH_WORDS 64U /* bulk round-trip length, in words */
#define RISCV_BENCH_SHORT_XFER_MAX 16U /* below this, word-wise beats streaming */
#define RISCV_BENCH_HALT_TIMEOUT_MS 2000U
#define RISCV_BENCH_BEST_TX_INIT 0xFFFFFFFFU
#define RISCV_PROGBUF_PROBE_MAX_SLOTS 16U
#define RISCV_PROGBUF_PROBE_TEST_VALUE 0x12345678U

typedef enum riscv_bench_method {
	RISCV_BENCH_ABSTRACT = 0,
	RISCV_BENCH_ABSTAUTO,
	RISCV_BENCH_PROGBUF,
	RISCV_BENCH_SYSBUS,
} riscv_bench_method_e;

/* --- Abstract drivers ------------------------------------------------------ */

/* Build the access-memory abstract command used by both abstract paths below */
static uint32_t riscv32_abstract_command(const uint8_t access_width, const bool is_write)
{
	return RV_DM_ABST_CMD_ACCESS_MEM | (is_write ? RV_ABST_WRITE : RV_ABST_READ) |
		((uint32_t)access_width << RV_ABST_MEM_ACCESS_SHIFT) | RV_ABST_MEM_ADDR_POST_INC;
}

/* --- Abstract DM register accessors ------------------------------------------
 * Named wrappers over riscv_dm_read/write() that pin each DM register slot,
 * so the driver loops below read like the RISC-V debug spec ("write abstract
 * arg0", "issue the abstract command", ...). Purely stateless: the hart (and
 * its DM handle) is always passed in explicitly. */

static bool riscv32_abstract_a0_write(riscv_hart_s *const hart, const uint32_t value)
{
	return riscv_dm_write(hart->dbg_module, RV_DM_DATA0, value);
}

static bool riscv32_abstract_a0_read(riscv_hart_s *const hart, uint32_t *const value)
{
	return riscv_dm_read(hart->dbg_module, RV_DM_DATA0, value);
}

static bool riscv32_abstract_a1_write(riscv_hart_s *const hart, const uint32_t value)
{
	return riscv_dm_write(hart->dbg_module, RV_DM_DATA1, value);
}

static bool riscv32_abstract_cmd_write(riscv_hart_s *const hart, const uint32_t command)
{
	return riscv_dm_write(hart->dbg_module, RV_DM_ABST_COMMAND, command);
}

static bool riscv32_abstract_autoexec_write(riscv_hart_s *const hart, const uint32_t value)
{
	return riscv_dm_write(hart->dbg_module, RV_DM_ABSTRACTAUTO, value);
}

/* Word-wise abstract: one full command + wait per element, no autoexec.
 * Cheapest for short transfers, where streaming's fixed setup/teardown cost
 * would dominate (see riscv32_bench_mem_write/read). */
static void riscv32_abstract_wordwise_mem_write(
	riscv_hart_s *const hart, const target_addr64_t dest, const void *const src, const size_t len)
{
	if (!len) return;
	const uint8_t access_width = riscv_mem_access_width(hart, dest, len);
	const uint8_t access_length = 1U << access_width;
	const uint32_t command = riscv32_abstract_command(access_width, true);

	if (!riscv32_abstract_a1_write(hart, dest)) return;
	const uint8_t *const data = (const uint8_t *)src;

	for (size_t offset = 0; offset < len; offset += access_length) {
		uint32_t value = riscv32_pack_data(data + offset, access_width);
		if (!riscv32_abstract_a0_write(hart, value)) return;
		if (!riscv32_abstract_cmd_write(hart, command) ||
			!riscv_command_wait_complete(hart)) return;
	}
}

static void riscv32_abstract_wordwise_mem_read(
	riscv_hart_s *const hart, void *const dest, const target_addr64_t src, const size_t len)
{
	if (!len) return;
	const uint8_t access_width = riscv_mem_access_width(hart, src, len);
	const uint8_t access_length = 1U << access_width;
	const uint32_t command = riscv32_abstract_command(access_width, false);

	if (!riscv32_abstract_a1_write(hart, src)) return;
	uint8_t *const data = (uint8_t *)dest;

	for (size_t offset = 0; offset < len; offset += access_length) {
		if (!riscv32_abstract_cmd_write(hart, command) ||
			!riscv_command_wait_complete(hart)) return;
		uint32_t value = 0;
		if (!riscv32_abstract_a0_read(hart, &value)) return;
		riscv32_unpack_data(data + offset, value, access_width);
	}
}

/* Streaming abstract: prime one command, then let AUTOEXECDATA_0 re-run it on
 * every DATA0 access while the address advances via POST_INC (~1 DMI/word).
 * Pure-abstract twin of the program-buffer streaming in riscv32.c. */
static void riscv32_abstract_streaming_mem_write(
	target_s *const target, const target_addr64_t dest, const void *const src, const size_t len)
{
	if (!len) return;
	riscv_hart_s *const hart = riscv_hart_struct(target);
	const uint8_t access_width = riscv_mem_access_width(hart, dest, len);
	const uint8_t access_length = 1U << access_width;
	const uint32_t command = riscv32_abstract_command(access_width, true);

	if (!riscv32_abstract_a1_write(hart, dest)) return;
	const uint8_t *const data = (const uint8_t *)src;

	/* First element goes through manually to prime the autoexec pipeline */
	uint32_t value = riscv32_pack_data(data, access_width);
	if (!riscv32_abstract_a0_write(hart, value)) return;
	if (!riscv32_abstract_cmd_write(hart, command) ||
		!riscv_command_wait_complete(hart)) return;

	if (!riscv32_abstract_autoexec_write(hart, RV_ABSTRACTAUTO_AUTOEXECDATA_0)) return;
	for (size_t offset = access_length; offset < len; offset += access_length) {
		value = riscv32_pack_data(data + offset, access_width);
		if (!riscv32_abstract_a0_write(hart, value)) {
			riscv32_abstract_autoexec_write(hart, 0U);
			return;
		}
	}
	riscv32_abstract_autoexec_write(hart, 0U);
	riscv_command_wait_complete(hart);
}

static void riscv32_abstract_streaming_mem_read(
	target_s *const target, void *const dest, const target_addr64_t src, const size_t len)
{
	if (!len) return;
	riscv_hart_s *const hart = riscv_hart_struct(target);
	const uint8_t access_width = riscv_mem_access_width(hart, src, len);
	const uint8_t access_length = 1U << access_width;
	const uint32_t command = riscv32_abstract_command(access_width, false);

	if (!riscv32_abstract_a1_write(hart, src)) return;
	if (!riscv32_abstract_cmd_write(hart, command) ||
		!riscv_command_wait_complete(hart)) return;

	uint8_t *const data = (uint8_t *)dest;
	uint32_t value = 0;
	if (!riscv32_abstract_autoexec_write(hart, RV_ABSTRACTAUTO_AUTOEXECDATA_0)) return;
	for (size_t offset = 0; offset < len - access_length; offset += access_length) {
		if (!riscv32_abstract_a0_read(hart, &value)) {
			riscv32_abstract_autoexec_write(hart, 0U);
			return;
		}
		riscv32_unpack_data(data + offset, value, access_width);
	}
	riscv32_abstract_autoexec_write(hart, 0U);

	/* Last element: drain the pipeline after autoexec is switched off */
	if (riscv32_abstract_a0_read(hart, &value))
		riscv32_unpack_data(data + len - access_length, value, access_width);
}

/* --- Method registry ------------------------------------------------------- */

typedef void (*riscv_bench_write_fn)(target_s *target, target_addr64_t dest, const void *src, size_t len);
typedef void (*riscv_bench_read_fn)(target_s *target, void *dest, target_addr64_t src, size_t len);

typedef struct riscv_bench_method_entry {
	riscv_bench_method_e id;
	const char *name;
	uint32_t flag;                               /* routing bit(s) for this method */
	bool (*supported)(const riscv_hart_s *hart); /* NULL = always supported */
	riscv_bench_write_fn write;
	riscv_bench_read_fn read;
} riscv_bench_method_s;

static bool riscv_method_progbuf_supported(const riscv_hart_s *const hart)
{
	return hart->progbuf_size > 0;
}

static bool riscv_method_sysbus_supported(const riscv_hart_s *const hart)
{
	return (hart->flags & RV_HART_CAP_HAS_SYSBUS) != 0;
}

/*
 * Abstract/Progbuf/Sysbus share riscv32_mem_read/write(): routing there is
 * purely a matter of which flag riscv_bench_method_select() leaves set.
 * AbstAuto is the only method implemented locally in this file.
 */
static const riscv_bench_method_s riscv_bench_methods[] = {
	{RISCV_BENCH_ABSTRACT, "Abstract", 0, NULL, riscv32_mem_write, riscv32_mem_read},
	{RISCV_BENCH_ABSTAUTO, "AbstAuto", RV_HART_FLAG_MEMORY_ABSTAUTO, NULL,
		riscv32_abstract_streaming_mem_write, riscv32_abstract_streaming_mem_read},
	{RISCV_BENCH_PROGBUF, "Progbuf", RV_HART_FLAG_MEMORY_PROGBUF, riscv_method_progbuf_supported,
		riscv32_mem_write, riscv32_mem_read},
	{RISCV_BENCH_SYSBUS, "Sysbus", RV_HART_FLAG_MEMORY_SYSBUS, riscv_method_sysbus_supported,
		riscv32_mem_write, riscv32_mem_read},
};

static void riscv_bench_method_select(riscv_hart_s *const hart, const riscv_bench_method_s *const method)
{
	/* Clear existing memory routing bits, then apply the method's routing flag */
	hart->flags &= ~(RV_HART_FLAG_MEMORY_SYSBUS | RV_HART_FLAG_MEMORY_PROGBUF | RV_HART_FLAG_MEMORY_ABSTAUTO);
	hart->flags |= method->flag;
}

static bool riscv_bench_method_supported(const riscv_hart_s *const hart, const riscv_bench_method_s *const method)
{
	return method->supported == NULL || method->supported(hart);
}

/* --- Benchmark harness ----------------------------------------------------- */

typedef struct riscv_bench_result {
	uint32_t tx_single;
	uint32_t tx_bulk;
	bool ok_single;
	bool ok_bulk;
} riscv_bench_result_s;

/* Round-trip `method` once (1 word + RISCV_BENCH_WORDS words) through the
 * scratch SRAM address, reporting DMI transaction counts and whether the data
 * survived the trip unchanged and error-free. */
static riscv_bench_result_s riscv_bench_method_roundtrip(
	target_s *const target, riscv_hart_s *const hart, const riscv_bench_method_s *const method)
{
	riscv_bench_result_s result = {0};
	const uint32_t test_value = RISCV_BENCH_TEST_VALUE;
	riscv_bench_method_select(hart, method);

	/* 1-word round trip */
	bmp_rv_tx_reset_c();
	uint32_t single_read = 0;
	method->write(target, RISCV_BENCH_SCRATCH_ADDR, &test_value, sizeof(test_value));
	method->read(target, &single_read, RISCV_BENCH_SCRATCH_ADDR, sizeof(test_value));
	result.tx_single = bmp_rv_tx_count_c();
	result.ok_single = !target_check_error(target) && single_read == test_value;

	/* Bulk round trip */
	bmp_rv_tx_reset_c();
	uint32_t *const buf =
		(uint32_t *)get_temp_buffer_c(sizeof(uint32_t) * RISCV_BENCH_WORDS);
	if (!buf)
		do_assert("riscv_benchmark: get_temp_buffer_c() too small");
	for (size_t i = 0; i < RISCV_BENCH_WORDS; ++i)
		buf[i] = test_value + (uint32_t)i;
	method->write(target, RISCV_BENCH_SCRATCH_ADDR, buf, sizeof(uint32_t) * RISCV_BENCH_WORDS);
	method->read(target, buf, RISCV_BENCH_SCRATCH_ADDR, sizeof(uint32_t) * RISCV_BENCH_WORDS);
	result.tx_bulk = bmp_rv_tx_count_c();

	result.ok_bulk = !target_check_error(target);
	for (size_t i = 0; i < RISCV_BENCH_WORDS; ++i) {
		if (buf[i] != test_value + (uint32_t)i) result.ok_bulk = false;
	}
	return result;
}

/* --- Runtime dispatcher (hooked onto target->mem_read/write) --------------- */

/* Recover the benchmark-selected method from the hart routing flags. The
 * dispatcher can't just defer to riscv32_mem_* when AbstAuto won: those
 * functions only know the SYSBUS/PROGBUF routing flags, so the
 * RV_HART_FLAG_MEMORY_ABSTAUTO bit would silently drop every access back to
 * plain word-wise abstract. */
static riscv_bench_method_e riscv_bench_active_method(const riscv_hart_s *const hart)
{
	if (hart->flags & RV_HART_FLAG_MEMORY_ABSTAUTO)
		return RISCV_BENCH_ABSTAUTO;
	if (hart->flags & RV_HART_FLAG_MEMORY_PROGBUF)
		return RISCV_BENCH_PROGBUF;
	if (hart->flags & RV_HART_FLAG_MEMORY_SYSBUS)
		return RISCV_BENCH_SYSBUS;
	return RISCV_BENCH_ABSTRACT;
}

static void riscv32_bench_mem_write(
	target_s *const target, const target_addr64_t dest, const void *const src, const size_t len)
{
	riscv_hart_s *const hart = riscv_hart_struct(target);
	if (!hart) return riscv32_mem_write(target, dest, src, len);

	/* AbstAuto is handled here: short transfers go word-wise (streaming's
	 * fixed setup/teardown cost only pays off beyond
	 * RISCV_BENCH_SHORT_XFER_MAX bytes). Everything else defers to
	 * riscv32_mem_write(), which routes SYSBUS/PROGBUF/plain abstract. */
	if (riscv_bench_active_method(hart) == RISCV_BENCH_ABSTAUTO) {
		if (len <= RISCV_BENCH_SHORT_XFER_MAX)
			riscv32_abstract_wordwise_mem_write(hart, dest, src, len);
		else
			riscv32_abstract_streaming_mem_write(target, dest, src, len);
		return;
	}
	riscv32_mem_write(target, dest, src, len);
}

static void riscv32_bench_mem_read(
	target_s *const target, void *const dest, const target_addr64_t src, const size_t len)
{
	riscv_hart_s *const hart = riscv_hart_struct(target);
	if (!hart) return riscv32_mem_read(target, dest, src, len);

	if (riscv_bench_active_method(hart) == RISCV_BENCH_ABSTAUTO) {
		if (len <= RISCV_BENCH_SHORT_XFER_MAX)
			riscv32_abstract_wordwise_mem_read(hart, dest, src, len);
		else
			riscv32_abstract_streaming_mem_read(target, dest, src, len);
		return;
	}
	riscv32_mem_read(target, dest, src, len);
}

/* --- Halt helper ----------------------------------------------------------- */

static bool riscv_bench_ensure_halted(target_s *const target)
{
	target_halt_reason_e reason = target_halt_poll(target, NULL);
	if (reason != TARGET_HALT_RUNNING)
		return reason != TARGET_HALT_ERROR;

	target_halt_request(target);
	platform_timeout_s timeout;
	platform_timeout_set(&timeout, RISCV_BENCH_HALT_TIMEOUT_MS);

	do {
		reason = target_halt_poll(target, NULL);
	} while (reason == TARGET_HALT_RUNNING && !platform_timeout_is_expired(&timeout));

	return reason != TARGET_HALT_RUNNING && reason != TARGET_HALT_ERROR;
}

/* --- Entry points ---------------------------------------------------------- */

/* `mon riscv_benchmark`: report raw DMI counts for every supported method. */
bool riscv32_run_benchmark(target_s *target)
{
	riscv_hart_s *const hart = riscv_hart_struct(target);
	if (!hart) {
		gdb_outf("Error: RISC-V hart structure is NULL.\n");
		return false;
	}

	if (!riscv_bench_ensure_halted(target)) {
		gdb_outf("Failed to halt target\n");
		return false;
	}

	const uint16_t probed_flags = hart->flags;
	gdb_outf("Running RISC-V memory benchmark at 0x%08x\n", RISCV_BENCH_SCRATCH_ADDR);

	for (size_t i = 0; i < ARRAY_LENGTH(riscv_bench_methods); ++i) {
		const riscv_bench_method_s *const method = &riscv_bench_methods[i];
		gdb_outf("%-10s: ", method->name);
		if (!riscv_bench_method_supported(hart, method)) {
			gdb_outf("UNSUPPORTED\n");
			continue;
		}

		const riscv_bench_result_s result = riscv_bench_method_roundtrip(target, hart, method);
		if (result.ok_single && result.ok_bulk)
			gdb_outf("OK (1-word: %lu DMI, 64-word: %lu DMI)\n",
				(unsigned long)result.tx_single, (unsigned long)result.tx_bulk);
		else
			gdb_outf("FAILED\n");
	}

	hart->flags = probed_flags;
	gdb_outf("Restored original memory method flags.\n");
	return true;
}

/* `mon riscv_benchmark2`: identical 1-word + bulk round trips as above, but
 * performed only through the generic target memory API (target_mem64_write /
 * target_mem64_read) — the same entry point the GDB stub, flash driver, etc.
 * use. Since that path dispatches through target->mem_write/mem_read (our
 * runtime dispatcher when the auto-select ran), the DMI counts must match the
 * best method reported by `mon riscv_benchmark` if the normal API really is
 * using the selected "best" path. Prints exactly one result line. */
bool riscv32_run_benchmark2(target_s *target)
{
	riscv_hart_s *const hart = riscv_hart_struct(target);
	if (!hart) {
		gdb_outf("Error: RISC-V hart structure is NULL.\n");
		return false;
	}

	if (!riscv_bench_ensure_halted(target)) {
		gdb_outf("Failed to halt target\n");
		return false;
	}

	const uint32_t test_value = RISCV_BENCH_TEST_VALUE;
	const char *const method_name = riscv_bench_methods[riscv_bench_active_method(hart)].name;

	/* 1-word round trip through the generic target API */
	bmp_rv_tx_reset_c();
	uint32_t single_read = 0;
	target_mem64_write(target, RISCV_BENCH_SCRATCH_ADDR, &test_value, sizeof(test_value));
	target_mem64_read(target, &single_read, RISCV_BENCH_SCRATCH_ADDR, sizeof(test_value));
	const uint32_t tx_single = bmp_rv_tx_count_c();
	const bool ok_single = !target_check_error(target) && single_read == test_value;

	/* Bulk round trip through the generic target API */
	bmp_rv_tx_reset_c();
	uint32_t *const buf = (uint32_t *)get_temp_buffer_c(sizeof(uint32_t) * RISCV_BENCH_WORDS);
	if (!buf)
		do_assert("riscv_benchmark2: get_temp_buffer_c() too small");
	for (size_t i = 0; i < RISCV_BENCH_WORDS; ++i)
		buf[i] = test_value + (uint32_t)i;
	target_mem64_write(target, RISCV_BENCH_SCRATCH_ADDR, buf, sizeof(uint32_t) * RISCV_BENCH_WORDS);
	target_mem64_read(target, buf, RISCV_BENCH_SCRATCH_ADDR, sizeof(uint32_t) * RISCV_BENCH_WORDS);
	const uint32_t tx_bulk = bmp_rv_tx_count_c();

	bool ok_bulk = !target_check_error(target);
	for (size_t i = 0; i < RISCV_BENCH_WORDS; ++i) {
		if (buf[i] != test_value + (uint32_t)i) ok_bulk = false;
	}

	gdb_outf("riscv_benchmark2: %s 1-word: %lu DMI, 64-word: %lu DMI %s\n",
		method_name, (unsigned long)tx_single, (unsigned long)tx_bulk,
		(ok_single && ok_bulk) ? "OK" : "FAILED");

	return ok_single && ok_bulk;
}

/* Auto-select: called from riscv32_probe(), picks the cheapest working method
 * and installs the runtime dispatcher on target->mem_read/write. */
void riscv32_benchmark_register(target_s *const target)
{
	if (!target) return;

	riscv_hart_s *const hart = riscv_hart_struct(target);
	if (!hart) return;

	/* We must be halted to perform abstract/progbuf memory accesses */
	if (!riscv_bench_ensure_halted(target)) {
		DEBUG_WARN("Failed to halt RISC-V target for memory benchmark\n");
		return;
	}

	const uint16_t probed_flags = hart->flags;
	uint32_t best_tx = RISCV_BENCH_BEST_TX_INIT;
	const riscv_bench_method_s *best_method = NULL;
	bool found_working = false;

	/* Keep the cheapest method whose bulk round trip worked */
	for (size_t i = 0; i < ARRAY_LENGTH(riscv_bench_methods); ++i) {
		const riscv_bench_method_s *const method = &riscv_bench_methods[i];
		if (!riscv_bench_method_supported(hart, method)) continue;

		const riscv_bench_result_s result = riscv_bench_method_roundtrip(target, hart, method);
		if (result.ok_bulk && result.tx_bulk < best_tx) {
			best_tx = result.tx_bulk;
			best_method = method;
			found_working = true;
		}
	}

	if (found_working) {
		DEBUG_INFO("RISC-V memory auto-select: picked method %d (tx count %lu)\n",
			best_method->id, (unsigned long)best_tx);
		riscv_bench_method_select(hart, best_method);
	} else {
		DEBUG_WARN("RISC-V memory auto-select: no methods worked, falling back to probed defaults\n");
		hart->flags = probed_flags;
	}

	/* Hook the target memory read/write pointers to our dispatcher */
	target->mem_write = riscv32_bench_mem_write;
	target->mem_read = riscv32_bench_mem_read;
}

/*
 * CH32V-specific capability probe: poke each PROGBUF register and count how
 * many slots accept data, populating hart->progbuf_size. Clears any abstract
 * errors raised by the probing. Kept as a standalone helper for manual/CH32V
 * debugging; not (yet) wired into the probe path.
 */
void riscv_progbuf_fallback_probe(riscv_hart_s *hart)
{
	/* Clear any pending errors that might block PROGBUF access */
	(void)riscv_dm_write(hart->dbg_module, RV_DM_ABST_CTRLSTATUS, RISCV_HART_OTHER << 8U);

	uint32_t read_val = 0;
	const bool w_ok = riscv_dm_write(hart->dbg_module, RV_DM_PROGBUF_BASE, RISCV_PROGBUF_PROBE_TEST_VALUE);
	const bool r_ok = riscv_dm_read(hart->dbg_module, RV_DM_PROGBUF_BASE, &read_val);

	DEBUG_WARN("CH32V Progbuf Fallback Probe: write_ok=%d, read_ok=%d, read_val=0x%08lx\n",
		w_ok, r_ok, (unsigned long)read_val);

	if (w_ok && r_ok && read_val == RISCV_PROGBUF_PROBE_TEST_VALUE) {
		hart->progbuf_size = 1U;
		for (uint32_t i = 1U; i < RISCV_PROGBUF_PROBE_MAX_SLOTS; ++i) {
			if (riscv_dm_write(hart->dbg_module, RV_DM_PROGBUF_BASE + i, RISCV_PROGBUF_PROBE_TEST_VALUE) &&
				riscv_dm_read(hart->dbg_module, RV_DM_PROGBUF_BASE + i, &read_val) &&
				read_val == RISCV_PROGBUF_PROBE_TEST_VALUE) {
				hart->progbuf_size++;
			} else {
				break;
			}
		}
		DEBUG_WARN("CH32V Progbuf Fallback Probe: discovered %u progbuf slots!\n", hart->progbuf_size);
	} else {
		DEBUG_WARN("CH32V Progbuf Fallback Probe: Failed! Hardware genuinely rejected progbuf access.\n");
	}

	/* Clear any errors generated during probing */
	(void)riscv_dm_write(hart->dbg_module, RV_DM_ABST_CTRLSTATUS, RISCV_HART_OTHER << 8U);
}
