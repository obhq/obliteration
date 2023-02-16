#include "initialize_wizard.hpp"
#include "settings.hpp"
#include "system.hpp"

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
        auto intro = new QLabel("This wizard will help you setup Obliteration.");
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
        setSubTitle("The selected directory will be using for any PS4 data like save game data and firmware files.");

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
            QMessageBox::critical(this, "Error", "The location does not exists.");
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
        setSubTitle("The selected directory will be using for game installation. Cannot be the same as system files and must be an empty directory.");

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
            QMessageBox::critical(this, "Error", "The specified location does not exists.");
            return false;
        }

        if (path == field(FIELD_SYSTEM_LOCATION).toString()) {
            QMessageBox::critical(this, "Error", "The specified location cannot be the same location as system files.");
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
    FirmwarePage() : m_completed(false)
    {
        auto layout = new QVBoxLayout();

        // Page properties.
        setTitle("Install firmware");
        setSubTitle("Obliteration required some firmware files from PS4 in order to work. You need to install those files before you can use Obliteration.");

        // Install button.
        auto install = new QPushButton("Install firmware...");

        connect(install, &QPushButton::clicked, this, &FirmwarePage::install);

        layout->addStretch();
        layout->addWidget(install, 0, Qt::AlignHCenter);
        layout->addStretch();

        setLayout(layout);
    }

    bool isComplete() const {
        return m_completed;
    }

private:
    void install()
    {
        // Get system path.
        auto wizard = this->wizard();
        auto systemPath = wizard->hasVisitedPage(PageSystem)
            ? field(FIELD_SYSTEM_LOCATION).toString()
            : readSystemDirectorySetting();

        // Install.
        m_completed = updateSystemFiles(systemPath, this);
        emit completeChanged();

        // Move to next page automatically if installation was completed successfully.
        if (m_completed) {
            wizard->next();
        }
    }

private:
    bool m_completed;
};

class ConclusionPage : public QWizardPage {
public:
    ConclusionPage()
    {
        auto layout = new QVBoxLayout();

        // Page properties.
        setTitle("Setup completed");

        // Introduction.
        auto intro = new QLabel("You can now install your games and play it with Obliteration.");
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
            if (!hasSystemFilesInstalled(field(FIELD_SYSTEM_LOCATION).toString())) {
                return PageFirmware;
            }
        } else if (!hasSystemFilesInstalled()) {
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
