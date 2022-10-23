#pragma once

#include <QDialog>

class QLayout;
class QLineEdit;

class InitializeDialog final : public QDialog {
public:
    InitializeDialog();
    ~InitializeDialog();

private slots:
    void browseSystemDirectory();
    void browseGamesDirectory();

private:
    QLayout *setupSettings();
    QLayout *setupSystemDirectory();
    QLayout *setupGamesDirectory();
    QWidget *setupDialogActions();
    void save();

private:
    QLineEdit *m_systemDirectory;
    QLineEdit *m_gamesDirectory;
};
