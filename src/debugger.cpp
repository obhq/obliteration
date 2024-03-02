#include "debugger.h"
#include "symbol_resolver.h"

#ifdef __linux__
#include <QDebug>
#include <QFile>
#include <QRegularExpression>
#include <QThread>

#include <libunwind.h>
#include <libunwind-ptrace.h>
#include <string.h>
#include <sys/ptrace.h>
#include <sys/user.h>
#include <sys/wait.h>
#endif

Debugger::Debugger(QProcess* kernel, SymbolResolver* resolver): m_kernel(kernel), t(nullptr), m_symbol_resolver(resolver)
{
    qDebug() << "starting debugger";
#ifdef __linux__

    t = QThread::create([this](){
        int pid = this->m_kernel->processId();
        auto seize = ptrace(PTRACE_SEIZE, pid, 0, PTRACE_O_TRACEEXIT | PTRACE_O_TRACECLONE | PTRACE_O_TRACEVFORKDONE | PTRACE_O_EXITKILL);
        if (seize == -1)
        {
            qDebug() << "PTRACE_SEIZE failed with " << strerror(errno);
            return;
        }

        int status, ret;

        while (ret = waitpid(-1, &status, __WALL), ret != -1)
        {
            if (WIFSTOPPED(status))
            {
                qDebug() << "PID" << ret << "stopped with signal" << WSTOPSIG(status);
                if (status>>8 == (SIGTRAP | (PTRACE_EVENT_EXIT<<8)))
                {
                    // kernel crashed, now we can probe why
                    unsigned long msg;
                    if (-1 == ptrace(PTRACE_GETEVENTMSG, ret, 0, &msg))
                    {
                        qDebug() << "PTRACE_GETEVENTMSG failed" << strerror(errno);
                    }
                    else
                    {
                        qDebug() << "kernel exit status" << msg;
                    }

                    emit this->kernelCrash();
                    return;
                }
                else if (status>>8 == (SIGTRAP | (PTRACE_EVENT_CLONE<<8)))
                {
                    // new thread created by kernel
                    unsigned long msg;
                    if (-1 == ptrace(PTRACE_GETEVENTMSG, ret, 0, &msg))
                    {
                        qDebug() << "PTRACE_GETEVENTMSG failed" << strerror(errno);
                    }
                    else
                    {
                        qDebug() << "new thread id" << msg;
                    }
                    if (-1 == ptrace(PTRACE_CONT, ret, 0, 0))
                    {
                        qDebug() << "PTRACE_CONT failed" << ret << strerror(errno);
                    }
                    continue;
                }
                else if (status>>8 == (SIGTRAP | (PTRACE_EVENT_VFORK_DONE<<8)))
                {
                    // kernel vfork
                    unsigned long msg;
                    if (-1 == ptrace(PTRACE_GETEVENTMSG, ret, 0, &msg))
                    {
                        qDebug() << "PTRACE_GETEVENTMSG failed" << strerror(errno);
                    }
                    else
                    {
                        qDebug() << "kernel vfork" << msg;
                    }
                    if (-1 == ptrace(PTRACE_CONT, ret, 0, 0))
                    {
                        qDebug() << "PTRACE_CONT failed" << strerror(errno);
                    }
                    continue;
                }
                else if (WSTOPSIG(status) == SIGSEGV)
                {
                    this->printBacktrace(pid, ret);
                    emit this->kernelCrash();
                    for (;;)
                    {
                        if (QThread::currentThread()->isInterruptionRequested())
                        {
                            qDebug() << "detaching from kernel";
                            if (-1 == ptrace(PTRACE_DETACH, pid, 0, 0))
                                qDebug() << "PTRACE_DETACH failed with" << strerror(errno);
                            return;
                        }
                    }
                }
                if (-1 == ptrace(PTRACE_CONT, ret, 0, WSTOPSIG(status)))
                {
                    qDebug() << "PTRACE_CONT failed" << strerror(errno);
                }
            }
        }
        qDebug() << "waitpid failed with" << strerror(errno);
        qDebug() << "thread exited";
    });

    t->start();

#endif
}

struct memory_mapping
{
    unsigned long start_addr;
    unsigned long end_addr;
    char permissions;
    std::string name_or_file;
};

struct process_memory_map
{
    std::vector<memory_mapping> maps;

    std::optional<std::pair<std::string, uint64_t>> find_area(uint64_t addr);
};

std::optional<std::pair<std::string, uint64_t>> process_memory_map::find_area(uint64_t addr)
{
    for (auto &&map : this->maps)
    {
        if (addr >= map.start_addr && addr <= map.end_addr)
        {
            auto offset = addr - map.start_addr;

            return {{map.name_or_file, offset}};
        }
    }

    return {};
}

std::optional<process_memory_map> read_memory_map(int pid);

