
#pragma once
#include <QObject>
#include <QTcpSocket>
#include <QTcpServer>
#include <QDebug>
//
//
//
class BmpTcpServer : public QObject
{
    Q_OBJECT
public:
    explicit BmpTcpServer(QObject *parent = 0);
    
signals:
    
public slots:
    void newConnection();
    
private:
    QTcpServer *_server;
};
//
//
//
#define QBUFFER_SIZE 512
class BMPTcp : public QObject
{
    Q_OBJECT
public :
    explicit BMPTcp(QTcpSocket *sock);
public slots:
    void disconnected();
    void readyRead();
private:
    QTcpSocket *_socket;   
    uint8_t     _buffer[QBUFFER_SIZE];
};
//