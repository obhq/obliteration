#include "cpu_settings.hpp"

#include <QGridLayout>
#include <QGroupBox>
#include <QLabel>
#include <QSlider>

CpuSettings::CpuSettings(QWidget *parent) :
    QWidget(parent)
{
    auto layout = new QGridLayout();

    layout->addWidget(buildCount(), 0, 0);
    layout->setRowStretch(1, 1);

    setLayout(layout);
}

CpuSettings::~CpuSettings()
{
}

QWidget *CpuSettings::buildCount()
{
    auto group = new QGroupBox("Count");
    auto layout = new QGridLayout();

    // Slider.
    auto slider = new QSlider(Qt::Horizontal);

    slider->setTickInterval(1);
    slider->setTickPosition(QSlider::TicksAbove);
    slider->setRange(1, 16);
    slider->setValue(8);

    layout->addWidget(slider, 0, 0);

    // Value.
    auto value = new QLabel("8");

    connect(slider, &QAbstractSlider::valueChanged, value, qOverload<int>(&QLabel::setNum));

    layout->addWidget(value, 0, 1);

    // Description.
    auto desc = new QLabel("Changing this value to other than 8 may crash the game.");

    desc->setWordWrap(true);

    layout->addWidget(desc, 1, 0, 1, -1);

    group->setLayout(layout);

    return group;
}
