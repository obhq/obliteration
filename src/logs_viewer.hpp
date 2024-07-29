#pragma once

#include <QWidget>

class LogFormatter;

class LogsViewer final : public QWidget {
public:
    LogsViewer();
    ~LogsViewer() override;

    void append(const QString &text);
private:
    LogFormatter *m_formatter;
};
