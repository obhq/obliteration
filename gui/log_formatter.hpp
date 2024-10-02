// This file is a derived works from Qt Creator.
// Licensed under GPL-3.0-only.
#pragma once

#include "ansi_escape.hpp"

#include <QList>
#include <QObject>
#include <QTextCharFormat>
#include <QTextCursor>

class QPlainTextEdit;

class LogFormatter : public QObject {
    Q_OBJECT
public:
    LogFormatter(QPlainTextEdit *output, QObject *parent = nullptr);
    ~LogFormatter() override;

public:
    void appendMessage(const QString &text);
    void reset();

private:
    void doAppendMessage(const QString &text);
    void append(const QString &text, const QTextCharFormat &format);
    void flushTrailingNewline();
    QList<FormattedText> parseAnsi(const QString &text, const QTextCharFormat &format);
    void scroll();

private:
    AnsiEscape m_escapeCodeHandler;
    QPlainTextEdit *m_output;
    QTextCursor m_cursor;
    bool m_prependLineFeed;
    bool m_prependCarriageReturn;
};
