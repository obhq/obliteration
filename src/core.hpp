#pragma once

#include "core.h"

#include <stdlib.h>

template<typename T>
class RustPtr final {
public:
    RustPtr() : m_ptr(nullptr) {}
    explicit RustPtr(T *ptr) : m_ptr(ptr) {}
    RustPtr(const RustPtr &) = delete;

    RustPtr(RustPtr &&other) : m_ptr(other.m_ptr)
    {
        other.m_ptr = nullptr;
    }

    ~RustPtr()
    {
        free();
    }

    RustPtr &operator=(const RustPtr &) = delete;

    RustPtr &operator=(RustPtr &&other)
    {
        free();

        m_ptr = other.m_ptr;
        other.m_ptr = nullptr;

        return *this;
    }

    RustPtr &operator=(T *ptr)
    {
        free();
        m_ptr = ptr;
        return *this;
    }

    operator T *() { return m_ptr; }
    operator bool() const { return m_ptr != nullptr; }

    T **operator&()
    {
        free();
        return &m_ptr;
    }

    T *get() { return m_ptr; }
    void free();
private:
    T *m_ptr;
};

template<>
inline void RustPtr<char>::free()
{
    ::free(m_ptr);
    m_ptr = nullptr;
}

template<>
inline void RustPtr<Param>::free()
{
    if (m_ptr) {
        param_close(m_ptr);
        m_ptr = nullptr;
    }
}

template<>
inline void RustPtr<Pkg>::free()
{
    if (m_ptr) {
        pkg_close(m_ptr);
        m_ptr = nullptr;
    }
}

template<>
inline void RustPtr<RustError>::free()
{
    if (m_ptr) {
        error_free(m_ptr);
        m_ptr = nullptr;
    }
}

template<>
inline void RustPtr<Vmm>::free()
{
    if (m_ptr) {
        vmm_free(m_ptr);
        m_ptr = nullptr;
    }
}
