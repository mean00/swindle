/**
 * @file main_qt.cpp
 * @brief Hosted-mode entry point with Qt6 GUI.
 */

#include <QApplication>
#include <QCloseEvent>
#include <QHBoxLayout>
#include <QLabel>
#include <QLoggingCategory>
#include <QPainter>
#include <QPushButton>
#include <QVBoxLayout>
#include <QWidget>
#include <csignal>
#include <cstdio>
#include <cstdlib>
#include <cstring>

#include "gdb_server_thread.h"

class StatusIndicator : public QWidget
{
    Q_OBJECT
  public:
    explicit StatusIndicator(QWidget *parent = nullptr) : QWidget(parent), m_connected(false)
    {
        setFixedSize(14, 14);
    }

    void setConnected(bool connected)
    {
        if (m_connected != connected)
        {
            m_connected = connected;
            update();
        }
    }

  protected:
    void paintEvent(QPaintEvent *) override
    {
        QPainter painter(this);
        painter.setRenderHint(QPainter::Antialiasing);
        painter.setPen(Qt::NoPen);
        painter.setBrush(m_connected ? QColor(46, 204, 113) : QColor(149, 165, 166)); // Green or Gray
        painter.drawEllipse(rect());
    }

  private:
    bool m_connected;
};

class MainWindow : public QWidget
{
    Q_OBJECT

  public:
    explicit MainWindow(GdbServerThread *serverThread, QWidget *parent = nullptr)
        : QWidget(parent), m_thread(serverThread)
    {
        setWindowTitle("Swindle Hosted");
        setMinimumWidth(280);

        auto *mainLayout = new QVBoxLayout(this);
        mainLayout->setContentsMargins(16, 16, 16, 16);
        mainLayout->setSpacing(12);

        // State row
        auto *statusLayout = new QHBoxLayout();
        m_indicator = new StatusIndicator(this);
        m_statusLabel = new QLabel("<b>Status:</b> Idle (Waiting for GDB...)", this);
        statusLayout->addWidget(m_indicator);
        statusLayout->addWidget(m_statusLabel);
        statusLayout->addStretch();
        mainLayout->addLayout(statusLayout);

        // Info label: port
        auto *infoLabel = new QLabel(QString("Port: localhost:%1").arg(DEFAULT_GDB_TCP_PORT), this);
        infoLabel->setStyleSheet("color: #7f8c8d; font-size: 11px;");
        mainLayout->addWidget(infoLabel);

        // Buttons
        auto *btnLayout = new QHBoxLayout();
        m_btnDisconnect = new QPushButton("Disconnect", this);
        m_btnDisconnect->setFocusPolicy(Qt::NoFocus);
        m_btnDisconnect->setEnabled(false);
        auto *btnQuit = new QPushButton("Quit", this);
        btnQuit->setFocusPolicy(Qt::NoFocus);

        btnLayout->addWidget(m_btnDisconnect);
        btnLayout->addWidget(btnQuit);
        mainLayout->addLayout(btnLayout);

        connect(m_btnDisconnect, &QPushButton::clicked, this, &MainWindow::onDisconnectClicked);
        connect(btnQuit, &QPushButton::clicked, this, &MainWindow::onQuitClicked);

        connect(m_thread, &GdbServerThread::connectionStateChanged, this, &MainWindow::onConnectionStateChanged);
    }

  protected:
    void closeEvent(QCloseEvent *event) override
    {
        onQuitClicked();
        event->accept();
    }

  private slots:
    void onConnectionStateChanged(bool connected)
    {
        m_indicator->setConnected(connected);
        if (connected)
        {
            m_statusLabel->setText("<b>Status:</b> Connected");
            m_btnDisconnect->setEnabled(true);
        }
        else
        {
            m_statusLabel->setText("<b>Status:</b> Idle");
            m_btnDisconnect->setEnabled(false);
        }
    }

    void onDisconnectClicked()
    {
        if (m_thread)
        {
            m_thread->disconnectClient();
        }
    }

    void onQuitClicked()
    {
        if (m_thread && m_thread->isRunning())
        {
            m_thread->stopServer();
            m_thread->wait(1000);
        }
        qApp->quit();
    }

  private:
    GdbServerThread *m_thread{nullptr};
    StatusIndicator *m_indicator{nullptr};
    QLabel *m_statusLabel{nullptr};
    QPushButton *m_btnDisconnect{nullptr};
};

/**
 * @brief Exit application cleanly on BMP shutdown.
 */
extern "C" void exit_from_bmp()
{
    QCoreApplication::exit(0);
}

#include "main_qt.moc"

int main(int argc, char **argv)
{
    for (int i = 1; i < argc; ++i)
    {
        if (strcmp(argv[i], "--decode-gdb") == 0 || strcmp(argv[i], "-d") == 0 || strcmp(argv[i], "-g") == 0)
        {
            gdb_decoder_set_enabled(true);
        }
        else if (strcmp(argv[i], "--help") == 0 || strcmp(argv[i], "-h") == 0)
        {
            printf("Usage: swindle_hosted_qt [options]\n");
            printf("Options:\n");
            printf("  --decode-gdb, -d, -g  Decode GDB commands on the fly in human-readable format\n");
            printf("  --help, -h            Show this help message\n");
            return 0;
        }
    }

    if (getenv("SWINDLE_DECODE_GDB") != nullptr)
    {
        gdb_decoder_set_enabled(true);
    }

    // Suppress benign QtWayland textinput disableSurface warning when button focus changes
    QLoggingCategory::setFilterRules("qt.qpa.wayland.textinput.warning=false");

    QApplication app(argc, argv);

    platform_timeout_init();

    auto *thread = new GdbServerThread;
    thread->start();

    MainWindow window(thread);
    window.show();

    int ret = app.exec();

    if (thread->isRunning())
    {
        thread->stopServer();
        thread->wait(1000);
    }
    delete thread;

    return ret;
}
