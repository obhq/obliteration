#include "launch_settings.hpp"

#include <QPushButton>
#include <QVBoxLayout>

LaunchSettings::LaunchSettings(QWidget *parent) :
    QWidget(parent)
{
    auto layout = new QVBoxLayout();

    // Start button.
    auto start = new QPushButton("Start");

    connect(start, &QAbstractButton::clicked, [this]() { emit startClicked(); });

    layout->addWidget(start);

    setLayout(layout);
}

LaunchSettings::~LaunchSettings()
{
}
