use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Idl {
    #[serde(default)]
    pub address: Option<String>,
    pub metadata: Metadata,
    #[serde(default)]
    pub instructions: Vec<Instruction>,
    #[serde(default)]
    pub accounts: Option<Vec<Account>>,
    #[serde(default)]
    pub types: Option<Vec<TypeDef>>,
    #[serde(default)]
    pub errors: Option<Vec<Error>>,
    #[serde(default)]
    pub events: Option<Vec<Event>>,
    #[serde(default)]
    pub constants: Option<Vec<Constant>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub spec: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instruction {
    pub name: String,
    #[serde(default)]
    pub docs: Option<Vec<String>>,
    #[serde(default)]
    pub discriminator: Option<Vec<u8>>,
    pub accounts: Vec<AccountArg>,
    pub args: Vec<Arg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountArg {
    pub name: String,
    #[serde(default)]
    pub docs: Option<Vec<String>>,
    #[serde(default)]
    pub signer: bool,
    #[serde(default)]
    pub writable: bool,
    #[serde(default)]
    pub pda: Option<Pda>,
    #[serde(default)]
    pub address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pda {
    pub seeds: Vec<Seed>,
    #[serde(default)]
    pub program: Option<Program>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Seed {
    #[serde(rename = "const")]
    Const { value: Vec<u8> },
    #[serde(rename = "arg")]
    Arg { path: String },
    #[serde(rename = "account")]
    Account { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Program {
    #[serde(rename = "const")]
    Const { value: Vec<u8> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arg {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub name: String,
    #[serde(default)]
    pub discriminator: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDef {
    pub name: String,
    #[serde(default)]
    pub docs: Option<Vec<String>>,
    #[serde(rename = "type")]
    pub ty: TypeDefType,
    #[serde(default)]
    pub serialization: Option<String>,
    #[serde(default)]
    pub repr: Option<Repr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repr {
    pub kind: String,
    #[serde(default)]
    pub packed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TypeDefType {
    #[serde(rename = "struct")]
    Struct { fields: Vec<Field> },
    #[serde(rename = "enum")]
    Enum { variants: Vec<EnumVariant> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: IdlType,
    #[serde(default)]
    pub docs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: String,
    #[serde(default)]
    pub fields: Option<EnumFields>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum EnumFields {
    Named(Vec<Field>),
    Tuple(Vec<IdlType>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IdlType {
    Simple(String),
    Vec { vec: Box<IdlType> },
    Option { option: Box<IdlType> },
    Array { array: ArrayType },
    Defined { defined: DefinedType },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArrayType {
    Tuple(#[serde(with = "array_tuple")] (Box<IdlType>, usize)),
}

mod array_tuple {
    use super::*;
    #[allow(unused_imports)]
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(t: &(Box<IdlType>, usize), serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(2))?;
        seq.serialize_element(&*t.0)?;
        seq.serialize_element(&t.1)?;
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<(Box<IdlType>, usize), D::Error>
    where
        D: Deserializer<'de>,
    {
        let arr: Vec<serde_json::Value> = Vec::deserialize(deserializer)?;
        if arr.len() != 2 {
            return Err(serde::de::Error::custom(
                "Array type must have exactly 2 elements",
            ));
        }
        let ty = IdlType::deserialize(&arr[0]).map_err(serde::de::Error::custom)?;
        let size = arr[1]
            .as_u64()
            .ok_or_else(|| serde::de::Error::custom("Array size must be a number"))?
            as usize;
        Ok((Box::new(ty), size))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinedType {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    pub code: u32,
    pub name: String,
    pub msg: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub name: String,
    #[serde(default)]
    pub discriminator: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constant {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: IdlType,
    pub value: String,
}
