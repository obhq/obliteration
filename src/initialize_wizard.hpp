#pragma once

#include <QWizard>

class InitializeWizard : public QWizard {
public:
    InitializeWizard();
    ~InitializeWizard();

public:
    int nextId() const override;
};
