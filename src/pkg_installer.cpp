#include "pkg_installer.hpp"
#include "core.hpp"
#include "path.hpp"
#include "pkg_extractor.hpp"

#include <QCloseEvent>
#include <QCoreApplication>
#include <QDir>
#include <QMessageBox>
#include <QPlainTextEdit>
#include <QProgressBar>
#include <QScrollBar>
#include <QThread>
#include <QVBoxLayout>

#include <cstddef>
#include <string>
#include <utility>

#include <string.h>

PkgInstaller::PkgInstaller(const QString &games, const QString &pkg, QWidget *parent) :
    QDialog(parent),
    m_games(games),
    m_pkg(pkg),
    m_bar1(nullptr),
    m_bar2(nullptr),
    m_log(nullptr),
    m_completed(false)
{
    auto layout = new QVBoxLayout(this);

    // Primary bar.
    m_bar1 = new QProgressBar();
    m_bar1->setMaximum(0);
    m_bar1->setTextVisible(false);
    m_bar1->setMinimumWidth(500);

    layout->addWidget(m_bar1);

    // Secondary bar.
    m_bar2 = new QProgressBar();
    m_bar2->setMaximum(0);
    m_bar2->setTextVisible(false);

    layout->addWidget(m_bar2);

    // Log.
    m_log = new QPlainTextEdit();
    m_log->setReadOnly(true);
    m_log->setLineWrapMode(QPlainTextEdit::NoWrap);
    m_log->setMinimumHeight(200);

#ifdef _WIN32
    m_log->document()->setDefaultFont(QFont("Courier New", 10));
#elif __APPLE__
    m_log->document()->setDefaultFont(QFont("menlo", 10));
#else
    m_log->document()->setDefaultFont(QFont("monospace", 10));
#endif

    layout->addWidget(m_log);

    setWindowTitle("Install PKG");
}

PkgInstaller::~PkgInstaller()
{
}

int PkgInstaller::exec()
{
    // Show the dialog.
    setModal(true);
    show();

    // Wait until the dialog is visible otherwise the user will see noting until the pkg_open is
    // returned, which can take a couple of seconds.
    while (!isVisible()) {
        QCoreApplication::processEvents();
    }

    // Open a PKG.
    Rust<Pkg> pkg;
    Rust<RustError> error;

    log(QString("Opening %1").arg(m_pkg));
    pkg = pkg_open(m_pkg.toStdString().c_str(), &error);

    if (!pkg) {
        QMessageBox::critical(
            this,
            "Error",
            QString("Couldn't open %1: %2").arg(m_pkg).arg(error_message(error)));
        return Rejected;
    }

    // Get param.sfo.
    Rust<Param> param;

    param = pkg_get_param(pkg, &error);

    if (!param) {
        QMessageBox::critical(
            this,
            "Error",
            QString("Couldn't get param.sfo from %1: %2").arg(m_pkg).arg(error_message(error)));
        return Rejected;
    }

    // Get path to install.
    Rust<char> id, category, appver, title;

    id = param_title_id_get(param);
    category = param_category_get(param);
    appver = param_app_ver_get(param);
    title = param_title_get(param);

    auto directory = joinPath(m_games, id.get());

    if (strcmp(category, "gp") == 0) {
        directory.append("-PATCH-").append(appver);
    } else if (strcmp(category, "ac") == 0) {
        // TODO: Add DLC support, short_content_id is most likely to be used.
        QMessageBox::critical(this, "Error", "DLC PKG support is not yet implemented.");
        return Rejected;
    } else if (strcmp(category, "gd")) {
        QMessageBox::critical(
            this,
            "Error",
            QString("Don't know how to install a PKG with category = %1.").arg(category.get()));
        return Rejected;
    }

    // Create game directory.
    auto path = QString::fromStdString(directory);

    log(QString("Creating %1").arg(path));

    if (!QDir().mkdir(path)) {
        QMessageBox::critical(this, "Error", QString("Couldn't create %1").arg(path));
        return Rejected;
    }

    setWindowTitle(title.get());

    // Setup extractor.
    QThread background;
    QObject context;
    QString fail;
    auto finished = false;
    auto extractor = new PkgExtractor(std::move(pkg), std::move(directory));

    extractor->moveToThread(&background);

    connect(&background, &QThread::started, extractor, &PkgExtractor::exec);
    connect(&background, &QThread::finished, extractor, &QObject::deleteLater);
    connect(extractor, &PkgExtractor::statusChanged, this, &PkgInstaller::update);
    connect(extractor, &PkgExtractor::finished, &context, [&](const QString &e) {
        fail = e;
        finished = true;
    });

    // Start extraction.
    background.start();

    while (!finished) {
        QCoreApplication::processEvents(QEventLoop::WaitForMoreEvents);
    }

    // Clean up.
    background.quit();
    background.wait();

    // Check if failed.
    if (!fail.isEmpty()) {
        QMessageBox::critical(this, "Error", QString("Failed to extract %1: %2").arg(m_pkg).arg(fail));
        return Rejected;
    }

    // Close the dialog.
    m_completed = true;

    close();

    while (isVisible()) {
        QCoreApplication::processEvents();
    }

    // Set success data.
    if (strcmp(category, "gd") == 0) {
        m_gameId = std::move(id);
    }

    return Accepted;
}

void PkgInstaller::closeEvent(QCloseEvent *event)
{
    // Do not allow the user to close the dialog until completed.
    if (!m_completed) {
        event->ignore();
    } else {
        QDialog::closeEvent(event);
    }
}

void PkgInstaller::keyPressEvent(QKeyEvent *event)
{
    // Do not allow the user to close the dialog until completed.
    event->ignore();
}

void PkgInstaller::update(const QString &status, std::size_t bar, std::uint64_t current, std::uint64_t total)
{
    switch (bar) {
    case 0:
        if (current) {
            m_bar1->setValue(static_cast<int>(current));
        } else {
            m_bar1->setValue(0);
            m_bar1->setMaximum(static_cast<int>(total));
        }
        break;
    case 1:
        if (current) {
            if (current == total) {
                m_bar2->setValue(1000000);
            } else {
                auto scale = static_cast<double>(current) / static_cast<double>(total);
                m_bar2->setValue(static_cast<int>(scale * 1000000.0));
            }
        } else if (total) {
            m_bar2->setValue(0);
            m_bar2->setMaximum(1000000);
        } else {
            m_bar2->setValue(0);
            m_bar2->setMaximum(0);
        }
        break;
    }

    if (status.isEmpty()) {
        QCoreApplication::processEvents();
    } else {
        log(status);
    }
}

void PkgInstaller::log(const QString &msg)
{
    auto scroll = m_log->verticalScrollBar();

    m_log->appendPlainText(msg);
    scroll->setValue(scroll->maximum());

    QCoreApplication::processEvents();
}
