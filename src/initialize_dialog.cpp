#include "initialize_dialog.hpp"

#include <QDialogButtonBox>
#include <QFileDialog>
#include <QGridLayout>
#include <QGroupBox>
#include <QLabel>
#include <QLineEdit>
#include <QPushButton>
#include <QVBoxLayout>

InitializeDialog::InitializeDialog() :
    gamesDirectory(nullptr)
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
    auto path = QFileDialog::getExistingDirectory(this, "Location to store games");

    if (!path.isEmpty()) {
        gamesDirectory->setText(path);
    }
}

QWidget *InitializeDialog::setupGamesDirectory()
{
    auto group = new QGroupBox("Directory to store games");
    auto layout = new QGridLayout(group);

    // Input.
    gamesDirectory = new QLineEdit();

    layout->addWidget(gamesDirectory, 0, 0);

    // Browse button.
    auto browse = new QPushButton("...");

    connect(browse, &QPushButton::clicked, this, &InitializeDialog::browse);

    layout->addWidget(browse, 0, 1);

    // Notice text.
    auto notice = new QLabel("If this location already have some games Obliteration will load all of it upon initialization.");

    notice->setStyleSheet("font-style: italic");

    layout->addWidget(notice, 1, 0, 1, 2);

    return group;
}

QWidget *InitializeDialog::setupDialogActions()
{
    auto actions = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel);

    connect(actions, &QDialogButtonBox::accepted, this, &InitializeDialog::accept);
    connect(actions, &QDialogButtonBox::rejected, this, &InitializeDialog::reject);

    return actions;
}
