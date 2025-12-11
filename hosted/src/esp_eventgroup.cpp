#include "bmp_logger.h"
#include <QCoreApplication>
#include <QDateTime>
#include <QObject>
#include <QTimer>
#include <QtGlobal>

#include "lnFreeRTOS_pp.h"

/*--*/

lnFastEventGroup::lnFastEventGroup()
{
    xAssert(0);
}
lnFastEventGroup::~lnFastEventGroup()
{
    xAssert(0);
}
void lnFastEventGroup::takeOwnership()
{
    xAssert(0);
} // the task calling this will own the FastEventGroup
void lnFastEventGroup::setEvents(uint32_t events)
{
    xAssert(0);
}
// -1 timeout means wait forever
uint32_t lnFastEventGroup::waitEvents(uint32_t maskint, int timeout)
{
    xAssert(0);
    return 0;
} //  the events are cleared upon return from here ! returns  0 if timeout
uint32_t lnFastEventGroup::readEvents(uint32_t maskInt)
{
    xAssert(0);
    return 0;
} // it is also cleared automatically !
#if 0
    volatile uint32_t _value;
    volatile uint32_t _mask;
    TaskHandle_t _waitingTask;
#endif
