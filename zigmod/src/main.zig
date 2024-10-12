const std = @import("std");

extern fn info_sys([*]const u8, usize) void;

extern fn get_mod_name_callback([*]const u8, usize) void;

fn info(string: []const u8) void {
    const ptr: [*]const u8 = string.ptr;
    const len: usize = string.len;
    info_sys(ptr, len);
}

export fn init() void {
    const my_string: []const u8 = "Hello, Zig!";
    info(my_string);
}

export fn update() void {
    const my_string: []const u8 = "Hello, Zig! in Update loop";
    info(my_string);
}

export fn key_event(_: u32) void {}

export fn get_mod_name() void {
    const my_string: []const u8 = "Ziga";
    const ptr: [*]const u8 = my_string.ptr;
    const len: usize = my_string.len;
    get_mod_name_callback(ptr, len);
}