void Debugger::printBacktrace(int pid, int tid)
{
#ifdef __linux__
    auto upt = _UPT_create(tid);
    auto addr_space = unw_create_addr_space(&_UPT_accessors, 0);

    auto memory_map = read_memory_map(pid);

    unw_cursor_t c;
    int ret = unw_init_remote(&c, addr_space, upt);
    if (ret != 0)
    {
        qDebug() << "unw_init_remote failed with" << ret;
        return;
    }

    user_regs_struct regs;
    if (-1 == ptrace(PTRACE_GETREGS, tid, 0, &regs))
    {
        qDebug() << "couldn't read registers from tid" << tid << "(" << strerror(errno) << ")";
    }
    else
    {
        qDebug() << "Thread" << tid << "registers:";
        qDebug() << Qt::hex << Qt::showbase << "rax:" << regs.rax << "    " << "r8: "    << regs.r8;
        qDebug() << Qt::hex << Qt::showbase << "rbx:" << regs.rbx << "    " << "r9: "    << regs.r9;
        qDebug() << Qt::hex << Qt::showbase << "rcx:" << regs.rcx << "    " << "r10:"    << regs.r10;
        qDebug() << Qt::hex << Qt::showbase << "rdx:" << regs.rdx << "    " << "r11:"    << regs.r11;
        qDebug() << Qt::hex << Qt::showbase << "rsi:" << regs.rsi << "    " << "r12:"    << regs.r12;
        qDebug() << Qt::hex << Qt::showbase << "rdi:" << regs.rdi << "    " << "r13:"    << regs.r13;
        qDebug() << Qt::hex << Qt::showbase << "rbp:" << regs.rbp << "    " << "r14:"    << regs.r14;
        qDebug() << Qt::hex << Qt::showbase << "rsp:" << regs.rsp << "    " << "r15:"    << regs.r15;
        qDebug() << Qt::hex << Qt::showbase << "fs: " << regs.fs  << "    " << "rip:"    << regs.rip;
        qDebug() << Qt::hex << Qt::showbase << "gs: " << regs.gs  << "    " << "eflags:" << regs.eflags;
    }

    do {
        unw_word_t  offset, pc;
        char        fname[64];

        unw_get_reg(&c, UNW_REG_IP, &pc);

        auto f = memory_map->find_area(pc);

        fname[0] = '\0';
        (void) unw_get_proc_name(&c, fname, sizeof(fname), &offset);
        auto demangled = fname[0] ? SymbolResolver::demangle(std::string(fname)) : "";

        if (f)
        {
            auto ps4symbol = m_symbol_resolver->resolve(QString::fromStdString((*f).first), (*f).second).value_or(std::make_pair(QString(), (*f).second));
            printf("\n%p : (%s+0x%x) [%p] [%s+0x%lx]", (void *)pc,
                   fname[0] ? demangled.c_str() : ps4symbol.first.toUtf8().data(),
                   fname[0] ? (int) offset : ps4symbol.second,
                   (void *) pc,
                   (*f).first.c_str(),
                   (*f).second);
        }
        else
        {
            printf("\n%p : (%s+0x%x) [%p]", (void *)pc,
                   fname,
                   (int) offset,
                   (void *) pc);
        }
    } while (unw_step(&c) > 0);

    _UPT_destroy(upt);
#endif
}

void Debugger::detach()
{
    if (t)
    {
        t->requestInterruption();
    }
}

const static auto whitespace = QRegularExpression("\\s+");
const static auto minus = QRegularExpression("-");

std::optional<process_memory_map> read_memory_map(int pid)
{
    QFile maps(("/proc/" + std::to_string(pid) + "/maps").c_str());

    if (!maps.open(QIODevice::ReadOnly | QIODevice::Text))
    {
        qDebug() << maps.error();
        return {};
    }

    std::vector<memory_mapping> ret;

    QTextStream in(&maps);
    while(1)
    {
        auto line = in.readLine();
        if (line.isNull())
            break;

        auto list = line.split(whitespace, Qt::SkipEmptyParts);

        auto addresses = list.at(0).split(minus);
        auto start_addr = addresses.at(0).toULong(nullptr, 16);
        auto end_addr = addresses.at(1).toULong(nullptr, 16);

        auto permissions = list.at(1);
        char prot = 0;
        for (auto &&a : permissions)
        {
            if (a == 'r')
                prot |= 4;
            if (a == 'w')
                prot |= 2;
            if (a == 'x')
                prot |= 1;
        }

        QString name_or_file = "";
        if (6 == list.count())
            name_or_file = list.at(5);

        ret.push_back({
            start_addr,
            end_addr,
            prot,
            name_or_file.replace("[anon:", "").replace("]", "").replace("[", "").toStdString()
        });
    }

    return process_memory_map{ret};
}

Debugger::~Debugger()
{
    if (t)
        t->deleteLater();
}


