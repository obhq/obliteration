#ifndef DEBUGGER_H
#define DEBUGGER_H

#include <QProcess>

#include "symbol_resolver.h"

class QThread;
class SymbolResolver;

class Debugger: public QObject
{
    Q_OBJECT
public:
    Debugger(QProcess* kernel, SymbolResolver* resolver);
    ~Debugger();

    void detach();

signals:
    void kernelCrash();

private:
    void printBacktrace(int pid, int tid);
    QProcess *m_kernel;
    QThread *t;
    SymbolResolver* m_symbol_resolver;
};

#endif // DEBUGGER_H
