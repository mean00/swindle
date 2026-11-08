
rp2040_write.elf:     file format elf32-littlearm


Disassembly of section .text:

20000000 <rp_flash_write_stub>:
	(void)dest;
	(void)src;
	(void)length;
	(void)page_size;
	/* Create a stack for our own sanity */
	__asm__("ldr r4, =#0x20042000\n"
20000000:	4c2f      	ldr	r4, [pc, #188]	@ (200000c0 <rp_flash_write+0xb0>)
20000002:	46a5      	mov	sp, r4
20000004:	f000 f804 	bl	20000010 <rp_flash_write>
20000008:	2800      	cmp	r0, #0
2000000a:	d000      	beq.n	2000000e <rp_flash_write_stub+0xe>
2000000c:	be01      	bkpt	0x0001
2000000e:	be00      	bkpt	0x0000

20000010 <rp_flash_write>:
	rp_spi_flash_deselect();
}

static bool __attribute__((used, section(".entry"))) rp_flash_write(
	const uint32_t dest, const uint8_t *const src, const size_t length, const uint32_t page_size)
{
20000010:	b5f0      	push	{r4, r5, r6, r7, lr}
20000012:	001f      	movs	r7, r3
	for (size_t offset = 0; offset < length; offset += page_size) {
20000014:	2500      	movs	r5, #0
{
20000016:	b085      	sub	sp, #20
20000018:	9002      	str	r0, [sp, #8]
2000001a:	9103      	str	r1, [sp, #12]
2000001c:	9201      	str	r2, [sp, #4]
	for (size_t offset = 0; offset < length; offset += page_size) {
2000001e:	9b01      	ldr	r3, [sp, #4]
20000020:	429d      	cmp	r5, r3
20000022:	d302      	bcc.n	2000002a <rp_flash_write+0x1a>
		const size_t amount = MIN(length - offset, page_size);
		rp_spi_write(dest + offset, src + offset, amount);
		while (rp_spi_read_status() & SPI_FLASH_STATUS_BUSY)
			continue;
	}
	return true;
20000024:	2001      	movs	r0, #1
}
20000026:	b005      	add	sp, #20
20000028:	bdf0      	pop	{r4, r5, r6, r7, pc}
	rp_spi_flash_select();
2000002a:	f000 f84b 	bl	200000c4 <rp_spi_flash_select>
	rp_spi_xfer_data(SPI_FLASH_CMD_WRITE_ENABLE);
2000002e:	2006      	movs	r0, #6
20000030:	f000 f860 	bl	200000f4 <rp_spi_xfer_data>
	rp_spi_flash_deselect();
20000034:	f000 f854 	bl	200000e0 <rp_spi_flash_deselect>
	rp_spi_flash_select();
20000038:	f000 f844 	bl	200000c4 <rp_spi_flash_select>
	rp_spi_xfer_data(SPI_FLASH_CMD_READ_STATUS);
2000003c:	2005      	movs	r0, #5
2000003e:	f000 f859 	bl	200000f4 <rp_spi_xfer_data>
	const uint8_t status = rp_spi_xfer_data(0U);
20000042:	2000      	movs	r0, #0
20000044:	f000 f856 	bl	200000f4 <rp_spi_xfer_data>
20000048:	0004      	movs	r4, r0
	rp_spi_flash_deselect();
2000004a:	f000 f849 	bl	200000e0 <rp_spi_flash_deselect>
		if (!(rp_spi_read_status() & SPI_FLASH_STATUS_WRITE_ENABLED))
2000004e:	2002      	movs	r0, #2
20000050:	2302      	movs	r3, #2
20000052:	4020      	ands	r0, r4
20000054:	421c      	tst	r4, r3
20000056:	d0e6      	beq.n	20000026 <rp_flash_write+0x16>
		const size_t amount = MIN(length - offset, page_size);
20000058:	9b01      	ldr	r3, [sp, #4]
2000005a:	1b5e      	subs	r6, r3, r5
2000005c:	42be      	cmp	r6, r7
2000005e:	d900      	bls.n	20000062 <rp_flash_write+0x52>
20000060:	003e      	movs	r6, r7
		rp_spi_write(dest + offset, src + offset, amount);
20000062:	9b02      	ldr	r3, [sp, #8]
20000064:	18ec      	adds	r4, r5, r3
	rp_spi_flash_select();
20000066:	f000 f82d 	bl	200000c4 <rp_spi_flash_select>
	rp_spi_xfer_data(SPI_FLASH_CMD_PAGE_PROGRAM);
2000006a:	2002      	movs	r0, #2
2000006c:	f000 f842 	bl	200000f4 <rp_spi_xfer_data>
	rp_spi_xfer_data((address >> 16U) & 0xffU);
20000070:	0c20      	lsrs	r0, r4, #16
20000072:	b2c0      	uxtb	r0, r0
20000074:	f000 f83e 	bl	200000f4 <rp_spi_xfer_data>
	rp_spi_xfer_data((address >> 8U) & 0xffU);
20000078:	0a20      	lsrs	r0, r4, #8
2000007a:	b2c0      	uxtb	r0, r0
2000007c:	f000 f83a 	bl	200000f4 <rp_spi_xfer_data>
	rp_spi_xfer_data(address & 0xffU);
20000080:	b2e0      	uxtb	r0, r4
20000082:	f000 f837 	bl	200000f4 <rp_spi_xfer_data>
	for (size_t i = 0; i < length; ++i)
20000086:	9b03      	ldr	r3, [sp, #12]
20000088:	195c      	adds	r4, r3, r5
2000008a:	19a6      	adds	r6, r4, r6
2000008c:	42b4      	cmp	r4, r6
2000008e:	d111      	bne.n	200000b4 <rp_flash_write+0xa4>
	rp_spi_flash_deselect();
20000090:	f000 f826 	bl	200000e0 <rp_spi_flash_deselect>
	rp_spi_flash_select();
20000094:	f000 f816 	bl	200000c4 <rp_spi_flash_select>
	rp_spi_xfer_data(SPI_FLASH_CMD_READ_STATUS);
20000098:	2005      	movs	r0, #5
2000009a:	f000 f82b 	bl	200000f4 <rp_spi_xfer_data>
	const uint8_t status = rp_spi_xfer_data(0U);
2000009e:	2000      	movs	r0, #0
200000a0:	f000 f828 	bl	200000f4 <rp_spi_xfer_data>
200000a4:	0004      	movs	r4, r0
	rp_spi_flash_deselect();
200000a6:	f000 f81b 	bl	200000e0 <rp_spi_flash_deselect>
		while (rp_spi_read_status() & SPI_FLASH_STATUS_BUSY)
200000aa:	2301      	movs	r3, #1
200000ac:	421c      	tst	r4, r3
200000ae:	d1f1      	bne.n	20000094 <rp_flash_write+0x84>
	for (size_t offset = 0; offset < length; offset += page_size) {
200000b0:	19ed      	adds	r5, r5, r7
200000b2:	e7b4      	b.n	2000001e <rp_flash_write+0xe>
		rp_spi_xfer_data(src[i]);
200000b4:	7820      	ldrb	r0, [r4, #0]
200000b6:	f000 f81d 	bl	200000f4 <rp_spi_xfer_data>
	for (size_t i = 0; i < length; ++i)
200000ba:	3401      	adds	r4, #1
200000bc:	e7e6      	b.n	2000008c <rp_flash_write+0x7c>
200000be:	0000      	.short	0x0000
200000c0:	20042000 	.word	0x20042000

200000c4 <rp_spi_flash_select>:
	SPI_CHIP_SELECT(RP_GPIO_QSPI_CS_DRIVE_LOW);
200000c4:	4904      	ldr	r1, [pc, #16]	@ (200000d8 <rp_spi_flash_select+0x14>)
200000c6:	4b05      	ldr	r3, [pc, #20]	@ (200000dc <rp_spi_flash_select+0x18>)
200000c8:	68ca      	ldr	r2, [r1, #12]
200000ca:	401a      	ands	r2, r3
200000cc:	2380      	movs	r3, #128	@ 0x80
200000ce:	009b      	lsls	r3, r3, #2
200000d0:	4313      	orrs	r3, r2
200000d2:	60cb      	str	r3, [r1, #12]
}
200000d4:	4770      	bx	lr
200000d6:	46c0      	nop			@ (mov r8, r8)
200000d8:	40018000 	.word	0x40018000
200000dc:	fffffcff 	.word	0xfffffcff

200000e0 <rp_spi_flash_deselect>:
	SPI_CHIP_SELECT(RP_GPIO_QSPI_CS_DRIVE_HIGH);
200000e0:	23c0      	movs	r3, #192	@ 0xc0
200000e2:	4a03      	ldr	r2, [pc, #12]	@ (200000f0 <rp_spi_flash_deselect+0x10>)
200000e4:	009b      	lsls	r3, r3, #2
200000e6:	68d1      	ldr	r1, [r2, #12]
200000e8:	430b      	orrs	r3, r1
200000ea:	60d3      	str	r3, [r2, #12]
}
200000ec:	4770      	bx	lr
200000ee:	46c0      	nop			@ (mov r8, r8)
200000f0:	40018000 	.word	0x40018000

200000f4 <rp_spi_xfer_data>:
	ssi->data = data;
200000f4:	23c0      	movs	r3, #192	@ 0xc0
	while (!(ssi->status & RP_SSI_STATUS_RX_FIFO_NOT_EMPTY))
200000f6:	2208      	movs	r2, #8
	ssi->data = data;
200000f8:	055b      	lsls	r3, r3, #21
200000fa:	6618      	str	r0, [r3, #96]	@ 0x60
	while (!(ssi->status & RP_SSI_STATUS_RX_FIFO_NOT_EMPTY))
200000fc:	6a99      	ldr	r1, [r3, #40]	@ 0x28
200000fe:	4211      	tst	r1, r2
20000100:	d0fc      	beq.n	200000fc <rp_spi_xfer_data+0x8>
	return ssi->data & 0xffU;
20000102:	6e18      	ldr	r0, [r3, #96]	@ 0x60
20000104:	b2c0      	uxtb	r0, r0
}
20000106:	4770      	bx	lr
