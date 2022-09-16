#pragma once

#include <QDialog>

namespace Ui {
    class InitializeDialog;
}

class InitializeDialog final : public QDialog {
public:
    InitializeDialog();
    ~InitializeDialog();

private:
    Ui::InitializeDialog *ui;
};
