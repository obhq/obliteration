#include "logs_viewer.hpp"
#include "log_formatter.hpp"

#include <QHBoxLayout>
#include <QPlainTextEdit>

LogsViewer::LogsViewer() :
    m_formatter(nullptr)
{
    auto layout = new QHBoxLayout();

    setWindowTitle("Obliteration Logs");

    // Setup viewer.
    auto viewer = new QPlainTextEdit();

    viewer->setReadOnly(true);
    viewer->setLineWrapMode(QPlainTextEdit::NoWrap);
    viewer->setMaximumBlockCount(10000);

#ifdef _WIN32
    viewer->document()->setDefaultFont(QFont("Courier New", 10));
#elif __APPLE__
    viewer->document()->setDefaultFont(QFont("menlo", 10));
#else
    viewer->document()->setDefaultFont(QFont("monospace", 10));
#endif

    layout->addWidget(viewer);

    // Setup formatter.
    m_formatter = new LogFormatter(viewer, this);

    setLayout(layout);
}

LogsViewer::~LogsViewer()
{
}
