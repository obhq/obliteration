#pragma once

#include <QDialog>

class QLineEdit;

class InitializeDialog final : public QDialog {
public:
    InitializeDialog();
    ~InitializeDialog();

private slots:
    void browse();

private:
    QWidget *setupGamesDirectory();
    QWidget *setupDialogActions();
    void save();

private:
    QLineEdit *gamesDirectory;
};
