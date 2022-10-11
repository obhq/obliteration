#include "stdafx.h"

#include "../runtime.hpp"
#include "../syscall_errors.hpp"
#include "../objects/event_flag.hpp"
#include "ipmi_client.hpp"

using namespace uplift;
using namespace uplift::ipmi;
using namespace uplift::syscall_errors;

IpmiClient::IpmiClient(Runtime* runtime)
  : Object(runtime, ObjectType)
  , arg1_(nullptr)
  , name_()
  , arg3_(nullptr)
  , event_flag_count_(0)
{
}

IpmiClient::~IpmiClient()
{
}

SCERR IpmiClient::Close()
{
  return SUCCESS;
}

SCERR IpmiClient::Initialize(void* arg1, const std::string& name, void* arg3)
{
  arg1_ = arg1;
  name_ = name;
  arg3_ = arg3;
  return SUCCESS;
}

SCERR IpmiClient::PrepareConnect(uint32_t event_flag_count)
{
  event_flag_count_ = event_flag_count;
  return SUCCESS;
}

SCERR IpmiClient::Connect(uint64_t* session_key, uint32_t* unknown, uint32_t* session_id, uint32_t* result)
{
  *session_key = 0xBEEFBEEFBEEFBEEFull;
  *unknown = 0xBEEF0;
  *session_id = 1;
  *result = 0;

  std::string prefix = name_.substr(0, 3);
  std::string name = name_;
  if (prefix != "sce" && prefix != "Sce")
  {
    prefix = "Sce";
  }
  else
  {
    name = name_.substr(3);
  }

  for (uint32_t i = 0; i < event_flag_count_ + 1; ++i)
  {
    char event_flag_name[32];
    std::snprintf(event_flag_name, sizeof(event_flag_name), "%.3s%.12s%05x%02x%01x%08x", prefix.c_str(), name.c_str(), *unknown, *session_id, i, static_cast<uint32_t>(*session_key));
    auto evf = object_ref<objects::EventFlag>(new objects::EventFlag(runtime_)).get();
    runtime_->object_table()->AddNameMapping(std::string(event_flag_name), evf->handle());
  }

  return SUCCESS;
}
