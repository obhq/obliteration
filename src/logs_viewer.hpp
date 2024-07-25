#pragma once

#include <QWidget>

class LogFormatter;

class LogsViewer final : public QWidget {
public:
    LogsViewer();
    ~LogsViewer() override;
private:
    LogFormatter *m_formatter;
};
