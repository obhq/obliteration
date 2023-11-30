#pragma once

#include <QString>
#include <QUtf8StringView>

struct error;
struct param;

extern "C" {
    void error_free(error *err);
    const char *error_message(const error *err);

    param *param_open(const char *file, error **error);
    void param_close(param *param);

    void param_category_get(const param *param, QString &buf);
    void param_title_get(const param *param, QString &buf);
    void param_title_id_get(const param *param, QString &buf);
    void param_version_get(const param *param, QString &buf);

    error *system_download(
        const char *from,
        const char *to,
        bool explicit_decryption,
        void (*status) (const char *, std::uint64_t, std::uint64_t, void *),
        void *ud);
}

class Error final {
public:
    Error() : m_obj(nullptr) {}
    Error(error *obj) : m_obj(obj) {}
    Error(const Error &) = delete;
    ~Error()
    {
        if (m_obj) {
            error_free(m_obj);
        }
    }

public:
    Error &operator=(const Error &) = delete;
    Error &operator=(error *obj)
    {
        if (m_obj) {
            error_free(m_obj);
        }

        m_obj = obj;
        return *this;
    }

    error **operator&()
    {
        if (m_obj) {
            error_free(m_obj);
            m_obj = nullptr;
        }

        return &m_obj;
    }

    operator bool() const { return m_obj != nullptr; }

public:
    // The caller must check if this error has a value before calling this method.
    const char *message() const { return error_message(m_obj); }

private:
    error *m_obj;
};

class Param final {
public:
    Param() : m_obj(nullptr) {}
    explicit Param(param *obj) : m_obj(obj) {}
    Param(const Param &) = delete;
    ~Param() { close(); }

public:
    Param &operator=(const Param &) = delete;
    Param &operator=(param *obj)
    {
        if (m_obj) {
            param_close(m_obj);
        }

        m_obj = obj;
        return *this;
    }

    operator param *() const { return m_obj; }

public:
    QString category() const
    {
        QString s;
        param_category_get(m_obj, s);
        return s;
    }

    QString title() const
    {
        QString s;
        param_title_get(m_obj, s);
        return s;
    }

    QString titleId() const
    {
        QString s;
        param_title_id_get(m_obj, s);
        return s;
    }

    QString version() const
    {
        QString s;
        param_version_get(m_obj, s);
        return s;
    }

    void close()
    {
        if (m_obj) {
            param_close(m_obj);
            m_obj = nullptr;
        }
    }

private:
    param *m_obj;
};
