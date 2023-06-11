/*

 */
#include "bmp_string.h"
#include "bmp_util.h"
#include "lnArduino.h"
#include "lnBmpTask.h"
#include "lnStopWatch.h"

static void map_ram(stringWrapper &wrapper, struct target_ram *ram)
{
    wrapper.append("<memory type=\"ram\" start=\"0x");
    wrapper.appendHex32(ram->start);
    wrapper.append("\" length=\"0x");
    wrapper.appendHex32((uint32_t)ram->length);
    wrapper.append("\"/>");
}

static void map_flash(stringWrapper &wrapper, struct target_flash *f)
{
    wrapper.append("<memory type=\"flash\" start=\"0x");
    wrapper.appendHex32(f->start);
    wrapper.append("\" length=\"0x");
    wrapper.appendHex32((uint32_t)f->length);
    wrapper.append("\">");

    wrapper.append("<property name=\"blocksize\">0x");
    wrapper.appendHex32((uint32_t)f->blocksize);
    wrapper.append("</property></memory>");
}

extern "C" char *ztarget_mem_map(const target *t)
{
    stringWrapper wrapper;
    wrapper.append("<memory-map>");

    /* Map each defined RAM */
    for (struct target_ram *r = t->ram; r; r = r->next)
        map_ram(wrapper, r);
    /* Map each defined Flash */
    for (struct target_flash *f = t->flash; f; f = f->next)
        map_flash(wrapper, f);
    wrapper.append("</memory-map>");
    char *out = wrapper.string();
    return out;
}

// check the given address is in flash or ram
extern "C" bool target_validate_address_flash_or_ram(target *t, uint32_t address)
{
    struct target_ram *r = t->ram;
    while (r)
    {
        if (address >= r->start && address <= (r->start + r->length))
            return true;
        r = r->next;
    }
    struct target_flash *p = t->flash;
    while (p)
    {
        if (address >= p->start && address <= (p->start + p->length))
            return true;
        p = p->next;
    }
    return false;
}

/**

*/

uint32_t readMem32(uint32_t base, uint32_t offset)
{
    if (!target_validate_address_flash_or_ram(cur_target, base))
    {
        Logger("Invalid ram read %x+%x\n", base, offset);
        return 0;
    }
    return target_mem_read32(cur_target, base + offset);
}
void writeMem32(uint32_t base, uint32_t offset, uint32_t value)
{
    if (!target_validate_address_flash_or_ram(cur_target, base))
    {
        Logger("Invalid ram read %x+%x\n", base, offset);
        return;
    }
    target_mem_write32(cur_target, base + offset, value);
}
// EOF
