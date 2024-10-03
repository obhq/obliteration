// This file is a derived works from Qt Creator.
// Licensed under GPL-3.0-only.
#pragma once

#include <QList>
#include <QString>
#include <QTextCharFormat>

class FormattedText {
public:
    FormattedText() = default;
    FormattedText(const QString &txt, const QTextCharFormat &fmt = QTextCharFormat()) :
        text(txt),
        format(fmt)
    {
    }

    QString text;
    QTextCharFormat format;
};

class AnsiEscape {
public:
    QList<FormattedText> parseText(const FormattedText &input);
    void endFormatScope();

private:
    void setFormatScope(const QTextCharFormat &charFormat);

private:
    bool            m_previousFormatClosed = true;
    bool            m_waitingForTerminator = false;
    QString         m_alternateTerminator;
    QTextCharFormat m_previousFormat;
    QString         m_pendingText;
};
