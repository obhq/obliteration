#include "update_firmware.hpp"

#include <QGridLayout>
#include <QGroupBox>
#include <QLabel>
#include <QLineEdit>
#include <QSizePolicy>
#include <QVBoxLayout>

UpdateFirmware::UpdateFirmware(QWidget *parent) :
    QWidget(parent),
    m_from(nullptr)
{
    auto layout = new QVBoxLayout();

    layout->addWidget(setupFromGroup());

    setLayout(layout);
}

UpdateFirmware::~UpdateFirmware()
{
}

QString UpdateFirmware::from() const
{
    return m_from->text();
}

QGroupBox *UpdateFirmware::setupFromGroup()
{
    auto group = new QGroupBox("PlayStation 4");
    auto layout = new QGridLayout();

    // Address label.
    auto address = new QLabel("&Address:");
    layout->addWidget(address, 0, 0);

    // Input.
    m_from = new QLineEdit();
    address->setBuddy(m_from);
    layout->addWidget(m_from, 0, 1);

    // Description.
    auto desc = new QLabel(
        R"(Specify the address of FTP server that are running on your jailbroken PS4 (e.g. 192.168.1.123:1337). )"
        R"(The FTP server must be capable of firmware decryption.)");

    desc->setSizePolicy(QSizePolicy::MinimumExpanding, QSizePolicy::Minimum);
    desc->setWordWrap(true);
    desc->setOpenExternalLinks(true);

    layout->addWidget(desc, 1, 0, 1, 2);

    group->setLayout(layout);

    return group;
}
