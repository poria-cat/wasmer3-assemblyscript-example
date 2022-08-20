use anyhow::anyhow;
use std::borrow::BorrowMut;
use std::os::unix::prelude::MetadataExt;
use std::sync::{Arc, Mutex};
use std::{error::Error, fs};
use wasmer::{
    imports, AsStoreMut, AsStoreRef, Function, FunctionEnv, FunctionEnvMut, Instance, Memory,
    MemoryView, Module, RuntimeError, Store, Value, WasmPtr,
};

#[derive(Debug)]
pub enum BindHelperError {
    Convert(String),
}

impl std::fmt::Display for BindHelperError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BindHelperError")
    }
}

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

    pub fn set_fn_new(&mut self, fn_new: Function) {
        if self.fn_new.is_some() {
            panic!("fn_new of a Env can only be set once!");
        }
        self.fn_new = Some(fn_new);
    }
}

fn get_str_ptr(ctx: &mut FunctionEnvMut<'_, Env>) -> anyhow::Result<()> {
    let new_str = String::from("hello, assemblyscript");
    let new_str_size: i32 = new_str.len().try_into().expect("can't convert to i32");
    let env = ctx.data().to_owned();

    let result = env
        .fn_new()
        .call(ctx, &[Value::I32(new_str_size << 1), Value::I32(1)])?;

    let string_ptr = match result.get(0) {
        Some(v) => match v.i32() {
            Some(i) => i,
            _ => {
                return Err(anyhow!("can't get new string pointer"));
            }
        },
        None => {
            return Err(anyhow!("can't get new string pointer"));
        }
    };

    println!("{:?}", string_ptr);
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("Hello, world!");
    let wasm_path = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/release.wasm");
    // println!("Results: {:?}", wasm_bytes);
    let wasm_bytes = fs::read(wasm_path)?;
    let mut store = Store::default();

    println!("Compiling module...");

    let module = Module::new(&store, wasm_bytes)?;

    let abort = |_: i32, _: i32, _: i32, _: i32| std::process::exit(-1);

    fn test_log(mut ctx: FunctionEnvMut<'_, Env>, string_ptr: i32) {
        let env = &ctx.data().to_owned();
        let view = env.memory_view(&ctx);

        let ptr: WasmPtr<u16> = WasmPtr::new(string_ptr as _);
        let offset = ptr.offset() / 4 - 1;
        // in assemblyscript, string's offset is -4
        // ptr / 4 - 1 if for u32, for u8, it need to be expanded 4 times
        let size = view.read_u8(offset as u64 * 4).expect("can't get size");
        // u8 -> u16, need / 2
        let values = ptr.slice(&view, size as u32 / 2).expect("can't get by ptr");
        let values_sliced = values.read_to_vec().expect("qaq");
        let result = String::from_utf16_lossy(values_sliced.as_slice());
        println!("{:#}", result);

        get_str_ptr(&mut ctx);
    }
    
    let env = FunctionEnv::new(&mut store, Env::new());
    let import_object = imports! {
            "env" => {
            "abort" => Function::new_typed(&mut store, abort)
        },
        "index" => {
            "log" => Function::new_typed_with_env(&mut store, &env , test_log)
        }
    };
    let instance = Instance::new(&mut store, &module, &import_object)?;
    let memory = instance.exports.get_memory("memory")?;
    let fn_new = instance.exports.get_function("__new")?;

    env.as_mut(&mut store).set_memory(memory.clone());
    env.as_mut(&mut store).set_fn_new(fn_new.clone());

    let log_func = instance.exports.get_function("testLog")?;
    log_func.call(&mut store, &[]).expect("call qaq");

    Ok(())
}
