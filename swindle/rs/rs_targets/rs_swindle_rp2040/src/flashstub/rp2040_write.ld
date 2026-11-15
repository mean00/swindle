/* Linker script for the RP2040 SPI flash writer stub (`rp2040_write.c`).
 *
 * Copy of blackmagic's `src/target/flashstub/rp.ld`: park the `.entry` function first so
 * `objcopy -Obinary -j.text` starts the image with it. The stub sets its own SP, so only
 * the image layout matters here.
 */
MEMORY { sram (rwx): ORIGIN = 0x20000000, LENGTH = 0x00001000 }

SECTIONS
{
	.text :
	{
		KEEP(*(.entry))
		*(.text.*, .text)
	} > sram
}
