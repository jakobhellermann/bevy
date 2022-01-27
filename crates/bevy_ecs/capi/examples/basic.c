#include <stdio.h>

#include "../bevy_ecs.h"

typedef struct {
    float x, y;
} CComponent;

void setup(World* world) {
    bevy_ecs_register_component(world, "CComponent", Table, false, sizeof(CComponent), __alignof__(CComponent));
}