#include "stdafx.h"

#include <algorithm>
#include <cstring>

#include "object_table.hpp"
#include "objects/object.hpp"
#include "syscall_errors.hpp"

using namespace uplift;

using Object = objects::Object;

ObjectTable::ObjectTable()
{
}

ObjectTable::~ObjectTable()
{
  Reset();
}

void ObjectTable::Reset()
{
  auto global_lock = global_critical_region_.Acquire();

  // Release all objects.
  for (uint32_t n = 0; n < table_capacity_; n++)
  {
    ObjectTableEntry& entry = table_[n];
    if (entry.object)
    {
      entry.object->Release();
    }
  }

  table_capacity_ = 0;
  last_free_entry_ = 0;
  free(table_);
  table_ = nullptr;
}

uint32_t ObjectTable::FindFreeSlot(uint32_t* out_slot)
{
  // Find a free slot.
  uint32_t slot = last_free_entry_;
  uint32_t scan_count = 0;
  while (scan_count < table_capacity_)
  {
    ObjectTableEntry& entry = table_[slot];
    if (!entry.object)
    {
      *out_slot = slot;
      return 0;
    }
    scan_count++;
    slot = (slot + 1) % table_capacity_;
    if (slot == 0)
    {
      // Never allow 0 handles.
      scan_count++;
      slot++;
    }
  }

  // Table out of slots, expand.
  uint32_t new_table_capacity = std::max(16 * 1024u, table_capacity_ * 2);
  if (!Resize(new_table_capacity))
  {
    return 12;
  }

  // Never allow 0 handles.
  slot = ++last_free_entry_;
  *out_slot = slot;

  return 0;
}

bool ObjectTable::Resize(uint32_t new_capacity)
{
  uint32_t new_size = new_capacity * sizeof(ObjectTableEntry);
  uint32_t old_size = table_capacity_ * sizeof(ObjectTableEntry);
  auto new_table = reinterpret_cast<ObjectTableEntry*>(realloc(table_, new_size));
  if (!new_table)
  {
    return false;
  }

  // Zero out new entries.
  if (new_size > old_size)
  {
    std::memset(reinterpret_cast<uint8_t*>(new_table) + old_size, 0, new_size - old_size);
  }

  last_free_entry_ = table_capacity_;
  table_capacity_ = new_capacity;
  table_ = new_table;

  return true;
}

uint32_t ObjectTable::AddHandle(Object* object, ObjectHandle* out_handle)
{
  uint32_t result = 0;

  uint32_t handle = 0;
  {
    auto global_lock = global_critical_region_.Acquire();

    // Find a free slot.
    uint32_t slot = 0;
    result = FindFreeSlot(&slot);

    // Stash.
    if (!result)
    {
      ObjectTableEntry& entry = table_[slot];
      entry.object = object;
      entry.handle_ref_count = 1;

      handle = slot << 2;
      object->handles().push_back(handle);

      // Retain so long as the object is in the table.
      object->Retain();
    }
  }

  if (!result)
  {
    if (out_handle)
    {
      *out_handle = handle;
    }
  }

  return result;
}

uint32_t ObjectTable::DuplicateHandle(ObjectHandle handle, ObjectHandle* out_handle)
{
  uint32_t result = 0;
  handle = TranslateHandle(handle);

  Object* object = LookupObject(handle, false);
  if (object)
  {
    result = AddHandle(object, out_handle);
    object->Release();  // Release the ref that LookupObject took
  }
  else
  {
    result = 9;
  }

  return result;
}

uint32_t ObjectTable::RetainHandle(ObjectHandle handle)
{
  auto global_lock = global_critical_region_.Acquire();

  ObjectTableEntry* entry = LookupTable(handle);
  if (!entry)
  {
    return 9;
  }

  entry->handle_ref_count++;
  return 0;
}

uint32_t ObjectTable::ReleaseHandle(ObjectHandle handle)
{
  auto global_lock = global_critical_region_.Acquire();

  ObjectTableEntry* entry = LookupTable(handle);
  if (!entry)
  {
    return 9;
  }

  if (--entry->handle_ref_count == 0)
  {
    // No more references. Remove it from the table.
    return RemoveHandle(handle);
  }

  return 0;
}

uint32_t ObjectTable::RemoveHandle(ObjectHandle handle)
{
  uint32_t result = 0;

  handle = TranslateHandle(handle);
  if (!handle)
  {
    return 9;
  }

  ObjectTableEntry* entry = LookupTable(handle);
  if (!entry)
  {
    return 9;
  }

  auto global_lock = global_critical_region_.Acquire();
  if (entry->object)
  {
    auto object = entry->object;
    entry->object = nullptr;
    entry->handle_ref_count = 0;

    // Walk the object's handles and remove this one.
    auto handle_entry = std::find(object->handles().begin(), object->handles().end(), handle);
    if (handle_entry != object->handles().end())
    {
      object->handles().erase(handle_entry);
    }

    // Release now that the object has been removed from the table.
    object->Release();
  }

  return 0;
}

