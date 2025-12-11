use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Idl {
    #[serde(default)]
    pub address: Option<String>,
    // Old format IDLs have version and name at top level, new format has metadata
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub metadata: Option<Metadata>,
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

impl Idl {
    pub fn get_name(&self) -> &str {
        if let Some(ref metadata) = self.metadata {
            if let Some(ref name) = metadata.name {
                return name;
            }
        }
        if let Some(ref name) = self.name {
            name
        } else {
            "unknown"
        }
    }

    pub fn get_version(&self) -> &str {
        if let Some(ref metadata) = self.metadata {
            if let Some(ref version) = metadata.version {
                return version;
            }
        }
        if let Some(ref version) = self.version {
            version
        } else {
            "0.0.0"
        }
    }

    pub fn get_address(&self) -> Option<&str> {
        // Check top-level address first
        if let Some(ref address) = self.address {
            return Some(address);
        }
        // Then check metadata.address
        if let Some(ref metadata) = self.metadata {
            if let Some(ref address) = metadata.address {
                return Some(address);
            }
        }
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub spec: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
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
    // Support both old and new format
    #[serde(default, alias = "isSigner")]
    pub signer: bool,
    #[serde(default, alias = "isMut")]
    pub writable: bool,
    #[serde(default)]
    pub pda: Option<Pda>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub optional: Option<bool>,
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
    #[serde(rename = "account")]
    Account { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arg {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: IdlType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub name: String,
    #[serde(default)]
    pub discriminator: Option<Vec<u8>>,
    #[serde(default)]
    pub docs: Option<Vec<String>>,
    #[serde(rename = "type", default)]
    pub ty: Option<TypeDefType>,
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
    Struct { fields: StructFields },
    #[serde(rename = "enum")]
    Enum { variants: Vec<EnumVariant> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StructFields {
    Named(Vec<Field>),
    Tuple(Vec<IdlType>),
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
    Defined { defined: DefinedTypeOrString },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DefinedTypeOrString {
    String(String),
    Nested(DefinedType),
}

impl DefinedTypeOrString {
    pub fn name(&self) -> &str {
        match self {
            DefinedTypeOrString::String(s) => s,
            DefinedTypeOrString::Nested(d) => &d.name,
        }
    }
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
    #[serde(default)]
    pub fields: Option<Vec<EventField>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventField {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: IdlType,
    #[serde(default)]
    pub index: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constant {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: IdlType,
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_idl_get_name_from_metadata() {
        let idl = Idl {
            address: None,
            version: None,
            name: None,
            metadata: Some(Metadata {
                name: Some("program_from_metadata".to_string()),
                version: None,
                spec: None,
                description: None,
                address: None,
            }),
            instructions: vec![],
            accounts: None,
            types: None,
            errors: None,
            events: None,
            constants: None,
        };

        assert_eq!(idl.get_name(), "program_from_metadata");
    }

    #[test]
    fn test_idl_get_name_from_name_field() {
        let idl = Idl {
            address: None,
            version: None,
            name: Some("program_from_name".to_string()),
            metadata: None,
            instructions: vec![],
            accounts: None,
            types: None,
            errors: None,
            events: None,
            constants: None,
        };

        assert_eq!(idl.get_name(), "program_from_name");
    }

    #[test]
    fn test_idl_get_name_default() {
        let idl = Idl {
            address: None,
            version: None,
            name: None,
            metadata: None,
            instructions: vec![],
            accounts: None,
            types: None,
            errors: None,
            events: None,
            constants: None,
        };

        assert_eq!(idl.get_name(), "unknown");
    }

    #[test]
    fn test_idl_get_version_from_metadata() {
        let idl = Idl {
            address: None,
            version: None,
            name: None,
            metadata: Some(Metadata {
                name: None,
                version: Some("2.0.0".to_string()),
                spec: None,
                description: None,
                address: None,
            }),
            instructions: vec![],
            accounts: None,
            types: None,
            errors: None,
            events: None,
            constants: None,
        };

        assert_eq!(idl.get_version(), "2.0.0");
    }

    #[test]
    fn test_idl_get_version_from_version_field() {
        let idl = Idl {
            address: None,
            version: Some("1.0.0".to_string()),
            name: None,
            metadata: None,
            instructions: vec![],
            accounts: None,
            types: None,
            errors: None,
            events: None,
            constants: None,
        };

        assert_eq!(idl.get_version(), "1.0.0");
    }

    #[test]
    fn test_idl_get_version_default() {
        let idl = Idl {
            address: None,
            version: None,
            name: None,
            metadata: None,
            instructions: vec![],
            accounts: None,
            types: None,
            errors: None,
            events: None,
            constants: None,
        };

        assert_eq!(idl.get_version(), "0.0.0");
    }

    #[test]
    fn test_defined_type_or_string_name_string() {
        let defined = DefinedTypeOrString::String("MyType".to_string());
        assert_eq!(defined.name(), "MyType");
    }

    #[test]
    fn test_defined_type_or_string_name_nested() {
        let defined = DefinedTypeOrString::Nested(DefinedType {
            name: "NestedType".to_string(),
        });
        assert_eq!(defined.name(), "NestedType");
    }

    #[test]
    fn test_deserialize_simple_idl_type() {
        let json = r#""u64""#;
        let result: IdlType = serde_json::from_str(json).unwrap();
        match result {
            IdlType::Simple(s) => assert_eq!(s, "u64"),
            _ => panic!("Expected Simple variant"),
        }
    }

    #[test]
    fn test_deserialize_vec_idl_type() {
        let json = r#"{"vec":"u64"}"#;
        let result: IdlType = serde_json::from_str(json).unwrap();
        match result {
            IdlType::Vec { vec } => match *vec {
                IdlType::Simple(s) => assert_eq!(s, "u64"),
                _ => panic!("Expected Simple variant inside Vec"),
            },
            _ => panic!("Expected Vec variant"),
        }
    }

    #[test]
    fn test_deserialize_option_idl_type() {
        let json = r#"{"option":"string"}"#;
        let result: IdlType = serde_json::from_str(json).unwrap();
        match result {
            IdlType::Option { option } => match *option {
                IdlType::Simple(s) => assert_eq!(s, "string"),
                _ => panic!("Expected Simple variant inside Option"),
            },
            _ => panic!("Expected Option variant"),
        }
    }

    #[test]
    fn test_deserialize_array_idl_type() {
        let json = r#"{"array":["u8",32]}"#;
        let result: IdlType = serde_json::from_str(json).unwrap();
        match result {
            IdlType::Array { array } => match array {
                ArrayType::Tuple((inner, size)) => {
                    match *inner {
                        IdlType::Simple(s) => assert_eq!(s, "u8"),
                        _ => panic!("Expected Simple variant inside Array"),
                    }
                    assert_eq!(size, 32);
                }
            },
            _ => panic!("Expected Array variant"),
        }
    }

    #[test]
    fn test_deserialize_defined_string_idl_type() {
        let json = r#"{"defined":"MyStruct"}"#;
        let result: IdlType = serde_json::from_str(json).unwrap();
        match result {
            IdlType::Defined { defined } => {
                assert_eq!(defined.name(), "MyStruct");
            }
            _ => panic!("Expected Defined variant"),
        }
    }

    #[test]
    fn test_deserialize_defined_nested_idl_type() {
        let json = r#"{"defined":{"name":"MyStruct"}}"#;
        let result: IdlType = serde_json::from_str(json).unwrap();
        match result {
            IdlType::Defined { defined } => {
                assert_eq!(defined.name(), "MyStruct");
            }
            _ => panic!("Expected Defined variant"),
        }
    }

    #[test]
    fn test_deserialize_enum_named_fields() {
        let json = r#"[{"name":"field1","type":"u64"},{"name":"field2","type":"string"}]"#;
        let result: EnumFields = serde_json::from_str(json).unwrap();
        match result {
            EnumFields::Named(fields) => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].name, "field1");
                assert_eq!(fields[1].name, "field2");
            }
            _ => panic!("Expected Named variant"),
        }
    }

    #[test]
    fn test_deserialize_enum_tuple_fields() {
        let json = r#"["u64","string"]"#;
        let result: EnumFields = serde_json::from_str(json).unwrap();
        match result {
            EnumFields::Tuple(types) => {
                assert_eq!(types.len(), 2);
            }
            _ => panic!("Expected Tuple variant"),
        }
    }

    #[test]
    fn test_deserialize_struct_type() {
        let json = r#"{"kind":"struct","fields":[{"name":"value","type":"u64"}]}"#;
        let result: TypeDefType = serde_json::from_str(json).unwrap();
        match result {
            TypeDefType::Struct { fields } => {
                match fields {
                    StructFields::Named(fields) => {
                        assert_eq!(fields.len(), 1);
                        assert_eq!(fields[0].name, "value");
                    }
                    _ => panic!("Expected Named struct fields"),
                }
            }
            _ => panic!("Expected Struct variant"),
        }
    }

    #[test]
    fn test_deserialize_tuple_struct_type() {
        let json = r#"{"kind":"struct","fields":["bool"]}"#;
        let result: TypeDefType = serde_json::from_str(json).unwrap();
        match result {
            TypeDefType::Struct { fields } => {
                match fields {
                    StructFields::Tuple(types) => {
                        assert_eq!(types.len(), 1);
                        match &types[0] {
                            IdlType::Simple(s) => assert_eq!(s, "bool"),
                            _ => panic!("Expected simple type"),
                        }
                    }
                    _ => panic!("Expected Tuple struct fields"),
                }
            }
            _ => panic!("Expected Struct variant"),
        }
    }

    #[test]
    fn test_deserialize_enum_type() {
        let json = r#"{"kind":"enum","variants":[{"name":"Variant1"},{"name":"Variant2"}]}"#;
        let result: TypeDefType = serde_json::from_str(json).unwrap();
        match result {
            TypeDefType::Enum { variants } => {
                assert_eq!(variants.len(), 2);
                assert_eq!(variants[0].name, "Variant1");
                assert_eq!(variants[1].name, "Variant2");
            }
            _ => panic!("Expected Enum variant"),
        }
    }

    #[test]
    fn test_deserialize_account_arg_with_aliases() {
        // Test old format with isSigner and isMut
        let json = r#"{"name":"user","isSigner":true,"isMut":false}"#;
        let result: AccountArg = serde_json::from_str(json).unwrap();
        assert_eq!(result.name, "user");
        assert_eq!(result.signer, true);
        assert_eq!(result.writable, false);
    }

    #[test]
    fn test_deserialize_account_arg_with_new_format() {
        // Test new format with signer and writable
        let json = r#"{"name":"user","signer":true,"writable":true}"#;
        let result: AccountArg = serde_json::from_str(json).unwrap();
        assert_eq!(result.name, "user");
        assert_eq!(result.signer, true);
        assert_eq!(result.writable, true);
    }

    #[test]
    fn test_deserialize_seed_const() {
        let json = r#"{"kind":"const","value":[1,2,3]}"#;
        let result: Seed = serde_json::from_str(json).unwrap();
        match result {
            Seed::Const { value } => {
                assert_eq!(value, vec![1, 2, 3]);
            }
            _ => panic!("Expected Const variant"),
        }
    }

    #[test]
    fn test_deserialize_seed_arg() {
        let json = r#"{"kind":"arg","path":"amount"}"#;
        let result: Seed = serde_json::from_str(json).unwrap();
        match result {
            Seed::Arg { path } => {
                assert_eq!(path, "amount");
            }
            _ => panic!("Expected Arg variant"),
        }
    }

    #[test]
    fn test_deserialize_seed_account() {
        let json = r#"{"kind":"account","path":"user.key"}"#;
        let result: Seed = serde_json::from_str(json).unwrap();
        match result {
            Seed::Account { path } => {
                assert_eq!(path, "user.key");
            }
            _ => panic!("Expected Account variant"),
        }
    }

    #[test]
    fn test_deserialize_full_instruction() {
        let json = r#"{
            "name": "transfer",
            "docs": ["Transfer tokens"],
            "discriminator": [1,2,3,4,5,6,7,8],
            "accounts": [
                {"name": "from", "signer": true, "writable": true},
                {"name": "to", "signer": false, "writable": true}
            ],
            "args": [
                {"name": "amount", "type": "u64"}
            ]
        }"#;
        let result: Instruction = serde_json::from_str(json).unwrap();
        assert_eq!(result.name, "transfer");
        assert_eq!(result.accounts.len(), 2);
        assert_eq!(result.args.len(), 1);
        assert_eq!(result.discriminator, Some(vec![1, 2, 3, 4, 5, 6, 7, 8]));
    }

    #[test]
    fn test_deserialize_minimal_idl() {
        let json = r#"{
            "version": "0.1.0",
            "name": "test_program",
            "instructions": []
        }"#;
        let result: Idl = serde_json::from_str(json).unwrap();
        assert_eq!(result.get_name(), "test_program");
        assert_eq!(result.get_version(), "0.1.0");
        assert_eq!(result.instructions.len(), 0);
    }

    #[test]
    fn test_deserialize_idl_with_metadata() {
        let json = r#"{
            "metadata": {
                "name": "test_program",
                "version": "1.0.0",
                "spec": "0.1.0"
            },
            "instructions": []
        }"#;
        let result: Idl = serde_json::from_str(json).unwrap();
        assert_eq!(result.get_name(), "test_program");
        assert_eq!(result.get_version(), "1.0.0");
    }

    #[test]
    fn test_serialize_and_deserialize_roundtrip() {
        let original = Idl {
            address: Some("11111111111111111111111111111111".to_string()),
            version: Some("1.0.0".to_string()),
            name: Some("test".to_string()),
            metadata: None,
            instructions: vec![Instruction {
                name: "test_ix".to_string(),
                docs: None,
                discriminator: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
                accounts: vec![],
                args: vec![],
            }],
            accounts: None,
            types: None,
            errors: None,
            events: None,
            constants: None,
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Idl = serde_json::from_str(&json).unwrap();

        assert_eq!(original.address, deserialized.address);
        assert_eq!(original.version, deserialized.version);
        assert_eq!(original.name, deserialized.name);
        assert_eq!(original.instructions.len(), deserialized.instructions.len());
    }
}
