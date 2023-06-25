#pragma once

#include "error.hpp"

struct param;

extern "C" {
    param *param_open(const char *file, error **error);
    void param_close(param *param);

    const char *param_title(const param *param);
    const char *param_title_id(const param *param);
}

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
    const char *title() const { return param_title(m_obj); }
    const char *titleId() const { return param_title_id(m_obj); }

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
