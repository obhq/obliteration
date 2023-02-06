// This file is a derived works from Qt Creator.
// Licensed under GPL-3.0-only.
#pragma once

#include "ansi_escape.hpp"

#include <QList>
#include <QObject>
#include <QTextCharFormat>
#include <QTextCursor>

class QPlainTextEdit;

enum LogFormat {
    InfoMessageFormat,
    ErrorMessageFormat,
    WarnMessageFormat,
    NumberOfFormats // Keep this entry last.
};

class LogFormatter : public QObject {
    Q_OBJECT
public:
    LogFormatter(QPlainTextEdit *plainTextEdit);
    ~LogFormatter() override;

public:
    void appendMessage(const QString &text, LogFormat format);
    void reset();

private:
    void initFormats();
    void doAppendMessage(const QString &text, LogFormat format);
    void append(const QString &text, const QTextCharFormat &format);
    void flushTrailingNewline();
    QTextCharFormat charFormat(LogFormat format) const;
    QList<FormattedText> parseAnsi(const QString &text, const QTextCharFormat &format);
    void dumpIncompleteLine(const QString &line, LogFormat format);
    void flushIncompleteLine();
    void clearLastLine();
    void scroll();

private:
    AnsiEscape m_escapeCodeHandler;
    QPlainTextEdit *m_plainTextEdit;
    QTextCursor m_cursor;
    QTextCharFormat m_formats[NumberOfFormats];
    QPair<QString, LogFormat> m_incompleteLine;
    bool m_prependLineFeed;
    bool m_prependCarriageReturn;
};
