MEMORY
{
  /* Leave 16k for the default bootloader on the Feather M4 */
  FLASH (rx) : ORIGIN = 0x00000000 + 16K, LENGTH = 512K - 16K
  RAM (xrw)  : ORIGIN = 0x20000000, LENGTH = 190K
  PDUMP (rw) : ORIGIN = 0x20000000 + LENGTH(RAM), LENGTH = 2K
}
_stack_start = ORIGIN(RAM) + LENGTH(RAM);
_panic_dump_start = ORIGIN(PDUMP);
_panic_dump_end   = ORIGIN(PDUMP) + LENGTH(PDUMP);
