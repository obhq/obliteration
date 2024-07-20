#pragma once

#include <QString>
#include <QUtf8StringView>

#include <cstddef>
#include <cstdint>

struct error;
struct param;
struct pkg;
struct vmm;

typedef void (*pkg_extract_status_t) (const char *status, std::size_t bar, std::uint64_t current, std::uint64_t total, void *ud);

extern "C" {
    void error_free(error *err);
    const char *error_message(const error *err);

    param *param_open(const char *file, error **error);
    void param_close(param *param);

    void param_app_ver_get(const param *param, QString &buf);
    void param_category_get(const param *param, QString &buf);
    void param_content_id_get(const param *param, QString &buf);
    void param_short_content_id_get(const param *param, QString &buf);
    void param_title_get(const param *param, QString &buf);
    void param_title_id_get(const param *param, QString &buf);
    void param_version_get(const param *param, QString &buf);

    pkg *pkg_open(const char *file, error **error);
    void pkg_close(pkg *pkg);

    param *pkg_get_param(const pkg *pkg, error **error);
    error *pkg_extract(const pkg *pkg, const char *dir, pkg_extract_status_t status, void *ud);

    error *update_firmware(
        const char *root,
        const char *fw,
        void *cx,
        void (*status) (const char *, std::uint64_t, std::uint64_t, void *));

    vmm *vmm_new();
    void vmm_free(vmm *vmm);
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
    QString appver() const
    {
        QString s;
        param_app_ver_get(m_obj, s);
        return s;
    }

    QString category() const
    {
        QString s;
        param_category_get(m_obj, s);
        return s;
    }

    QString contentId() const
    {
        QString s;
        param_content_id_get(m_obj, s);
        return s;
    }

    QString shortContentId() const
    {
        QString s;
        param_short_content_id_get(m_obj, s);
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

class Pkg final {
public:
    Pkg() : m_obj(nullptr) {}
    Pkg(const Pkg &) = delete;
    Pkg(Pkg &&other)
    {
        m_obj = other.m_obj;
        other.m_obj = nullptr;
    }

    ~Pkg()
    {
        close();
    }

public:
    Pkg &operator=(const Pkg &) = delete;
    Pkg &operator=(pkg *obj)
    {
        if (m_obj) {
            pkg_close(m_obj);
        }

        m_obj = obj;
        return *this;
    }

    operator pkg *() const { return m_obj; }

public:
    void close()
    {
        if (m_obj) {
            pkg_close(m_obj);
            m_obj = nullptr;
        }
    }

private:
    pkg *m_obj;
};

class Vmm final {
public:
    Vmm() : m_obj(nullptr) {}
    Vmm(const Vmm &) = delete;
    ~Vmm() { kill(); }

public:
    Vmm &operator=(const Vmm &) = delete;
    operator bool() const { return m_obj != nullptr; }

public:
    void kill()
    {
        if (m_obj) {
            vmm_free(m_obj);
            m_obj = nullptr;
        }
    }

private:
    vmm *m_obj;
};
