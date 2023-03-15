// Cheap stdstring using heap

#include "lnArduino.h"
#include "bmp_string.h"
#include "stdint.h"
#include "inttypes.h"
extern "C"
{
  #include "hex_utils.h"
}


int maxWrappedString=0;
/**

*/
stringWrapper::stringWrapper()
{
    _limit=256;
    _st=(char *)malloc(_limit);
    _st[0]=0;
}
/**

*/
stringWrapper::~stringWrapper()
{
  // we dont free _st here NOT A MISTAKE!
}
/**

*/
void stringWrapper::doubleLimit()
{
  _limit*=2;
  if(_limit>maxWrappedString) maxWrappedString=_limit;
  char *st2=new char[_limit];
  strcpy(st2,_st);
  free(_st);
  _st=st2;
}
/**

*/
void stringWrapper::append(const char *a)
{
  int l=strlen(a);
  if( (strlen(_st)+l+1)>=_limit)
  {  // increase
    doubleLimit();
  }
  strcat(_st,a);
}

void stringWrapper::appendU32(uint32_t val)
{
  char st[12];
  sprintf(st,"%u",val);
  if( (strlen(_st)+strlen(st)+1)>=_limit)
  {  // increase
    doubleLimit();
  }
  strcat(_st,st);
}

/**
*/
void stringWrapper::appendHexified(const char *a)
{
  int l=strlen(a);
  l=l*2+1;
  int cur=strlen(_st);
  if(cur+l >=_limit)
      doubleLimit();
  hexify(_st+cur,a,l);
}

/**

*/
void stringWrapper::appendHex32(const uint32_t value)
{
  int l=strlen(_st)+1;
  if( (strlen(_st)+8+1)>=_limit)
  {  // increase
    doubleLimit();
  }
  char hex32[9];
  sprintf(hex32, "%08" PRIx32, value);
  strcat(_st,hex32);
}


//
