#pragma once

#include <atomic>
#include <cstdint>
#include <string>
#include <vector>

namespace uplift
{
  class Runtime;

  typedef uint32_t ObjectHandle;

  template <typename T>
  class object_ref;

  enum class SyscallError : uint64_t;
}

namespace uplift::objects
{
  class Object
  {
  public:
    enum class Type
    {
      Invalid = 0,
      Module,
      Device,
      File,
      SharedMemory,
      Socket,
      Queue,
      Semaphore,
      Eport,
      EventFlag,
      IpmiClient,
    };

  protected:
    Object(Runtime* runtime, Type type);

  public:
    virtual ~Object();

    int32_t pointer_ref_count() const { return pointer_ref_count_; }

    Runtime* runtime() const { return runtime_; }
    Type type() const { return type_; }

    uint32_t handle() const { return handles_[0]; }

    std::vector<ObjectHandle> handles() const { return handles_; }
    std::vector<ObjectHandle>& handles() { return handles_; }

    const std::string& name() const { return name_; }

    void RetainHandle();
    bool ReleaseHandle();
    void Retain();
    void Release();
    uint32_t Delete();

  public:
    virtual SyscallError Close() = 0;
    virtual SyscallError Read(void* data_buffer, size_t data_size, size_t* read_size);
    virtual SyscallError Write(const void* data_buffer, size_t data_size, size_t* written_size);
    virtual SyscallError Truncate(int64_t length);
    virtual SyscallError IOControl(uint32_t request, void* argp);
    virtual SyscallError MMap(void* addr, size_t len, int prot, int flags, size_t offset, void*& allocation);

  protected:
    Runtime* runtime_;

  private:
    std::atomic<int32_t> pointer_ref_count_;

    Type type_;
    std::vector<ObjectHandle> handles_;
    std::string name_;
  };
}

namespace uplift
{
  template <typename T>
  class object_ref
  {
  public:
    object_ref() noexcept : value_(nullptr) {}
    object_ref(std::nullptr_t) noexcept
      : value_(nullptr) {}
    object_ref& operator=(std::nullptr_t) noexcept 
    {
      reset();
      return (*this);
    }

    explicit object_ref(T* value) noexcept : value_(value)
    {
      // Assumes retained on call.
    }
    explicit object_ref(const object_ref& right) noexcept
    {
      reset(right.get());
      if (value_) value_->Retain();
    }
    template <class V, class = typename std::enable_if<
      std::is_convertible<V*, T*>::value, void>::type>
      object_ref(const object_ref<V>& right) noexcept
    {
      reset(right.get());
      if (value_) value_->Retain();
    }

    object_ref(object_ref&& right) noexcept : value_(right.release()) {}
    object_ref& operator=(object_ref&& right) noexcept
    {
      object_ref(std::move(right)).swap(*this);
      return (*this);
    }
    template <typename V>
    object_ref& operator=(object_ref<V>&& right) noexcept
    {
      object_ref(std::move(right)).swap(*this);
      return (*this);
    }

    object_ref& operator=(const object_ref& right) noexcept
    {
      object_ref(right).swap(*this);
      return (*this);
    }
    template <typename V>
    object_ref& operator=(const object_ref<V>& right) noexcept
    {
      object_ref(right).swap(*this);
      return (*this);
    }

    void swap(object_ref& right) noexcept { std::swap(value_, right.value_); }

    ~object_ref() noexcept 
    {
      if (value_) {
        value_->Release();
        value_ = nullptr;
      }
    }

    typename std::add_lvalue_reference<T>::type operator*() const
    {
      return (*get());
    }

    T* operator->() const noexcept
    {
      return std::pointer_traits<T*>::pointer_to(**this);
    }

    T* get() const noexcept { return value_; }

    template <typename V>
    V* get() const noexcept 
    {
      return reinterpret_cast<V*>(value_);
    }

    explicit operator bool() const noexcept { return value_ != nullptr; }

    T* release() noexcept
    {
      T* value = value_;
      value_ = nullptr;
      return value;
    }

    void reset() noexcept { object_ref().swap(*this); }

    void reset(T* value) noexcept { object_ref(value).swap(*this); }

    inline bool operator==(const T* right) noexcept { return value_ == right; }

  private:
    T* value_ = nullptr;
  };

  template <class _Ty>
  bool operator==(const object_ref<_Ty>& _Left, std::nullptr_t) noexcept 
  {
    return (_Left.get() == reinterpret_cast<_Ty*>(0));
  }

  template <class _Ty>
  bool operator==(std::nullptr_t, const object_ref<_Ty>& _Right) noexcept
  {
    return (reinterpret_cast<_Ty*>(0) == _Right.get());
  }

  template <class _Ty>
  bool operator!=(const object_ref<_Ty>& _Left, std::nullptr_t _Right) noexcept
  {
    return (!(_Left == _Right));
  }

  template <class _Ty>
  bool operator!=(std::nullptr_t _Left, const object_ref<_Ty>& _Right) noexcept
  {
    return (!(_Left == _Right));
  }

  template <typename T>
  object_ref<T> retain_object(T* ptr)
  {
    if (ptr) ptr->Retain();
    return object_ref<T>(ptr);
  }
}
