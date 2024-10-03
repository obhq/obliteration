#pragma once

#include <QDialog>

class QPlainTextEdit;
class QProgressBar;

class PkgInstaller final : public QDialog {
public:
    PkgInstaller(const QString &games, const QString &pkg, QWidget *parent = nullptr);
    ~PkgInstaller() override;

public:
    int exec() override;

public:
    const QString &gameId() const { return m_gameId; }

protected:
    void closeEvent(QCloseEvent *event) override;
    void keyPressEvent(QKeyEvent *event) override;

private slots:
    void update(const QString &status, std::size_t bar, std::uint64_t current, std::uint64_t total);

private:
    void log(const QString &msg);

private:
    QString m_games;
    QString m_pkg;
    QProgressBar *m_bar1;
    QProgressBar *m_bar2;
    QPlainTextEdit *m_log;
    QString m_gameId;
    bool m_completed;
};
