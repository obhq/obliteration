#include "initialize_wizard.hpp"

#include <QDir>
#include <QFileDialog>
#include <QFileInfo>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QMessageBox>
#include <QPushButton>
#include <QVBoxLayout>
#include <QWizardPage>

#define FIELD_SYSTEM_LOCATION "systemLocation"
#define FIELD_GAMES_LOCATION "gamesLocation"

enum PageId {
    PageFirmware,
};

class FirmwarePage : public QWizardPage {
public:
    FirmwarePage() : m_input(nullptr)
    {
        auto layout = new QVBoxLayout();

        // Page properties.
        setTitle("Install firmware");
        setSubTitle("Select a firmware dump that you got from Firmware Dumper.");

        // Widgets.
        layout->addLayout(setupInputRow());

        setLayout(layout);
    }
private:
    QLayout *setupInputRow()
    {
        auto layout = new QHBoxLayout();

        // Label.
        auto label = new QLabel("&File:");
        layout->addWidget(label);

        // Input.
        m_input = new QLineEdit();

        label->setBuddy(m_input);
        layout->addWidget(m_input);

        // Browse button.
        auto browse = new QPushButton("...");

        connect(browse, &QPushButton::clicked, this, &FirmwarePage::browseDump);

        layout->addWidget(browse);

        return layout;
    }

    void browseDump()
    {
        auto path = QFileDialog::getOpenFileName(this, "Select a firmware dump", {}, "Firmware Dump (*.obf)");

        if (!path.isEmpty()) {
            m_input->setText(QDir::toNativeSeparators(path));
        }
    }

    QLineEdit *m_input;
};

InitializeWizard::InitializeWizard()
{
    // Window properties.
    setWindowTitle("Setup Obliteration");

    // The aero style, which is the default on Windows; does not work well with dark theme.
#ifdef _WIN32
    setWizardStyle(QWizard::ModernStyle);
#endif

    // Pages.
    setPage(PageFirmware, new FirmwarePage());
}

InitializeWizard::~InitializeWizard()
{
}
