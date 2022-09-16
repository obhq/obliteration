#include "initialize_dialog.hpp"
#include "ui_initialize_dialog.h"

InitializeDialog::InitializeDialog() :
    ui(new Ui::InitializeDialog())
{
    ui->setupUi(this);
}

InitializeDialog::~InitializeDialog()
{
    delete ui;
}
