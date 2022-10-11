#pragma once

#include <string>
#include <unordered_map>
#include <vector>

#include <xenia/base/mutex.h>

#include "objects/object.hpp"

namespace uplift
{
  class ObjectTable
  {
  public:
    ObjectTable();
    ~ObjectTable();

    void Reset();

    uint32_t AddHandle(objects::Object* object, ObjectHandle* out_handle);
    uint32_t DuplicateHandle(ObjectHandle handle, ObjectHandle* out_handle);
    uint32_t RetainHandle(ObjectHandle handle);
    uint32_t ReleaseHandle(ObjectHandle handle);
    uint32_t RemoveHandle(ObjectHandle handle);

    object_ref<objects::Object> LookupObject(ObjectHandle handle)
    {
      auto object = LookupObject(handle, false);
      return object_ref<objects::Object>(object);
    }

    template <typename T>
    object_ref<T> LookupObject(ObjectHandle handle)
    {
      auto object = LookupObject(handle, false);
      if (object)
      {
        assert_true(object->type() == T::ObjectType);
      }
      return object_ref<T>(reinterpret_cast<T*>(object));
    }

    uint32_t AddNameMapping(const std::string& name, ObjectHandle handle);
    void RemoveNameMapping(const std::string& name);
    bool GetObjectByName(const std::string& name, ObjectHandle* out_handle);
    
    template <typename T>
    std::vector<object_ref<T>> GetObjectsByType(objects::Object::Type type)
    {
      std::vector<object_ref<T>> results;
      GetObjectsByType(type, reinterpret_cast<std::vector<object_ref<objects::Object>>*>(&results));
      return results;
    }

    template <typename T>
    std::vector<object_ref<T>> GetObjectsByType()
    {
      std::vector<object_ref<T>> results;
      GetObjectsByType(T::ObjectType, reinterpret_cast<std::vector<object_ref<objects::Object>>*>(&results));
      return results;
    }

    std::vector<object_ref<objects::Object>> GetAllObjects();
    void PurgeAllObjects();

  private:
    typedef struct
    {
      int handle_ref_count = 0;
      objects::Object* object = nullptr;
    }
    ObjectTableEntry;

    ObjectTableEntry* LookupTable(ObjectHandle handle);
    objects::Object* LookupObject(ObjectHandle handle, bool already_locked);
    void GetObjectsByType(objects::Object::Type type, std::vector<object_ref<objects::Object>>* results);

    ObjectHandle TranslateHandle(ObjectHandle handle);
    uint32_t FindFreeSlot(uint32_t* out_slot);
    bool Resize(uint32_t new_capacity);

    xe::global_critical_region global_critical_region_;
    uint32_t table_capacity_ = 0;
    ObjectTableEntry* table_ = nullptr;
    uint32_t last_free_entry_ = 0;
    std::unordered_map<std::string, ObjectHandle> name_table_;
  };

  // Generic lookup
  template <>
  object_ref<objects::Object> ObjectTable::LookupObject<objects::Object>(ObjectHandle handle);
}
