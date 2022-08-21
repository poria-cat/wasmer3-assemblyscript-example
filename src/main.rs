use anyhow::anyhow;
use bytemuck;
use std::{error::Error, fs};
use wasmer::{
    imports, AsStoreRef, Function, FunctionEnv, FunctionEnvMut, Instance, Memory, MemoryView,
    Module, RuntimeError, Store, Value, WasmPtr,
};

#[derive(Clone, Default)]
pub struct Env {
    memory: Option<Memory>,
    pub fn_new: Option<Function>,
    pub fn_pin: Option<Function>,
    pub fn_unpin: Option<Function>,
}

impl Env {
    pub fn new() -> Self {
        Self {
            memory: None,
            fn_new: None,
            fn_pin: None,
            fn_unpin: None,
        }
    }

    /// Copy the lazy reference so that when it's initialized during the
    /// export phase, all the other references get a copy of it
    pub fn memory_clone(&self) -> Option<Memory> {
        self.memory.clone()
    }

    /// Set the memory of the WasiEnv (can only be done once)
    pub fn set_memory(&mut self, memory: Memory) {
        if self.memory.is_some() {
            panic!("Memory of a Env can only be set once!");
        }
        self.memory = Some(memory);
    }

    /// Providers safe access to the memory
    /// (it must be initialized before it can be used)
    pub fn memory_view<'a>(&'a self, store: &'a impl AsStoreRef) -> MemoryView<'a> {
        self.memory().view(store)
    }

    /// Get memory, that needs to have been set fist
    pub fn memory(&self) -> &Memory {
        self.memory.as_ref().expect("can't get memory")
    }

    pub fn fn_new(&self) -> &Function {
        self.fn_new.as_ref().expect("can't get function")
    }

    pub fn fn_pin(&self) -> &Function {
        self.fn_pin.as_ref().expect("can't get function")
    }

    pub fn set_fn_new(&mut self, fn_new: Function) {
        if self.fn_new.is_some() {
            panic!("fn_new of a Env can only be set once!");
        }
        self.fn_new = Some(fn_new);
    }

    pub fn set_fn_pin(&mut self, fn_pin: Function) {
        if self.fn_pin.is_some() {
            panic!("fn_pin of a Env can only be set once!");
        }
        self.fn_pin = Some(fn_pin);
    }
}

fn lift_string(ctx: &FunctionEnvMut<'_, Env>, string_ptr: i32) -> anyhow::Result<String> {
    let env = ctx.data();
    let view = env.memory_view(&ctx);

    let ptr: WasmPtr<u16> = WasmPtr::new(string_ptr as _);
    let offset = ptr.offset() / 4 - 1;
    // in assemblyscript, string's offset is -4
    // ptr / 4 - 1 if for u32, for u8, it need to be expanded 4 times
    let size = view.read_u8(offset as u64 * 4)?;
    // u8 -> u16, need / 2
    let values = ptr.slice(&view, size as u32 / 2)?;
    let values_sliced = values.read_to_vec().expect("qaq");
    let result = String::from_utf16_lossy(values_sliced.as_slice());

    Ok(result)
}

fn lower_string(ctx: &mut FunctionEnvMut<'_, Env>, value: &String) -> anyhow::Result<u32> {
    let env = ctx.data().to_owned();

    let str_size: i32 = value.len().try_into()?;
    let result = env
        .fn_new()
        .call(ctx, &[Value::I32(str_size << 1), Value::I32(1)])?;

    let ptr = result
        .get(0)
        .ok_or(anyhow!("can't get new string pointer"))?
        .i32()
        .ok_or(anyhow!("can't get new string pointer"))?;

    let utf16: Vec<u16> = value.encode_utf16().collect();
    let utf16_to_u8: &[u8] = bytemuck::try_cast_slice(&utf16.as_slice()).expect("qaq");

    let view = env.memory_view(&ctx);

    view.write(ptr as u64, utf16_to_u8)?;
    env.fn_pin().call(ctx, &[Value::I32(ptr)])?;

    Ok(ptr as u32)
}

fn test_log(ctx: FunctionEnvMut<'_, Env>, string_ptr: i32) -> Result<(), RuntimeError> {
    let result = lift_string(&ctx, string_ptr).map_err(|e| RuntimeError::new(e.to_string()))?;

    println!("{:#}", result);

    Ok(())
}

fn get_string(mut ctx: FunctionEnvMut<'_, Env>) -> Result<u32, RuntimeError> {
    let ptr = lower_string(&mut ctx, &"Hello AssemblyScript!".to_string())
        .map_err(|e| RuntimeError::new(e.to_string()))?;

    Ok(ptr)
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("Hello, world!");
    let wasm_path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/release.wasm");

    let wasm_bytes = fs::read(wasm_path)?;
    let mut store = Store::default();

    println!("Compiling module...");

    let module = Module::new(&store, wasm_bytes)?;

    let abort = |_: i32, _: i32, _: i32, _: i32| std::process::exit(-1);

    let env = FunctionEnv::new(&mut store, Env::new());
    let import_object = imports! {
            "env" => {
            "abort" => Function::new_typed(&mut store, abort)
        },
        "index" => {
            "log" => Function::new_typed_with_env(&mut store, &env , test_log),
            "getString" => Function::new_typed_with_env(&mut store, &env , get_string)
        }
    };

    let instance = Instance::new(&mut store, &module, &import_object)?;
    let memory = instance.exports.get_memory("memory")?;

    env.as_mut(&mut store).set_memory(memory.clone());

    let fn_pin = instance.exports.get_function("__pin")?;
    env.as_mut(&mut store).set_fn_pin(fn_pin.clone());

    let fn_new = instance.exports.get_function("__new")?;
    env.as_mut(&mut store).set_fn_new(fn_new.clone());

    let log_func = instance.exports.get_function("testLog")?;
    log_func.call(&mut store, &[])?;

    let test_get_string_func = instance.exports.get_function("testGetString")?;
    test_get_string_func.call(&mut store, &[])?;

    Ok(())
}
