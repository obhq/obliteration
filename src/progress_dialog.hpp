#pragma once

#include <QDialog>

class QLabel;
class QProgressBar;

// QProgressDialog required a positive value to show, which is not a desired behavior in some cases.
// So we invent our own progress dialog.
class ProgressDialog final : public QDialog {
public:
    ProgressDialog(const QString &title, const QString &status, QWidget *parent = nullptr);
    ~ProgressDialog();

public:
    void setMaximum(int v);
    void setValue(int v);

    QString statusText() const;
    void setStatusText(const QString &v);

    void complete();

protected:
    void closeEvent(QCloseEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;

private:
    QProgressBar *m_progress;
    QLabel *m_status;
    bool m_completed;
};
