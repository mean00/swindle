
#include "lnBMPArduino.h"
extern "C"
{
#include "rtt_if.h"
}

int rtt_if_init(void)
{
    return 0;
}
/* hosted teardown */
int rtt_if_exit(void)
{
    return 0;
}

/* target to host: write len bytes from the buffer on the channel starting at buf. return number bytes written */
extern "C" uint32_t rtt_write(const uint32_t channel, const char *buf, uint32_t len)
{
    return len;
}
/* host to target: read one character from the channel, non-blocking. return character, -1 if no character */
extern "C" int32_t rtt_getchar(const uint32_t channel)
{
    return -1;
}
/* host to target: true if no characters available for reading in the selected channel */
extern "C" bool rtt_nodata(const uint32_t channel)
{
    return false;
}

// EOF
