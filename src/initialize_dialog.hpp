#pragma once

#include <QDialog>

class InitializeDialog final : public QDialog {
public:
    InitializeDialog();
    ~InitializeDialog();

private:
    QWidget *setupGamesDirectory();
    QWidget *setupDialogActions();
};
