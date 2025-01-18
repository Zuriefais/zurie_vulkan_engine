#include "zurie_mod.h"
#include <string.h> // Make sure to include string.h for strlen

void exports_zurie_mod_init()
{
    zurie_mod_string_t module_name = { "cpp_hell", strlen("cpp_hell") };
    zurie_mod_string_t message = { "Hello from hell", strlen("Hello from hell") };

    zurie_engine_core_info(&module_name, &message);
}

void exports_zurie_mod_update() {}

void exports_zurie_mod_event(zurie_mod_event_handle_t handle, zurie_mod_event_data_t *data) {}

void exports_zurie_mod_key_event(uint32_t key_code) {}

void exports_zurie_mod_scroll(float amount) {
    zurie_engine_camera_set_zoom(zurie_engine_camera_get_zoom()+amount);
}
