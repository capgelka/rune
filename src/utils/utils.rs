//! Utilities and other miscellaneous functions for `RuneContext`

use r2pipe::r2::R2;
use r2api::structs::LRegInfo;
use r2api::api_trait::R2Api;

use context::rune_ctx::RuneContext;
use context::context::{ContextAPI};

use memory::memory::Memory;
use memory::qword_mem::QWordMemory;

use regstore::regstore::RegStore;
use regstore::regfile::RuneRegFile;

use libsmt::backends::smtlib2::SMTLib2;
use libsmt::logics::qf_abv;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ValType {
    Concrete(usize),
    Symbolic,
    Break,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Key {
    Mem(usize),
    Reg(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SAssignment {
    pub lvalue: Key,
    pub rvalue: ValType,
}

/// Hex/Decimal to Memory address, any other string maps to Registers
///
/// Useful when input strings is to be interpretted either as a Memory Address or a register name.
pub fn to_key<T: AsRef<str>>(s: T) -> Key {
    let v = s.as_ref();
    if v.len() > 2 && &v[0..2] == "0x" {
        Key::Mem(usize::from_str_radix(&v[2..], 16).expect("Invalid number!"))
    } else if v.chars().nth(0).unwrap().is_digit(10) {
        Key::Mem(usize::from_str_radix(v, 10).expect("Invalid number!"))
    } else {
        Key::Reg(v.to_owned())
    }
}

pub fn to_valtype<T: AsRef<str>>(s: T) -> Option<ValType> {
    let v = s.as_ref();

    if v == "SYM" {
        Some(ValType::Symbolic)
    } else if let Some(val) = convert_to_u64(v) {
        Some(ValType::Concrete(val as usize))
    } else {
        None
    }
}

pub fn to_assignment<T: AsRef<str>>(s: T) -> Option<SAssignment> {
    let v = s.as_ref();
    let ops: Vec<&str> = v.split('=').collect();

    let lvalue: Key = to_key(ops[0].trim());
    if let Some(rvalue) = to_valtype(ops[1].trim()) {
        Some(SAssignment {
                lvalue: lvalue,
                rvalue: rvalue,
            })
    } else {
        None
    }
}

pub fn convert_to_u64<T: AsRef<str>>(s: T) -> Option<u64> {
    let v = s.as_ref();
    if v.len() > 2 && &v[0..2] == "0x" {
        if let Ok(val) = usize::from_str_radix(&v[2..], 16) {
            Some(val as u64)
        } else {
            None
        }
    } else if v.chars().nth(0).unwrap().is_digit(10) {
        if let Ok(val) = usize::from_str_radix(v, 10) {
            Some(val as u64)
        } else {
            None
        }
    } else {
        None
    }
}

pub fn new_ctx(ip: Option<u64>,
               syms: &Option<Vec<Key>>,
               consts: &Option<Vec<(Key, u64)>>,
               mut r2: &mut R2)
               -> RuneContext<QWordMemory, RuneRegFile> {

    // TODO: Use entire arch information for creating suitable context later.

    let mut lreginfo = r2.reg_info().unwrap();
    let rregfile = RuneRegFile::new(&mut lreginfo);

    let bin = r2.bin_info().unwrap().bin.unwrap();
    let bits = bin.bits.unwrap();
    let endian = bin.endian.unwrap();
    let mut rmem = QWordMemory::new(bits, endian);

    let mut smt = SMTLib2::new(Some(qf_abv::QF_ABV));
    rmem.init_memory(&mut smt);

    let mut ctx = RuneContext::new(ip, rmem, rregfile, smt);

    if let Some(ref sym_vars) = *syms {
        for var in sym_vars {
            let  _ = match *var {
                Key::Mem(addr) => ctx.set_mem_as_sym(addr as u64, 64),
                Key::Reg(ref reg) => ctx.set_reg_as_sym(reg),
            };
        }
    }

    if let Some(ref const_var) = *consts {
        for &(ref k, v) in const_var.iter() {
            let _ = match *k {
                Key::Mem(addr) => ctx.set_mem_as_const(addr as u64, v, 64),
                Key::Reg(ref reg) => ctx.set_reg_as_const(reg, v),
            };
        }
    }

    for register in &lreginfo.reg_info {
        ctx.set_reg_as_const(register.name.clone(), 0);
    }

    ctx
}