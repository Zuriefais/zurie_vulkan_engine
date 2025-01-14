# <a id="zurie_mod"></a>World zurie-mod


 - Imports:
    - interface `zurie:engine/core@0.1.0`
    - type `event-handle`
    - interface `zurie:engine/events@0.1.0`
    - type `event-data`
    - interface `zurie:engine/audio@0.1.0`
    - interface `zurie:engine/camera@0.1.0`
    - interface `zurie:engine/ecs@0.1.0`
    - interface `zurie:engine/input@0.1.0`
 - Exports:
    - function `init`
    - function `update`
    - function `key-event`
    - function `scroll`
    - function `event`

## <a id="zurie_engine_core_0_1_0"></a>Import interface zurie:engine/core@0.1.0


----

### Types

#### <a id="vec2"></a>`record vec2`


##### Record Fields

- <a id="vec2.x"></a>`x`: `f32`
- <a id="vec2.y"></a>`y`: `f32`
#### <a id="color"></a>`record color`


##### Record Fields

- <a id="color.r"></a>`r`: `f32`
- <a id="color.g"></a>`g`: `f32`
- <a id="color.b"></a>`b`: `f32`
- <a id="color.a"></a>`a`: `f32`
#### <a id="entity_id"></a>`type entity-id`
`u64`
<p>
#### <a id="component_id"></a>`type component-id`
`u64`
<p>
#### <a id="event_handle"></a>`type event-handle`
`u64`
<p>
#### <a id="sprite_handle"></a>`type sprite-handle`
`u64`
<p>
#### <a id="sound_handle"></a>`type sound-handle`
`u64`
<p>
----

### Functions

#### <a id="info"></a>`info: func`


##### Params

- <a id="info.module_path"></a>`module-path`: `string`
- <a id="info.text"></a>`text`: `string`

#### <a id="warn"></a>`warn: func`


##### Params

- <a id="warn.module_path"></a>`module-path`: `string`
- <a id="warn.text"></a>`text`: `string`

#### <a id="error"></a>`error: func`


##### Params

- <a id="error.module_path"></a>`module-path`: `string`
- <a id="error.text"></a>`text`: `string`

#### <a id="debug"></a>`debug: func`


##### Params

- <a id="debug.module_path"></a>`module-path`: `string`
- <a id="debug.text"></a>`text`: `string`

#### <a id="trace"></a>`trace: func`


##### Params

- <a id="trace.module_path"></a>`module-path`: `string`
- <a id="trace.text"></a>`text`: `string`

## <a id="zurie_engine_events_0_1_0"></a>Import interface zurie:engine/events@0.1.0


----

### Types

