use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Undefined,
    Int(i32),
    Float(f32),
    String(Arc<str>),
    Vector([f32; 3]),
    Object(u64),
    Array(u64),
    Function(u32),
    Builtin(u32),
    LocalizedString(Arc<str>),
    Animation { tree: Arc<str>, name: Arc<str> },
    AnimationTree(Arc<str>),
}

impl Value {
    pub(crate) fn ensure_finite(&self) -> Result<(), String> {
        match self {
            Self::Float(n) if !n.is_finite() => Err("non-finite numeric value".into()),
            Self::Vector(v) if !v.iter().all(|n| n.is_finite()) => {
                Err("non-finite numeric value".into())
            }
            _ => Ok(()),
        }
    }

    pub fn string(text: &str) -> Self {
        Self::String(text.into())
    }

    pub fn level() -> Self {
        Self::Object(0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ArrayKey {
    Integer(i32),
    String(Arc<str>),
}
