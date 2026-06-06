/**
 * @file esp_eventgroup.cpp
 * @brief Qt-based implementation of lnFastEventGroup using QMutex and QWaitCondition.
 *
 * Provides an event-group synchronisation primitive for the hosted build,
 * analogous to FreeRTOS xEventGroup on embedded targets.
 */

#include <QMutex>
#include <QWaitCondition>
#include <QDeadlineTimer>
#include <QCoreApplication>
#include <QObject>
#include <QFlags>

#include "bmp_logger.h"

#include "lnFreeRTOS_pp.h"

/**
 * @brief Internal Qt-based event group implementation.
 */
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

    /**
     * @brief Set bits in the event group and wake up any waiters.
     * @param bits  Bitmask of bits to set.
     */
    void setBits(quint32 bits)
    {
        QMutexLocker locker(&m_mutex);
        m_bits |= bits;
        m_cond.wakeAll();
    }

    /**
     * @brief Clear bits in the event group.
     * @param bits  Bitmask of bits to clear.
     */
    void clearBits(quint32 bits)
    {
        QMutexLocker locker(&m_mutex);
        m_bits &= ~bits;
    }

    /**
     * @brief Wait for specified bits to become set.
     *
     * Processes Qt events during the wait to allow cross-thread signal delivery.
     * @param bitsToWaitFor  Bitmask of bits to wait for.
     * @param waitForAll     If true, all bits must be set; if false, any bit suffices.
     * @param clearOnExit    If true, clear the matched bits before returning.
     * @param timeoutMs      Timeout in milliseconds (-1 = wait forever).
     * @return The bits that were set (may be 0 on timeout).
     */
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
            // Process Qt events so that cross-thread signal delivery
            // (e.g. QTcpServer newConnection from the main thread)
            // reaches objects living on this thread.
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

    /**
     * @brief Get the current event bits.
     * @return Current bitmask value.
     */
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

/**
 * @brief Construct a new lnFastEventGroup wrapping a qlnFastEventGroup.
 */
lnFastEventGroup::lnFastEventGroup()
{
    _cookie = new qlnFastEventGroup;
}
#define Q ((qlnFastEventGroup *)_cookie)

/**
 * @brief Destroy the event group and free the internal implementation.
 */
lnFastEventGroup::~lnFastEventGroup()
{
    delete Q;
    _cookie = NULL;
}

/**
 * @brief Take ownership of the event group (stub for hosted build).
 */
void lnFastEventGroup::takeOwnership()
{
    //    xAssert(0);
} // the task calling this will own the FastEventGroup

/**
 * @brief Set events in the group.
 * @param events  Bitmask of events to set.
 */
void lnFastEventGroup::setEvents(uint32_t events)
{
    Q->setBits(events);
}

/**
 * @brief Wait for events with a timeout.
 *
 * Events are cleared upon return.
 * @param maskint  Bitmask of events to wait for.
 * @param timeout  Timeout in milliseconds (-1 = wait forever).
 * @return The bits that were set, or 0 on timeout.
 */
uint32_t lnFastEventGroup::waitEvents(uint32_t maskint, int timeout)
{
    return Q->waitBits(maskint, false, true, timeout);
} //  the events are cleared upon return from here ! returns  0 if timeout

/**
 * @brief Read the current event bits without waiting.
 * @param maskInt  Bitmask to filter (unused, returns all bits).
 * @return Current event bits.
 */
uint32_t lnFastEventGroup::readEvents(uint32_t maskInt)
{
    return Q->bits();
} // it is also cleared automatically !
// EOF