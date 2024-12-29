use anyhow::Ok;
use log::info;
use wasmtime::{Caller, Extern, Memory, TypedFunc};
use zurie_types::{
    flexbuffers,
    serde::{Deserialize, Serialize},
};

pub fn get_string_by_ptr(
    caller: &mut Caller<'_, ()>,
    ptr: u32,
    len: u32,
) -> anyhow::Result<String> {
    let data = get_bytes_from_wasm(caller, ptr, len)?;
    Ok(std::str::from_utf8(&data)?.to_string())
}

pub fn get_obj_by_ptr<T: for<'a> Deserialize<'a>>(
    caller: &mut Caller<'_, ()>,
    ptr: u32,
    len: u32,
    reason: &str,
) -> anyhow::Result<T> {
    info!("trying get object by pointer {}", reason);
    let data = get_bytes_from_wasm(caller, ptr, len)?;
    let r = flexbuffers::Reader::get_root(data.as_slice()).unwrap();
    let obj = T::deserialize(r).unwrap();
    Ok(obj)
}

pub fn get_memory(caller: &mut Caller<'_, ()>) -> anyhow::Result<Memory> {
    match caller.get_export("memory") {
        Some(Extern::Memory(mem)) => Ok(mem),
        _ => anyhow::bail!("failed to find host memory"),
    }
}

pub fn get_bytes_from_wasm(
    caller: &mut Caller<'_, ()>,
    ptr: u32,
    len: u32,
) -> anyhow::Result<Vec<u8>> {
    let data = get_memory(caller)?
        .data(&caller)
        .get(ptr as usize..)
        .and_then(|arr| arr.get(..len as usize))
        .ok_or_else(|| anyhow::anyhow!("failed to read memory"))?;
    Ok(data.to_vec())
}

pub fn copy_to_memory(
    caller: &mut Caller<'_, ()>,
    bytes: &[u8],
    alloc_fn: TypedFunc<u32, u32>,
) -> anyhow::Result<()> {
    let memory = get_memory(caller)?;
    let guest_ptr_offset = alloc_fn.call(&mut *caller, bytes.len() as u32)? as usize;
    memory
        .data_mut(caller)
        .get_mut(guest_ptr_offset..)
        .and_then(|slice| slice.get_mut(..bytes.len()))
        .ok_or_else(|| anyhow::anyhow!("failed to write to memory"))?
        .copy_from_slice(bytes);
    Ok(())
}

pub fn obj_to_bytes(obj: impl Serialize) -> Vec<u8> {
    let mut serializer = flexbuffers::FlexbufferSerializer::new();
    obj.serialize(&mut serializer).unwrap();
    serializer.take_buffer()
}

pub fn copy_string_to_memory(
    caller: &mut Caller<'_, ()>,
    string: &String,
    alloc_fn: TypedFunc<u32, u32>,
) -> anyhow::Result<()> {
    copy_to_memory(caller, string.as_bytes(), alloc_fn)?;
    Ok(())
}

pub fn copy_obj_to_memory(
    caller: &mut Caller<'_, ()>,
    obj: impl Serialize,
    alloc_fn: TypedFunc<u32, u32>,
) -> anyhow::Result<()> {
    let bytes = obj_to_bytes(obj);
    copy_to_memory(caller, &bytes, alloc_fn)?;
    Ok(())
}