std::vector<object_ref<Object>> ObjectTable::GetAllObjects()
{
  auto lock = global_critical_region_.Acquire();
  std::vector<object_ref<Object>> results;

  for (uint32_t slot = 0; slot < table_capacity_; slot++)
  {
    auto& entry = table_[slot];
    if (entry.object && std::find(results.begin(), results.end(), entry.object) == results.end())
    {
      entry.object->Retain();
      results.push_back(object_ref<Object>(entry.object));
    }
  }

  return results;
}

void ObjectTable::PurgeAllObjects()
{
  auto lock = global_critical_region_.Acquire();
  for (uint32_t slot = 0; slot < table_capacity_; slot++)
  {
    auto& entry = table_[slot];
    if (entry.object)
    {
      entry.handle_ref_count = 0;
      entry.object->Release();
      entry.object = nullptr;
    }
  }
}

ObjectTable::ObjectTableEntry* ObjectTable::LookupTable(ObjectHandle handle)
{
  handle = TranslateHandle(handle);
  if (!handle)
  {
    return nullptr;
  }

  auto global_lock = global_critical_region_.Acquire();

  // Lower 2 bits are ignored.
  uint32_t slot = handle >> 2;
  if (slot <= table_capacity_)
  {
    return &table_[slot];
  }

  return nullptr;
}

// Generic lookup
template <>
object_ref<Object> ObjectTable::LookupObject<Object>(ObjectHandle handle)
{
  auto object = ObjectTable::LookupObject(handle, false);
  auto result = object_ref<Object>(reinterpret_cast<Object*>(object));
  return result;
}

Object* ObjectTable::LookupObject(ObjectHandle handle, bool already_locked)
{
  handle = TranslateHandle(handle);
  if (!handle)
  {
    return nullptr;
  }

  Object* object = nullptr;
  if (!already_locked)
  {
    global_critical_region_.mutex().lock();
  }

  // Lower 2 bits are ignored.
  uint32_t slot = handle >> 2;

  // Verify slot.
  if (slot < table_capacity_)
  {
    ObjectTableEntry& entry = table_[slot];
    if (entry.object)
    {
      object = entry.object;
    }
  }

  // Retain the object pointer.
  if (object)
  {
    object->Retain();
  }

  if (!already_locked) 
  {
    global_critical_region_.mutex().unlock();
  }

  return object;
}

void ObjectTable::GetObjectsByType(Object::Type type, std::vector<object_ref<Object>>* results)
{
  auto global_lock = global_critical_region_.Acquire();
  for (uint32_t slot = 0; slot < table_capacity_; ++slot)
  {
    auto& entry = table_[slot];
    if (entry.object) {
      if (entry.object->type() == type) 
      {
        entry.object->Retain();
        results->push_back(object_ref<Object>(entry.object));
      }
    }
  }
}

ObjectHandle ObjectTable::TranslateHandle(ObjectHandle handle)
{
  return handle;
}

uint32_t ObjectTable::AddNameMapping(const std::string& name, ObjectHandle handle)
{
  // Names are case-insensitive.
  std::string lower_name = name;
  std::transform(lower_name.begin(), lower_name.end(), lower_name.begin(), tolower);

  auto global_lock = global_critical_region_.Acquire();
  if (name_table_.count(lower_name))
  {
    return 22;
  }
  name_table_.insert({ lower_name, handle });
  return 0;
}

void ObjectTable::RemoveNameMapping(const std::string& name)
{
  // Names are case-insensitive.
  std::string lower_name = name;
  std::transform(lower_name.begin(), lower_name.end(), lower_name.begin(), tolower);

  auto global_lock = global_critical_region_.Acquire();
  auto it = name_table_.find(lower_name);
  if (it != name_table_.end())
  {
    name_table_.erase(it);
  }
}

bool ObjectTable::GetObjectByName(const std::string& name, ObjectHandle* out_handle)
{
  if (!out_handle)
  {
    return false;
  }

  // Names are case-insensitive.
  std::string lower_name = name;
  std::transform(lower_name.begin(), lower_name.end(), lower_name.begin(), tolower);

  auto global_lock = global_critical_region_.Acquire();
  auto it = name_table_.find(lower_name);
  if (it == name_table_.end()) 
  {
    *out_handle = (ObjectHandle)-1;
    return false;
  }
  *out_handle = it->second;

  // We need to ref the handle. I think.
  auto obj = LookupObject(it->second, true);
  if (obj) 
  {
    obj->RetainHandle();
    obj->Release();
  }

  return true;
}
