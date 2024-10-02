// This file is a derived works from Qt Creator.
// Licensed under GPL-3.0-only.
#include "log_formatter.hpp"

#include <QBrush>
#include <QPlainTextEdit>
#include <QScrollBar>

static QString normalizeNewlines(const QString &text)
{
    QString res = text;
    const auto newEnd = std::unique(res.begin(), res.end(), [](const QChar c1, const QChar c2) {
        return c1 == '\r' && c2 == '\r'; // QTCREATORBUG-24556
    });
    res.chop(std::distance(newEnd, res.end()));
    res.replace("\r\n", "\n");
    return res;
}

LogFormatter::LogFormatter(QPlainTextEdit *output, QObject *parent) :
    QObject(parent),
    m_output(output),
    m_cursor(output->textCursor()),
    m_prependLineFeed(false),
    m_prependCarriageReturn(false)
{
    m_cursor.movePosition(QTextCursor::End);
}

LogFormatter::~LogFormatter()
{
}

void LogFormatter::appendMessage(const QString &text)
{
    if (text.isEmpty()) {
        return;
    }

    QString out = text;

    if (m_prependCarriageReturn) {
        m_prependCarriageReturn = false;
        out.prepend('\r');
    }

    out = normalizeNewlines(out);

    if (out.endsWith('\r')) {
        m_prependCarriageReturn = true;
        out.chop(1);
    }

    // Forward all complete lines to the specialized formatting code, and handle a
    // potential trailing incomplete line the same way as above.
    for (qsizetype startPos = 0; startPos < out.size();) {
        auto eolPos = out.indexOf('\n', startPos);

        if (eolPos == -1) {
            doAppendMessage(out.mid(startPos));
            break;
        }

        doAppendMessage(out.mid(startPos, eolPos - startPos));

        m_prependLineFeed = true;
        startPos = eolPos + 1;
    }
}

void LogFormatter::reset()
{
    m_output->clear();
    m_prependLineFeed = false;
    m_prependCarriageReturn = false;
    m_escapeCodeHandler = AnsiEscape();
}

void LogFormatter::doAppendMessage(const QString &text)
{
    QTextCharFormat charFmt;
    QList<FormattedText> formattedText = parseAnsi(text, charFmt);

    for (FormattedText output : formattedText) {
        append(output.text, output.format);
    }

    if (formattedText.isEmpty()) {
        append({}, charFmt); // This might cause insertion of a newline character.
    }
}

void LogFormatter::append(const QString &text, const QTextCharFormat &format)
{
    flushTrailingNewline();

    int startPos = 0;
    int crPos = -1;

    while ((crPos = text.indexOf('\r', startPos)) >= 0) {
        m_cursor.insertText(text.mid(startPos, crPos - startPos), format);
        m_cursor.clearSelection();
        m_cursor.movePosition(QTextCursor::StartOfBlock, QTextCursor::KeepAnchor);
        startPos = crPos + 1;
    }

    if (startPos < text.size()) {
        m_cursor.insertText(text.mid(startPos), format);
    }
}

void LogFormatter::flushTrailingNewline()
{
    if (m_prependLineFeed) {
        m_cursor.insertText("\n");
        m_prependLineFeed = false;
        scroll();
    }
}

QList<FormattedText> LogFormatter::parseAnsi(const QString &text, const QTextCharFormat &format)
{
    return m_escapeCodeHandler.parseText(FormattedText(text, format));
}

void LogFormatter::scroll()
{
    auto bar = m_output->verticalScrollBar();
    auto max = bar->maximum();
    auto bottom  = (bar->value() >= (max - 4)); // 4 is an error threshold.

    if (bottom) {
        bar->setValue(max);
    }
}
