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

extern "C" void gdb_putpacket(const char *packet, int size);

extern "C" void execqOffsets(const char *packet, int len)
{
  gdb_putpacket("Text=0;Data=0;Bss=0", 19); // 7 7 5=>19
}
//
extern "C" void execqSymbol(const char *packet, int len)
{
  Logger("*** execqSymbol:%s\n",packet);
  gdb_putpacket("OK", 2);
}
//
extern "C" void execqThreadInfo(const char *packet, int len)
{
  Logger("*** execqThreadInfo:%s\n",packet);
  gdb_putpacket("", 0);
}
//--
extern "C" void execqfThreadInfo(const char *packet, int len)
{
  Logger("*** qfThreadinfo:%s\n",packet);
  gdb_putpacket("m", 1);
}


/**

*/
extern "C" bool lnProcessCommand(int size, const char *data)
{
    Logger("Received packet %s\n",data);
    return false;
}
