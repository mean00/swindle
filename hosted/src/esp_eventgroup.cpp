
#include <QMutex>
#include <QWaitCondition>
#include <QDeadlineTimer>
#include <QCoreApplication>
#include <QObject>
#include <QFlags>

#include "bmp_logger.h"

#include "lnFreeRTOS_pp.h"

class qlnFastEventGroup
{
  public:
    qlnFastEventGroup()
    {
        m_bits = 0;
    }
    virtual ~qlnFastEventGroup()
    {
    }
    // Set bits in the event group
    void setBits(quint32 bits)
    {
        QMutexLocker locker(&m_mutex);
        m_bits |= bits;
        m_cond.wakeAll();
    }

    // Clear bits in the event group
    void clearBits(quint32 bits)
    {
        QMutexLocker locker(&m_mutex);
        m_bits &= ~bits;
    }

    // Wait for bits (similar to xEventGroupWaitBits)
    quint32 waitBits(quint32 bitsToWaitFor, bool waitForAll = true, bool clearOnExit = false, int timeoutMs = -1)
    {
        QDeadlineTimer deadline(timeoutMs);
        QDeadlineTimer oneMS(1);

        bool r;
        quint32 result;
        while (true)
        {
            m_mutex.lock();
            result = m_bits & bitsToWaitFor;
            if (result != 0)
            {
                m_bits &= ~result;
                m_mutex.unlock();
                return result;
            }
            r = m_cond.wait(&m_mutex, oneMS);
            m_mutex.unlock();
            QCoreApplication::processEvents();
            m_mutex.lock();
            result = m_bits & bitsToWaitFor;
            if (result != 0 || deadline.hasExpired())
            {
                m_bits &= ~result;
                m_mutex.unlock();
                return result;
            }
            m_mutex.unlock();
        }
    }

    // Get current bits
    quint32 bits() const
    {
        QMutexLocker locker(&m_mutex);
        return m_bits;
    }

  private:
    mutable QMutex m_mutex;
    QWaitCondition m_cond;
    quint32 m_bits;
};

lnFastEventGroup::lnFastEventGroup()
{
    _cookie = new qlnFastEventGroup;
}
#define Q ((qlnFastEventGroup *)_cookie)
lnFastEventGroup::~lnFastEventGroup()
{
    delete Q;
    _cookie = NULL;
}
void lnFastEventGroup::takeOwnership()
{
    //    xAssert(0);
} // the task calling this will own the FastEventGroup
void lnFastEventGroup::setEvents(uint32_t events)
{
    Q->setBits(events);
}
// -1 timeout means wait forever
uint32_t lnFastEventGroup::waitEvents(uint32_t maskint, int timeout)
{
    return Q->waitBits(maskint, false, true, timeout);
} //  the events are cleared upon return from here ! returns  0 if timeout
uint32_t lnFastEventGroup::readEvents(uint32_t maskInt)
{
    return Q->bits();
} // it is also cleared automatically !
// EOF
