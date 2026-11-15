#ifndef RP2350_QMI_H
#define RP2350_QMI_H

#include <stdint.h>
#include <stddef.h>

/// Hardware layout of the QMI controller registers used in direct mode.
typedef struct {
    volatile uint32_t direct_csr; // 0x00
    volatile uint32_t direct_tx;  // 0x04
    volatile uint32_t direct_rx;  // 0x08
} qmi_hw_t;

#define QMI_BASE        0x400d0000U
#define QMI             ((qmi_hw_t *)QMI_BASE)
#define QMI_SET         ((qmi_hw_t *)(QMI_BASE + 0x2000U))
#define QMI_CLR         ((qmi_hw_t *)(QMI_BASE + 0x3000U))

#define QMI_REG_OFFSET(member) offsetof(qmi_hw_t, member)

// DIRECT_CSR bits
#define QMI_DIRECT_CSR_BUSY         (1U << 1)
#define QMI_DIRECT_CSR_ASSERT_CS0N  (1U << 2)
#define QMI_DIRECT_CSR_TXFULL       (1U << 10)
#define QMI_DIRECT_CSR_TXEMPTY      (1U << 11)
#define QMI_DIRECT_CSR_RXEMPTY      (1U << 16)

// DIRECT_TX bits
#define QMI_DIRECT_TX_NOPUSH_RX     (1U << 20)
#define QMI_DIRECT_TX_DATA_16BIT    (1U << 18)

// SPI Flash Commands
#define SPI_CMD_WRITE_ENABLE        0x06U
#define SPI_CMD_PAGE_PROGRAM        0x02U
#define SPI_CMD_READ_STATUS         0x05U
#define SPI_STATUS_BUSY             (1U << 0)

static inline void qmi_cs_assert(void)
{
    QMI_SET->direct_csr = QMI_DIRECT_CSR_ASSERT_CS0N;
}

static inline void qmi_cs_deassert(void)
{
    QMI_CLR->direct_csr = QMI_DIRECT_CSR_ASSERT_CS0N;
}

static void qmi_wait_not_busy(void)
{
    while ((QMI->direct_csr & QMI_DIRECT_CSR_TXEMPTY) == 0)
        continue;
    while (QMI->direct_csr & QMI_DIRECT_CSR_BUSY)
        continue;
}

static void qmi_tx(uint32_t val)
{
    while (QMI->direct_csr & QMI_DIRECT_CSR_TXFULL)
        continue;
    QMI->direct_tx = val;
}

static inline void qmi_tx8(uint8_t byte)
{
    qmi_tx(QMI_DIRECT_TX_NOPUSH_RX | byte);
}

static inline void qmi_tx16(uint16_t word)
{
    qmi_tx(QMI_DIRECT_TX_NOPUSH_RX | QMI_DIRECT_TX_DATA_16BIT | word);
}

static void qmi_cmd(uint8_t cmd)
{
    qmi_cs_assert();
    qmi_tx8(cmd);
    qmi_wait_not_busy();
    qmi_cs_deassert();
}

static void qmi_wait_ready(void)
{
    for (;;)
    {
        while ((QMI->direct_csr & QMI_DIRECT_CSR_RXEMPTY) == 0)
            (void)QMI->direct_rx;

        qmi_cs_assert();
        qmi_tx8(SPI_CMD_READ_STATUS);
        qmi_tx(0);
        qmi_wait_not_busy();
        qmi_cs_deassert();

        while (QMI->direct_csr & QMI_DIRECT_CSR_RXEMPTY)
            continue;
        uint8_t status = (uint8_t)QMI->direct_rx;
        if ((status & SPI_STATUS_BUSY) == 0)
            break;
    }
}

#endif
