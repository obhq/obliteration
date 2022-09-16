#include "initialize_dialog.hpp"

#include <QDialogButtonBox>
#include <QGridLayout>
#include <QGroupBox>
#include <QLabel>
#include <QLineEdit>
#include <QPushButton>
#include <QVBoxLayout>

InitializeDialog::InitializeDialog()
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

QWidget *InitializeDialog::setupGamesDirectory()
{
    auto group = new QGroupBox("Directory to store games");
    auto layout = new QGridLayout(group);

    auto edit = new QLineEdit();

    layout->addWidget(edit, 0, 0);

    auto browse = new QPushButton("...");

    layout->addWidget(browse, 0, 1);

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
