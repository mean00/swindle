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
#include <QDateTime>

#include "bmp_logger.h"

#define BMP_VID 0x1d50
#define BMP_PID 0x6030
#define LNBMP_PID 0x6030

#define TAG 0x44

#define xAssert(x) if(!(x)) {printf("%s FAILED in %s \n",#x,__PRETTY_FUNCTION__);exit(0);}

QSerialPort *qserial = NULL;
/**
*/
bool fullread(int total, uint8_t *data)
{
    while(total)
    {
        while(qserial->waitForReadyRead(10)==false)
        {

        }
        int nb=qserial->read((char *)data,total);
        if(nb<=0)
        {
           QBMPLOG("************Warning : serial error \n");
           exit(-1);
           return false;
        }
        total-=nb;
        data+=nb;
    }
    return true;                
}
extern "C" int platform_buffer_read(uint8_t *data, int maxsize)
{
    // This breaks layers separation oh well
    // The expected input is REMOTE_RESP ... REMOTE_EOM
    // and we take only the payload
    // too bad we have a proper automaton in the rust side
    {
        uint8_t c;
        if(!fullread(1,&c)) return -6;
        if(c!='&')
        {
            QBMPLOG("*************Warning : invalid resp \n");
            return -1;
        }
    }
    int nb=0;
    
    while(1)
    {
          if(!fullread(1,data+nb)) 
          {
            return -6;
          }
          
          if(data[nb]=='#') 
          {
            data[nb]=0;
            QBMPLOG("Serial Read %d bytes ",nb);
            QBMPLOGN(nb,(const char *)data);
            QBMPLOG("\n");
            
            return nb;
          }
          nb++;
          if(nb>=maxsize)
          {
            return nb;
          }
    }
    return -1;
}
/**
*/

extern "C" int platform_buffer_write(const uint8_t *data, int size)
{
    int orgsize=size;
    while(size)
    {
        int nb=qserial->write((const char *)data,size);
        if(nb<=0)
        {
            QBMPLOG("***** WRITE FAILURE\n");
            exit(-1);
            break;
        }
        QBMPLOG("Write %d bytes",nb);
        QBMPLOGN(nb,(const char *)data);
        QBMPLOG("\n");
        size-=nb;
        data+=nb;        
    }    
    QBMPLOG("-- flush in \n");
    qserial->flush();
    QBMPLOG("-- flush end \n");
    QBMPLOG("Write %d bytes\n",orgsize);
    return size;
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
                if(pid==BMP_PID)// || pid==LNBMP_PID)
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
    QBMPLOG("Probing : %s\n", info->manufacturer);    
    
    QString port(info->manufacturer);
    qserial= new QSerialPort(port);    
}
/**
*/

extern "C" int serial_open(const bmda_cli_options_s *opt, const char *serial)
{    
    QBMPLOG("Serial open\n");
    if(!qserial->open(QIODevice::ReadWrite))
    {
        QBMPLOG("cannot open\n");
        return -1;
    }
    qserial->setFlowControl(QSerialPort::NoFlowControl);
    qserial->setBaudRate(QSerialPort::Baud115200);
    qserial->setDataBits(QSerialPort::Data8);
    qserial->setReadBufferSize(1);
    //qserial->setFlowControl(QSerialPort::NoFlowControl);
    return 0;
}

// EOF