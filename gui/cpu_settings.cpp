#include "cpu_settings.hpp"

#include <QGridLayout>
#include <QGroupBox>
#include <QLabel>
#include <QLineEdit>
#include <QMessageBox>
#include <QPushButton>
#include <QSlider>

CpuSettings::CpuSettings(QWidget *parent) :
    QWidget(parent),
    m_debugAddr(nullptr)
{
    auto layout = new QGridLayout();

    layout->addWidget(buildCount(), 0, 0);
    layout->addWidget(buildDebug(), 0, 1);
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

QWidget *CpuSettings::buildDebug()
{
    auto group = new QGroupBox("Debug");
    auto layout = new QGridLayout();

    // Address label.
    auto label = new QLabel("Listen address:");

    layout->addWidget(label, 0, 0);

    // Address editor.
    m_debugAddr = new QLineEdit("127.0.0.1:1234");

    label->setBuddy(m_debugAddr);

    layout->addWidget(m_debugAddr, 0, 1);

    // Start.
    auto start = new QPushButton("Start");

    connect(start, &QAbstractButton::clicked, [this] {
        auto addr = m_debugAddr->text();

        if (addr.isEmpty()) {
            QMessageBox::critical(this, "Error", "Listen address cannot be empty.");
            return;
        }

        emit debugClicked(addr);
    });

    layout->addWidget(start, 0, 2);

    // Description.
    auto desc = new QLabel(
        "Specify a TCP address to listen for a debugger. The kernel will wait for a debugger to "
        "connect before start.");

    desc->setWordWrap(true);

    layout->addWidget(desc, 1, 0, 1, -1);

    group->setLayout(layout);

    return group;
}