#### <a id="event_handle"></a>`type event-handle`
[`event-handle`](#event_handle)
<p>
#### <a id="entity_id"></a>`type entity-id`
[`entity-id`](#entity_id)
<p>
#### <a id="component_id"></a>`type component-id`
[`component-id`](#component_id)
<p>
#### <a id="vec2"></a>`type vec2`
[`vec2`](#vec2)
<p>
#### <a id="color"></a>`type color`
[`color`](#color)
<p>
#### <a id="event_data"></a>`variant event-data`


##### Variant Cases

- <a id="event_data.none"></a>`none`
- <a id="event_data.str"></a>`str`: `string`
- <a id="event_data.vec2"></a>`vec2`: [`vec2`](#vec2)
- <a id="event_data.color"></a>`color`: [`color`](#color)
- <a id="event_data.raw"></a>`raw`: list<`u8`>
- <a id="event_data.i32"></a>`i32`: `s32`
- <a id="event_data.i64"></a>`i64`: `s64`
----

### Functions

#### <a id="subscribe_by_name"></a>`subscribe-by-name: func`


##### Params

- <a id="subscribe_by_name.name"></a>`name`: `string`

##### Return values

- <a id="subscribe_by_name.0"></a> [`event-handle`](#event_handle)

#### <a id="subscribe_by_handle"></a>`subscribe-by-handle: func`


##### Params

- <a id="subscribe_by_handle.handle"></a>`handle`: [`event-handle`](#event_handle)

#### <a id="emit"></a>`emit: func`


##### Params

- <a id="emit.handle"></a>`handle`: [`event-handle`](#event_handle)
- <a id="emit.data"></a>`data`: [`event-data`](#event_data)

## <a id="zurie_engine_audio_0_1_0"></a>Import interface zurie:engine/audio@0.1.0


----

### Types

#### <a id="sound_handle"></a>`type sound-handle`
[`sound-handle`](#sound_handle)
<p>
----

### Functions

#### <a id="load_sound"></a>`load-sound: func`


##### Params

- <a id="load_sound.path"></a>`path`: `string`

##### Return values

- <a id="load_sound.0"></a> [`sound-handle`](#sound_handle)

#### <a id="play_sound"></a>`play-sound: func`


##### Params

- <a id="play_sound.handle"></a>`handle`: [`sound-handle`](#sound_handle)

## <a id="zurie_engine_camera_0_1_0"></a>Import interface zurie:engine/camera@0.1.0


----

### Types

#### <a id="vec2"></a>`type vec2`
[`vec2`](#vec2)
<p>
#### <a id="camera"></a>`record camera`


##### Record Fields

- <a id="camera.position"></a>`position`: [`vec2`](#vec2)
- <a id="camera.zoom_factor"></a>`zoom-factor`: `f32`
----

### Functions

#### <a id="get_camera"></a>`get-camera: func`


##### Return values

- <a id="get_camera.0"></a> [`camera`](#camera)

#### <a id="set_camera"></a>`set-camera: func`


##### Params

- <a id="set_camera.camera"></a>`camera`: [`camera`](#camera)

#### <a id="set_zoom"></a>`set-zoom: func`


##### Params

- <a id="set_zoom.factor"></a>`factor`: `f32`

#### <a id="get_zoom"></a>`get-zoom: func`


##### Return values

- <a id="get_zoom.0"></a> `f32`

#### <a id="set_position"></a>`set-position: func`


##### Params

- <a id="set_position.position"></a>`position`: [`vec2`](#vec2)

#### <a id="get_position"></a>`get-position: func`


##### Return values

- <a id="get_position.0"></a> [`vec2`](#vec2)

## <a id="zurie_engine_ecs_0_1_0"></a>Import interface zurie:engine/ecs@0.1.0


----

### Types

#### <a id="entity_id"></a>`type entity-id`
[`entity-id`](#entity_id)
<p>
#### <a id="component_id"></a>`type component-id`
[`component-id`](#component_id)
<p>
#### <a id="vec2"></a>`type vec2`
[`vec2`](#vec2)
<p>
#### <a id="color"></a>`type color`
[`color`](#color)
<p>
#### <a id="component_data"></a>`variant component-data`


##### Variant Cases

- <a id="component_data.none"></a>`none`
- <a id="component_data.str"></a>`str`: `string`
- <a id="component_data.vec2"></a>`vec2`: [`vec2`](#vec2)
- <a id="component_data.color"></a>`color`: [`color`](#color)
- <a id="component_data.raw"></a>`raw`: list<`u8`>
- <a id="component_data.i32"></a>`i32`: `s32`
- <a id="component_data.i64"></a>`i64`: `s64`
- <a id="component_data.sprite"></a>`sprite`: `u64`
----

### Functions

#### <a id="spawn_entity"></a>`spawn-entity: func`


##### Return values

- <a id="spawn_entity.0"></a> [`entity-id`](#entity_id)

#### <a id="despawn_entity"></a>`despawn-entity: func`


##### Params

- <a id="despawn_entity.entity"></a>`entity`: [`entity-id`](#entity_id)

#### <a id="register_component"></a>`register-component: func`


##### Params

- <a id="register_component.name"></a>`name`: `string`

##### Return values

- <a id="register_component.0"></a> [`component-id`](#component_id)

#### <a id="set_component"></a>`set-component: func`


##### Params

- <a id="set_component.entity"></a>`entity`: [`entity-id`](#entity_id)
- <a id="set_component.component"></a>`component`: [`component-id`](#component_id)
- <a id="set_component.data"></a>`data`: [`component-data`](#component_data)

#### <a id="get_component"></a>`get-component: func`


##### Params

- <a id="get_component.entity"></a>`entity`: [`entity-id`](#entity_id)
- <a id="get_component.component"></a>`component`: [`component-id`](#component_id)

##### Return values

- <a id="get_component.0"></a> option<[`component-data`](#component_data)>

#### <a id="get_entities_with_component"></a>`get-entities-with-component: func`


##### Params

- <a id="get_entities_with_component.component"></a>`component`: [`component-id`](#component_id)

##### Return values

- <a id="get_entities_with_component.0"></a> list<[`entity-id`](#entity_id)>

#### <a id="get_entities_with_components"></a>`get-entities-with-components: func`


##### Params

- <a id="get_entities_with_components.required"></a>`required`: list<[`component-id`](#component_id)>
- <a id="get_entities_with_components.optional"></a>`optional`: list<[`component-id`](#component_id)>

##### Return values

- <a id="get_entities_with_components.0"></a> list<[`entity-id`](#entity_id)>

## <a id="zurie_engine_input_0_1_0"></a>Import interface zurie:engine/input@0.1.0


----

### Types

#### <a id="vec2"></a>`type vec2`
[`vec2`](#vec2)
<p>
----

### Functions

#### <a id="key_clicked"></a>`key-clicked: func`

Keyboard

##### Params

- <a id="key_clicked.key"></a>`key`: `u32`

##### Return values

- <a id="key_clicked.0"></a> `bool`

#### <a id="subscribe_to_key_event"></a>`subscribe-to-key-event: func`


##### Params

- <a id="subscribe_to_key_event.key"></a>`key`: `u32`

#### <a id="mouse_pos"></a>`mouse-pos: func`

Mouse

##### Return values

- <a id="mouse_pos.0"></a> [`vec2`](#vec2)

## Exported types from world `zurie-mod`

----

### Types

#### <a id="event_handle"></a>`type event-handle`
[`event-handle`](#event_handle)
<p>
#### <a id="event_data"></a>`type event-data`
[`event-data`](#event_data)
<p>
## Exported functions from world `zurie-mod`

#### <a id="init"></a>`init: func`


#### <a id="update"></a>`update: func`


#### <a id="key_event"></a>`key-event: func`


##### Params

- <a id="key_event.key_code"></a>`key-code`: `u32`

#### <a id="scroll"></a>`scroll: func`


##### Params

- <a id="scroll.amount"></a>`amount`: `f32`

#### <a id="event"></a>`event: func`


##### Params

- <a id="event.handle"></a>`handle`: [`event-handle`](#event_handle)
- <a id="event.data"></a>`data`: [`event-data`](#event_data)

