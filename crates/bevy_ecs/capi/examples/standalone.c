#include "../bevy_ecs.h"
#include <stdio.h>


typedef struct {
    float x, y;
} CComponent;

int main() {
    World* world = bevy_ecs_world_new();
    ComponentId id = bevy_ecs_register_component(world, "Component", Table, false, sizeof(CComponent), __alignof__(CComponent));
    ComponentId id_by_name = bevy_ecs_world_get_component_id_by_name(world, "Component");

    printf("Id: %ld, Id by name: %ld.\n", id._0, id_by_name._0);

    Entity entity = bevy_ecs_spawn_entity(world);

    const CComponent* value = bevy_ecs_get(world, entity, id);

    if (!value) {
        printf("component not set\n");
    }

    // bevy_ecs_debug_world(world);
    bevy_ecs_world_free(world);

    return 0;
}