#include "stdio.h"
#include "stdint.h"
#include "stdarg.h"
#include "stdarg.h"
#include "string.h"

#include <unistd.h>
#include <time.h>
extern "C"
{
    #include "../hosted/cli.h"
    #include "../hosted/bmp_hosted.h"
}

#include <QSerialPort>
#include <QSerialPortInfo>
#include <QDebug>

#define BMP_VID 0x1d50
#define BMP_PID 1
#define LNBMP_PID 0x6030

#define TAG 0x44

#define xAssert(x) if(!(x)) {printf("%s FAILED in %s \n",#x,__PRETTY_FUNCTION__);exit(0);}

QSerialPort *qserial = NULL;
/**
*/

extern "C" int platform_buffer_read(uint8_t *data, int maxsize)
{
    qserial->waitForReadyRead(-1);
    int nb=qserial->read((char *)data,maxsize);
    qWarning() << "Read " << nb << "\n";
    return nb;
}
/**
*/

extern "C" int platform_buffer_write(const uint8_t *data, int size)
{
    int nb=qserial->write((const char *)data,size);
    if(nb)
    {
        qserial->flush();
    }
    qWarning() << "Write " << nb << "\n";
    return nb;
}
/**
*/

extern "C"  int find_debuggers(bmda_cli_options_s *cl_opts, bmp_info_s *info)
{
     const auto serialPortInfos = QSerialPortInfo::availablePorts();
    for (const QSerialPortInfo &portInfo : serialPortInfos) 
    {
        if(portInfo.hasVendorIdentifier() && portInfo.hasProductIdentifier())
        {
            quint16 pid= portInfo.productIdentifier();
            if(portInfo.vendorIdentifier() == BMP_VID)
            {
                // take the 1st one (?)
                if(pid==BMP_PID || pid==LNBMP_PID)
                {
                    // take the 1st one (?)
                    memset(info,0,sizeof(*info));
                    info->bmp_type=BMP_TYPE_BMP;
                    // this is ugly, dont check anything just copy hoping it fits    
                    strcpy(info->manufacturer,portInfo.systemLocation().toLatin1().constData());
                }                
                return 0;
            }            
        }
        
    }
    xAssert(0);    
    return 0;
}
/**
*/

extern "C" void libusb_exit_function(bmp_info_s *info)
{
    qWarning("Exiting libusb..\n");
    if(qserial)
    {
        qserial->close();
        delete qserial;
        qserial = NULL;
    }
}
/**
*/
extern "C" void bmp_ident(bmp_info_s *info)
{
    qWarning() << "Probing" << info->manufacturer << "\n";    
    QString port(info->manufacturer);
    qWarning() << "Port:" << port << "\n";
    qserial= new QSerialPort(port);    
}
/**
*/

extern "C" int serial_open(const bmda_cli_options_s *opt, const char *serial)
{    
   qWarning() << "Serial open\n";
     if(!qserial->open(QIODevice::ReadWrite))
    {
        qWarning() << "cannot open";
        return -1;
    }
   return 0;
}
//
//  Stubs for RPC mode
//
//
extern "C" bool bmp_rpc_init_swd_c()
{
    xAssert(0);
    return true;
}
extern "C"  bool bmp_rpc_swd_in_c(uint32_t *value, uint32_t nb_bits)
{
   xAssert(0);
   return true;
}
/*
*/
extern "C"  bool bmp_rpc_swd_in_par_c(uint32_t *value, bool *par, uint32_t nb_bits)
{
   xAssert(0);
   return true;
}
/*
*/
extern "C" bool bmp_rpc_swd_out_c(const uint32_t value, uint32_t nb_bits)
{
   xAssert(0);
   return true;
}
/*
*/
extern "C" bool bmp_rpc_swd_out_par_c(const uint32_t value, uint32_t nb_bits)
{
   xAssert(0);
   return true;
}
// EOF