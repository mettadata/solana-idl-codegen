use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Idl {
    pub version: String,
    pub name: String,
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
pub struct Instruction {
    pub name: String,
    pub accounts: Vec<AccountArg>,
    pub args: Vec<Field>,
    #[serde(default)]
    pub discriminator: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountArg {
    pub name: String,
    #[serde(rename = "isMut")]
    pub is_mut: bool,
    #[serde(rename = "isSigner")]
    pub is_signer: bool,
    #[serde(default)]
    pub docs: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: AccountType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountType {
    pub kind: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDef {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: TypeDefType,
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
    Array { array: [Box<IdlType>; 2] },
    Defined { defined: String },
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
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constant {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: IdlType,
    pub value: String,
}
