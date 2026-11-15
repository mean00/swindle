MEMORY
{
  ram (rwx) : ORIGIN = 0x20001000, LENGTH = 4K
}

SECTIONS
{
  .entry :
  {
    KEEP(*(.entry))
  } > ram

  .text :
  {
    *(.text*)
    *(.rodata*)
  } > ram

  /DISCARD/ :
  {
    *(.data*)
    *(.bss*)
    *(.comment)
    *(.note*)
  }
}
