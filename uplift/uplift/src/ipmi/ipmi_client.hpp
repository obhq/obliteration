#pragma once

#include "../objects/object.hpp"

namespace uplift::ipmi
{
  class IpmiClient : public objects::Object
  {
  public:
    static const Object::Type ObjectType = Type::IpmiClient;

  public:
    IpmiClient(Runtime* runtime);
    virtual ~IpmiClient();

    SyscallError Close();

    SyscallError Initialize(void* arg1, const std::string& name, void* arg3);

    SyscallError PrepareConnect(uint32_t event_flag_count);
    SyscallError Connect(uint64_t* session_key, uint32_t* unknown, uint32_t* session_id, uint32_t* result);

  private:
    void* arg1_;
    std::string name_;
    void* arg3_;
    uint32_t event_flag_count_;
  };
}
