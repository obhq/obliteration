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
    initFormats();
}

LogFormatter::~LogFormatter()
{
}

void LogFormatter::appendMessage(const QString &text, LogFormat format)
{
    if (text.isEmpty()) {
        return;
    }

    // If we have an existing incomplete line and its format is different from this one,
    // then we consider the two messages unrelated. We re-insert the previous incomplete line,
    // possibly formatted now, and start from scratch with the new input.
    if (!m_incompleteLine.first.isEmpty() && m_incompleteLine.second != format) {
        flushIncompleteLine();
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

    // If the input is a single incomplete line, we do not forward it to the specialized
    // formatting code, but simply dump it as-is. Once it becomes complete or it needs to
    // be flushed for other reasons, we remove the unformatted part and re-insert it, this
    // time with proper formatting.
    if (!out.contains('\n')) {
        dumpIncompleteLine(out, format);
        return;
    }

    // We have at least one complete line, so let's remove the previously dumped
    // incomplete line and prepend it to the first line of our new input.
    if (!m_incompleteLine.first.isEmpty()) {
        clearLastLine();
        out.prepend(m_incompleteLine.first);
        m_incompleteLine.first.clear();
    }

    // Forward all complete lines to the specialized formatting code, and handle a
    // potential trailing incomplete line the same way as above.
    for (int startPos = 0; ;) {
        const int eolPos = out.indexOf('\n', startPos);
        if (eolPos == -1) {
            dumpIncompleteLine(out.mid(startPos), format);
            break;
        }
        doAppendMessage(out.mid(startPos, eolPos - startPos), format);
        scroll();
        m_prependLineFeed = true;
        startPos = eolPos + 1;
    }
}

void LogFormatter::reset()
{
    m_output->clear();
    m_prependLineFeed = false;
    m_prependCarriageReturn = false;
    m_incompleteLine.first.clear();
    m_escapeCodeHandler = AnsiEscape();
}

void LogFormatter::initFormats()
{
    m_formats[InfoMessageFormat].setForeground(QBrush(Qt::darkGreen));
    m_formats[ErrorMessageFormat].setForeground(QBrush(Qt::darkRed));
    m_formats[ErrorMessageFormat].setFontWeight(QFont::Bold);
    m_formats[WarnMessageFormat].setForeground(QBrush(Qt::darkYellow));
    m_formats[WarnMessageFormat].setFontWeight(QFont::Bold);
}

void LogFormatter::doAppendMessage(const QString &text, LogFormat format)
{
    QTextCharFormat charFmt = charFormat(format);
    QList<FormattedText> formattedText = parseAnsi(text, charFmt);

    const QString cleanLine = std::accumulate(formattedText.begin(), formattedText.end(), QString(),
            [](const FormattedText &t1, const FormattedText &t2) -> QString
            { return t1.text + t2.text; });

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
    }
}

QTextCharFormat LogFormatter::charFormat(LogFormat format) const
{
    return m_formats[format];
}

QList<FormattedText> LogFormatter::parseAnsi(const QString &text, const QTextCharFormat &format)
{
    return m_escapeCodeHandler.parseText(FormattedText(text, format));
}

void LogFormatter::dumpIncompleteLine(const QString &line, LogFormat format)
{
    if (line.isEmpty())
        return;

    append(line, charFormat(format));
    m_incompleteLine.first.append(line);
    m_incompleteLine.second = format;
}

void LogFormatter::flushIncompleteLine()
{
    clearLastLine();
    doAppendMessage(m_incompleteLine.first, m_incompleteLine.second);
    m_incompleteLine.first.clear();
}

void LogFormatter::clearLastLine()
{
    // Note that this approach will fail if the text edit is not read-only and users
    // have messed with the last line between programmatic inputs.
    // We live with this risk, as all the alternatives are worse.
    if (!m_cursor.atEnd()) {
        m_cursor.movePosition(QTextCursor::End);
    }

    m_cursor.movePosition(QTextCursor::StartOfBlock, QTextCursor::KeepAnchor);
    m_cursor.removeSelectedText();
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
