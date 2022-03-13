/*

 */
 #include "lnArduino.h"

//http://web.mit.edu/rhel-doc/3/rhel-gdb-en-3/general-query-packets.html
// https://www.embecosm.com/appnotes/ean4/embecosm-howto-rsp-server-ean4-issue-2.html#id3059472
// qC : current Thread  <=
// H : set thread
// Hg0
// qOffsets
// qSymbol::
// qfThreadInfo

/**

*/
extern "C" bool lnProcessCommand(int size, const char *data)
{
    Logger("Received packet %s\n",data);
    return false;
}
