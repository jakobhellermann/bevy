#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

typedef enum {
  Table,
  SparseSet,
} StorageType;

typedef struct World World;

typedef struct {
  uintptr_t _0;
} ComponentId;

typedef struct {
  uint32_t generation;
  uint32_t id;
} Entity;

World *bevy_ecs_world_new(void);

void bevy_ecs_debug_world(const World *world);

void bevy_ecs_world_free(World *world);

ComponentId bevy_ecs_register_component(World *world,
                                        const char *name,
                                        StorageType storage_type,
                                        bool is_send_and_sync,
                                        uintptr_t size,
                                        uintptr_t alignment);

Entity bevy_ecs_spawn_entity(World *world);

const void *bevy_ecs_get(World *world, Entity entity, ComponentId component_id);

void *bevy_ecs_get_mut(World *world, Entity entity, ComponentId component_id);

ComponentId bevy_ecs_world_get_component_id_by_name(const World *world, const char *name);
