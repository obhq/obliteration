#include "initialize_wizard.hpp"
#include "settings.hpp"
#include "system.hpp"
#include "update_firmware.hpp"

#include <QDir>
#include <QFileDialog>
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
    PageIntro,
    PageSystem,
    PageGame,
    PageFirmware,
    PageConclusion
};

class IntroPage : public QWizardPage {
public:
    IntroPage()
    {
        auto layout = new QVBoxLayout();

        // Page properties.
        setTitle("Introduction");

        // Introduction.
        auto intro = new QLabel(
            "This wizard will help you setup Obliteration. To ensure you're ready, make sure you "
            "have a jailbroken PS4 with an enabled FTP server. You will also need your PS4's IP "
            "address and the port used for FTP connection.");
        intro->setWordWrap(true);
        layout->addWidget(intro);

        setLayout(layout);
    }
};

class SystemPage : public QWizardPage {
public:
    SystemPage() : m_input(nullptr)
    {
        auto layout = new QVBoxLayout();

        // Page properties.
        setTitle("Location for system files");
        setSubTitle("The selected directory will be used for any PS4 data. (Save Data and Firmware Files).");

        // Widgets.
        layout->addLayout(setupInputRow());

        setLayout(layout);
    }

    bool validatePage() override
    {
        auto path = m_input->text();

        if (!QDir::isAbsolutePath(path)) {
            QMessageBox::critical(this, "Error", "The location must be an absolute path.");
            return false;
        }

        if (!QDir(path).exists()) {
            QMessageBox::critical(this, "Error", "The location does not exist.");
            return false;
        }

        return true;
    }

private:
    QLayout *setupInputRow()
    {
        auto layout = new QHBoxLayout();

        // Label.
        auto label = new QLabel("&Location:");
        layout->addWidget(label);

        // Input.
        m_input = new QLineEdit();
        m_input->setText(readSystemDirectorySetting());

        label->setBuddy(m_input);
        layout->addWidget(m_input);

        registerField(FIELD_SYSTEM_LOCATION "*", m_input);

        // Browse.
        auto browse = new QPushButton("...");

        connect(browse, &QPushButton::clicked, this, &SystemPage::browseDirectory);

        layout->addWidget(browse);

        return layout;
    }

    void browseDirectory()
    {
        auto path = QFileDialog::getExistingDirectory(this, "Location for system files");

        if (!path.isEmpty()) {
            m_input->setText(QDir::toNativeSeparators(path));
        }
    }

private:
    QLineEdit *m_input;
};

class GamePage : public QWizardPage {
public:
    GamePage() : m_input(nullptr)
    {
        auto layout = new QVBoxLayout();

        // Page properties.
        setTitle("Location to install games");
        setSubTitle("The selected directory will be used for game installation. The directory cannot be the same as the system directory.");

        // Widgets.
        layout->addLayout(setupInputRow());

        setLayout(layout);
    }

    bool validatePage() override
    {
        auto path = m_input->text();

        if (!QDir::isAbsolutePath(path)) {
            QMessageBox::critical(this, "Error", "The specified location must be an absolute path.");
            return false;
        }

        if (!QDir(path).exists()) {
            QMessageBox::critical(this, "Error", "The specified location does not exist.");
            return false;
        }

        if (path == field(FIELD_SYSTEM_LOCATION).toString()) {
            QMessageBox::critical(this, "Error", "The specified location cannot be the same as the system directory.");
            return false;
        }

        return true;
    }

private:
    QLayout *setupInputRow()
    {
        auto layout = new QHBoxLayout();

        // Label.
        auto label = new QLabel("&Location:");
        layout->addWidget(label);

        // Input.
        m_input = new QLineEdit();
        m_input->setText(readGamesDirectorySetting());

        label->setBuddy(m_input);
        layout->addWidget(m_input);

        registerField(FIELD_GAMES_LOCATION "*", m_input);

        // Browse button.
        auto browse = new QPushButton("...");

        connect(browse, &QPushButton::clicked, this, &GamePage::browseDirectory);

        layout->addWidget(browse);

        return layout;
    }

    void browseDirectory()
    {
        auto path = QFileDialog::getExistingDirectory(this, "Location to install games");

        if (!path.isEmpty()) {
            m_input->setText(QDir::toNativeSeparators(path));
        }
    }

private:
    QLineEdit *m_input;
};

class FirmwarePage : public QWizardPage {
public:
    FirmwarePage() : m_form(nullptr)
    {
        auto layout = new QVBoxLayout();

        // Page properties.
        setTitle("Install firmware");
        setSubTitle("Obliteration requires some firmware files from your PS4 in order to work.");

        // Page widgets.
        m_form = new UpdateFirmware();
        layout->addWidget(m_form);

        setLayout(layout);
    }

    bool validatePage() override
    {
        // Get system path.
        auto systemPath = wizard()->hasVisitedPage(PageSystem)
            ? field(FIELD_SYSTEM_LOCATION).toString()
            : readSystemDirectorySetting();

        // Load update form.
        auto from = m_form->from();

        if (from.isEmpty()) {
            QMessageBox::critical(this, "Error", "No FTP server was specified.");
            return false;
        }

        auto explicitDecryption = m_form->explicitDecryption();

        // Install.
        return initSystem(systemPath, from, explicitDecryption, this);
    }

private:
    UpdateFirmware *m_form;
};

class ConclusionPage : public QWizardPage {
public:
    ConclusionPage()
    {
        auto layout = new QVBoxLayout();

        // Page properties.
        setTitle("Setup complete.");

        // Introduction.
        auto intro = new QLabel("You can now install your games and play them using Obliteration.");
        layout->addWidget(intro);

        setLayout(layout);
    }

    bool validatePage() override
    {
        auto wizard = this->wizard();

        if (wizard->hasVisitedPage(PageSystem)) {
            auto path = field(FIELD_SYSTEM_LOCATION).toString();
            writeSystemDirectorySetting(QDir::toNativeSeparators(path));
        }

        if (wizard->hasVisitedPage(PageGame)) {
            auto path = field(FIELD_GAMES_LOCATION).toString();
            writeGamesDirectorySetting(QDir::toNativeSeparators(path));
        }

        return true;
    }
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
    setPage(PageIntro, new IntroPage());
    setPage(PageSystem, new SystemPage());
    setPage(PageGame, new GamePage());
    setPage(PageFirmware, new FirmwarePage());
    setPage(PageConclusion, new ConclusionPage());
}

InitializeWizard::~InitializeWizard()
{
}

int InitializeWizard::nextId() const
{
    switch (currentId()) {
    case PageIntro:
        if (!hasSystemDirectorySetting()) {
            return PageSystem;
        }

        Q_FALLTHROUGH();
    case PageSystem:
        if (!hasGamesDirectorySetting()) {
            return PageGame;
        }

        Q_FALLTHROUGH();
    case PageGame:
        if (hasVisitedPage(PageSystem)) {
            // No system path has been configured before.
            if (!isSystemInitialized(field(FIELD_SYSTEM_LOCATION).toString())) {
                return PageFirmware;
            }
        } else if (!isSystemInitialized()) {
            return PageFirmware;
        }

        Q_FALLTHROUGH();
    case PageFirmware:
        return PageConclusion;
    case PageConclusion:
    default:
        return -1;
    }
}
