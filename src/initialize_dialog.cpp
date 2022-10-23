#include "initialize_dialog.hpp"
#include "settings.hpp"

#include <QDialogButtonBox>
#include <QDir>
#include <QFileDialog>
#include <QFormLayout>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QMessageBox>
#include <QPushButton>
#include <QVBoxLayout>

InitializeDialog::InitializeDialog() :
    m_systemDirectory(nullptr),
    m_gamesDirectory(nullptr)
{
    auto layout = new QVBoxLayout(this);

    layout->addLayout(setupSettings());
    layout->addStretch();
    layout->addWidget(setupDialogActions());

    setWindowTitle("Initialize Obliteration");
}

InitializeDialog::~InitializeDialog()
{
}

void InitializeDialog::browseSystemDirectory()
{
    auto path = QFileDialog::getExistingDirectory(this, "Location to install system files");

    if (!path.isEmpty()) {
        m_systemDirectory->setText(QDir::toNativeSeparators(path));
    }
}

void InitializeDialog::browseGamesDirectory()
{
    auto path = QFileDialog::getExistingDirectory(this, "Location to install games");

    if (!path.isEmpty()) {
        m_gamesDirectory->setText(QDir::toNativeSeparators(path));
    }
}

QLayout *InitializeDialog::setupSettings()
{
    auto layout = new QFormLayout();

    layout->setLabelAlignment(Qt::AlignRight);
    layout->addRow("Path to install system files:", setupSystemDirectory());
    layout->addRow("Path to install games:", setupGamesDirectory());

    return layout;
}

QLayout *InitializeDialog::setupSystemDirectory()
{
    auto layout = new QHBoxLayout();

    // Input.
    m_systemDirectory = new QLineEdit();
    m_systemDirectory->setText(readSystemDirectorySetting());
    m_systemDirectory->setMinimumWidth(400);

    layout->addWidget(m_systemDirectory);

    // Browse button.
    auto browse = new QPushButton("...");

    connect(browse, &QPushButton::clicked, this, &InitializeDialog::browseSystemDirectory);

    layout->addWidget(browse);

    return layout;
}

QLayout *InitializeDialog::setupGamesDirectory()
{
    auto layout = new QHBoxLayout();

    // Input.
    m_gamesDirectory = new QLineEdit();
    m_gamesDirectory->setText(readGamesDirectorySetting());
    m_gamesDirectory->setMinimumWidth(400);

    layout->addWidget(m_gamesDirectory);

    // Browse button.
    auto browse = new QPushButton("...");

    connect(browse, &QPushButton::clicked, this, &InitializeDialog::browseGamesDirectory);

    layout->addWidget(browse);

    return layout;
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
    // Check systen directory.
    auto systemDirectory = m_systemDirectory->text();

    if (systemDirectory.isEmpty() || !QDir(systemDirectory).exists() || !QDir::isAbsolutePath(systemDirectory)) {
        QMessageBox::critical(this, "Error", "The value for location to install system files is not valid.");
        return;
    }

    // Check games directory.
    auto gamesDirectory = m_gamesDirectory->text();

    if (gamesDirectory.isEmpty() || !QDir(gamesDirectory).exists() || !QDir::isAbsolutePath(gamesDirectory)) {
        QMessageBox::critical(this, "Error", "The value for location to install games is not valid.");
        return;
    }

    // Write settings and close dialog.
    writeSystemDirectorySetting(QDir::toNativeSeparators(systemDirectory));
    writeGamesDirectorySetting(QDir::toNativeSeparators(gamesDirectory));

    accept();
}
