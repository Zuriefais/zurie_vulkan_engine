use anyhow::Ok;
use wasmtime::{Caller, Extern};
use zurie_types::bitcode::{self, Decode};

pub fn get_string_by_ptr(mut caller: Caller<'_, ()>, ptr: u32, len: u32) -> anyhow::Result<String> {
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => anyhow::bail!("failed to find host memory"),
    };

    let data = mem
        .data(&caller)
        .get(ptr as usize..)
        .and_then(|arr| arr.get(..len as usize))
        .unwrap();
    Ok(std::str::from_utf8(data)?.to_string())
}

pub fn get_obj_by_ptr<T: for<'a> Decode<'a>>(
    mut caller: Caller<'_, ()>,
    ptr: u32,
    len: u32,
) -> anyhow::Result<T> {
    let mem = match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => mem,
        _ => anyhow::bail!("failed to find host memory"),
    };
    let data = mem
        .data(&caller)
        .get(ptr as usize..)
        .and_then(|arr| arr.get(..len as usize))
        .unwrap();
    let obj = bitcode::decode(data)?;
    Ok(obj)
}
