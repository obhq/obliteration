#include "progress_dialog.hpp"

#include <QCloseEvent>
#include <QCoreApplication>
#include <QFontMetrics>
#include <QLabel>
#include <QProgressBar>
#include <QTimer>
#include <QVBoxLayout>

ProgressDialog::ProgressDialog(const QString &title, const QString &status, QWidget *parent) :
    QDialog(parent),
    m_completed(false)
{
    // Main layout.
    auto layout = new QVBoxLayout(this);

    layout->setAlignment(Qt::AlignTop);

    // Progress bar.
    m_progress = new QProgressBar();
    m_progress->setMaximum(0);
    m_progress->setTextVisible(false);
    m_progress->setMinimumWidth(400);

    layout->addWidget(m_progress);

    // Status text.
    m_status = new QLabel();
    m_status->setText(status);

    layout->addWidget(m_status);

    // Window's properties.
    setWindowTitle(title);
    setModal(true);
    show();

    // Wait until dialog is visible.
    while (!isVisible()) {
        QCoreApplication::processEvents();
    }
}

ProgressDialog::~ProgressDialog()
{
}

void ProgressDialog::setMaximum(int v)
{
    m_progress->setMaximum(v);
    QCoreApplication::processEvents();
}

void ProgressDialog::setValue(int v)
{
    m_progress->setValue(v);
    QCoreApplication::processEvents();
}

QString ProgressDialog::statusText() const
{
    return m_status->text();
}

void ProgressDialog::setStatusText(const QString &v)
{
    QFontMetrics metrics(m_status->font());

    m_status->setText(metrics.elidedText(v, Qt::ElideRight, m_status->width()));

    QCoreApplication::processEvents();
}

void ProgressDialog::complete()
{
    m_completed = true;

    close();

    while (isVisible()) {
        QCoreApplication::processEvents();
    }
}

void ProgressDialog::closeEvent(QCloseEvent *event)
{
    if (!m_completed) {
        event->ignore();
    } else {
        QDialog::closeEvent(event);
    }
}

void ProgressDialog::keyPressEvent(QKeyEvent *event)
{
    event->ignore();
}
