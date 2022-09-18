#include "initialize_dialog.hpp"
#include "settings.hpp"

#include <QDialogButtonBox>
#include <QDir>
#include <QFileDialog>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QMessageBox>
#include <QPushButton>
#include <QVBoxLayout>

InitializeDialog::InitializeDialog() :
    m_gamesDirectory(nullptr)
{
    auto layout = new QVBoxLayout(this);

    layout->addWidget(setupGamesDirectory());
    layout->addStretch();
    layout->addWidget(setupDialogActions());

    setWindowTitle("Initialize Obliteration");
}

InitializeDialog::~InitializeDialog()
{
}

void InitializeDialog::browse()
{
    auto path = QFileDialog::getExistingDirectory(this, "Location for PKG files");

    if (!path.isEmpty()) {
        m_gamesDirectory->setText(QDir::toNativeSeparators(path));
    }
}

QWidget *InitializeDialog::setupGamesDirectory()
{
    auto group = new QGroupBox("Location for PKG files");
    auto layout = new QHBoxLayout(group);

    // Input.
    m_gamesDirectory = new QLineEdit();
    m_gamesDirectory->setText(readGamesDirectorySetting());
    m_gamesDirectory->setMinimumWidth(static_cast<int>(400.0 * devicePixelRatioF()));

    layout->addWidget(m_gamesDirectory);

    // Browse button.
    auto browse = new QPushButton("...");

    connect(browse, &QPushButton::clicked, this, &InitializeDialog::browse);

    layout->addWidget(browse);

    return group;
}

QWidget *InitializeDialog::setupDialogActions()
{
    auto actions = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel);

    connect(actions, &QDialogButtonBox::accepted, this, &InitializeDialog::save);
    connect(actions, &QDialogButtonBox::rejected, this, &InitializeDialog::reject);

    return actions;
}

void InitializeDialog::save()
{
    // Check games directory.
    auto gamesDirectory = m_gamesDirectory->text();

    if (gamesDirectory.isEmpty() || !QDir(gamesDirectory).exists() || !QDir::isAbsolutePath(gamesDirectory)) {
        QMessageBox::critical(this, "Error", "The value for directory to store games is not valid.");
        return;
    }

    // Write settings and close dialog.
    writeGamesDirectorySetting(QDir::toNativeSeparators(gamesDirectory));

    accept();
}
