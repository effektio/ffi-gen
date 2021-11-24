use crate::{Interface, Type};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Abi {
    Native(u8),
    Wasm(u8),
}

impl Abi {
    pub fn native() -> Self {
        #[cfg(target_pointer_width = "32")]
        return Abi::Native(32);
        #[cfg(target_pointer_width = "64")]
        return Abi::Native(64);
    }

    pub fn ptr_width(self) -> usize {
        match self {
            Self::Native(ptr_width) => ptr_width as _,
            Self::Wasm(ptr_width) => ptr_width as _,
        }
    }

    /// Returns the size and alignment of a primitive type.
    pub fn layout(self, ty: PrimType) -> (usize, usize) {
        let size = match ty {
            PrimType::U8 | PrimType::I8 | PrimType::Bool => 1,
            PrimType::U16 | PrimType::I16 => 2,
            PrimType::U32 | PrimType::I32 | PrimType::F32 => 4,
            PrimType::U64 | PrimType::I64 | PrimType::F64 => 8,
            PrimType::Usize | PrimType::Isize => self.ptr_width() / 4,
        };
        let size = match self {
            Self::Native(_) => size,
            Self::Wasm(_) => core::cmp::max(4, size),
        };
        (size, size)
    }
}

pub struct AbiObject {
    pub name: String,
    pub methods: Vec<AbiFunction>,
}

pub struct AbiFunction {
    pub is_static: bool,
    pub is_async: bool,
    pub object: Option<String>,
    pub name: String,
    pub args: Vec<(String, AbiType)>,
    pub ret: Option<AbiType>,
}

impl AbiFunction {
    pub fn fqn(&self) -> String {
        if let Some(object) = self.object.as_ref() {
            format!("{}_{}", object, &self.name)
        } else {
            self.name.clone()
        }
    }

    pub fn needs_self(&self) -> bool {
        self.object.is_some() && !self.is_static
    }

    pub fn self_type(&self) -> AbiType {
        AbiType::Ref(self.object.clone().unwrap())
    }
}

#[derive(Clone, Debug)]
pub enum AbiType {
    Prim(PrimType),
    RefStr,
    String,
    RefSlice(PrimType),
    Vec(PrimType),
    Box(String),
    Ref(String),
}

#[derive(Clone, Copy, Debug)]
pub enum PrimType {
    U8,
    U16,
    U32,
    U64,
    Usize,
    I8,
    I16,
    I32,
    I64,
    Isize,
    Bool,
    F32,
    F64,
}

impl Interface {
    pub fn objects(&self) -> Vec<AbiObject> {
        let mut objs = vec![];
        for object in &self.objects {
            let mut methods = vec![];
            for method in &object.methods {
                let func = AbiFunction {
                    is_static: method.is_static,
                    is_async: method.func.ty.is_async,
                    object: Some(object.ident.clone()),
                    name: method.func.ident.clone(),
                    args: method
                        .func
                        .ty
                        .args
                        .iter()
                        .map(|(n, ty)| (n.clone(), ty.to_type()))
                        .collect(),
                    ret: method.func.ty.ret.as_ref().map(|ty| ty.to_type()),
                };
                methods.push(func);
            }
            objs.push(AbiObject {
                name: object.ident.clone(),
                methods,
            });
        }
        objs
    }

    pub fn functions(&self) -> Vec<AbiFunction> {
        let mut funcs = vec![];
        for func in &self.functions {
            let func = AbiFunction {
                is_static: false,
                is_async: func.ty.is_async,
                object: None,
                name: func.ident.clone(),
                args: func
                    .ty
                    .args
                    .iter()
                    .map(|(n, ty)| (n.clone(), ty.to_type()))
                    .collect(),
                ret: func.ty.ret.as_ref().map(|ty| ty.to_type()),
            };
            funcs.push(func);
        }
        funcs
    }

    pub fn into_functions(self) -> Vec<AbiFunction> {
        let mut funcs = self.functions();
        for object in self.objects() {
            funcs.extend(object.methods);
        }
        funcs
    }
}

impl Type {
    pub fn to_type(&self) -> AbiType {
        match self {
            Self::U8 => AbiType::Prim(PrimType::U8),
            Self::U16 => AbiType::Prim(PrimType::U16),
            Self::U32 => AbiType::Prim(PrimType::U32),
            Self::U64 => AbiType::Prim(PrimType::U64),
            Self::Usize => AbiType::Prim(PrimType::Usize),
            Self::I8 => AbiType::Prim(PrimType::I8),
            Self::I16 => AbiType::Prim(PrimType::I16),
            Self::I32 => AbiType::Prim(PrimType::I32),
            Self::I64 => AbiType::Prim(PrimType::I64),
            Self::Isize => AbiType::Prim(PrimType::Isize),
            Self::Bool => AbiType::Prim(PrimType::Bool),
            Self::F32 => AbiType::Prim(PrimType::F32),
            Self::F64 => AbiType::Prim(PrimType::F64),
            Self::Ref(inner) => match &**inner {
                Self::String => AbiType::RefStr,
                Self::Slice(inner) => match inner.to_type() {
                    AbiType::Prim(ty) => AbiType::RefSlice(ty),
                    ty => unimplemented!("&{:?}", ty),
                },
                Self::Ident(ident) => AbiType::Ref(ident.clone()),
                ty => unimplemented!("&{:?}", ty),
            },
            Self::String => AbiType::String,
            Self::Vec(inner) => match inner.to_type() {
                AbiType::Prim(ty) => AbiType::Vec(ty),
                ty => unimplemented!("Vec<{:?}>", ty),
            },
            Self::Box(inner) => match &**inner {
                Self::Ident(ident) => AbiType::Box(ident.clone()),
                ty => unimplemented!("Box<{:?}>", ty),
            },
            ty => unimplemented!("{:?}", ty),
        }
    }
}
