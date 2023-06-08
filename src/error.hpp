#pragma once

struct error;

extern "C" {
    void error_free(error *err);
    const char *error_message(const error *err);
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
