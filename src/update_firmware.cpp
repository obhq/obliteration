#include "update_firmware.hpp"

#include <QCheckBox>
#include <QGridLayout>
#include <QLabel>
#include <QLineEdit>

UpdateFirmware::UpdateFirmware(QWidget *parent) :
    QWidget(parent),
    m_from(nullptr),
    m_explicitDecryption(nullptr)
{
    auto layout = new QGridLayout();

    // Address label.
    auto address = new QLabel("&Address:");
    layout->addWidget(address, 0, 0);

    // Address input.
    m_from = new QLineEdit();
    address->setBuddy(m_from);
    layout->addWidget(m_from, 0, 1);

    // Address description.
    auto desc = new QLabel(
        R"(IP Address and Port of the FTP server running on your jailbroken PS4 (e.g. 192.168.1.123:2121). )"
        R"(The FTP server must be capable of firmware decryption.)");

    desc->setWordWrap(true);
    desc->setOpenExternalLinks(true);

    layout->addWidget(desc, 1, 0, 1, 2);

    // Explicit decryption checkbox.
    m_explicitDecryption = new QCheckBox("Explicit &decryption");
    layout->addWidget(m_explicitDecryption, 2, 0, 1, 2);

    // Explicit decryption decription.
    desc = new QLabel(
        R"(Enable this if the FTP server requires the command 'DECRYPT' to enable firmware decryption. )"
        R"(If you are unsure, try enabling this first. If the FTP server gives the error 'UNKNOWN COMMAND' then you will need to disable this.)");

    desc->setSizePolicy(QSizePolicy::MinimumExpanding, QSizePolicy::Minimum);
    desc->setWordWrap(true);
    desc->setOpenExternalLinks(true);

    layout->addWidget(desc, 3, 0, 1, 2);

    setLayout(layout);
}

UpdateFirmware::~UpdateFirmware()
{
}

QString UpdateFirmware::from() const
{
    return m_from->text();
}

bool UpdateFirmware::explicitDecryption() const
{
    return m_explicitDecryption->isChecked();
}
