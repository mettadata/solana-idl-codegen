use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;
pub const CREATE_EVENT_EVENT_DISCM: [u8; 8] = [27, 114, 169, 77, 222, 235, 99, 118];
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CreateEvent {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    #[serde(serialize_with = "serialize_as_string")]
    pub mint: Pubkey,
    #[serde(serialize_with = "serialize_as_string")]
    pub bonding_curve: Pubkey,
    #[serde(serialize_with = "serialize_as_string")]
    pub user: Pubkey,
}

// Helper function to use Pubkey's to_string
fn serialize_as_string<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: ToString,
{
    serializer.serialize_str(&value.to_string())
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CreateEventEvent(pub CreateEvent);
impl BorshSerialize for CreateEventEvent {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        CREATE_EVENT_EVENT_DISCM.serialize(writer)?;
        self.0.serialize(writer)
    }
}
impl CreateEventEvent {
    pub fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let maybe_discm = <[u8; 8]>::deserialize(buf)?;
        if maybe_discm != CREATE_EVENT_EVENT_DISCM {
            return Err(std::io::Error::other(format!(
                "discm does not match. Expected: {:?}. Received: {:?}",
                CREATE_EVENT_EVENT_DISCM, maybe_discm
            )));
        }
        Ok(Self(CreateEvent::deserialize(buf)?))
    }
}
pub const TRADE_EVENT_EVENT_DISCM: [u8; 8] = [189, 219, 127, 211, 78, 230, 97, 238];
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TradeEvent {
    #[serde(serialize_with = "serialize_as_string")]
    pub mint: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub is_buy: bool,
    #[serde(serialize_with = "serialize_as_string")]
    pub user: Pubkey,
    pub timestamp: i64,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
}
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TradeEventEvent(pub TradeEvent);
impl BorshSerialize for TradeEventEvent {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        TRADE_EVENT_EVENT_DISCM.serialize(writer)?;
        self.0.serialize(writer)
    }
}
impl TradeEventEvent {
    pub fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let maybe_discm = <[u8; 8]>::deserialize(buf)?;
        if maybe_discm != TRADE_EVENT_EVENT_DISCM {
            return Err(std::io::Error::other(format!(
                "discm does not match. Expected: {:?}. Received: {:?}",
                TRADE_EVENT_EVENT_DISCM, maybe_discm
            )));
        }
        Ok(Self(TradeEvent::deserialize(buf)?))
    }
}
pub const COMPLETE_EVENT_EVENT_DISCM: [u8; 8] = [95, 114, 97, 156, 212, 46, 152, 8];
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct CompleteEvent {
    pub user: Pubkey,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub timestamp: i64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CompleteEventEvent(pub CompleteEvent);
impl BorshSerialize for CompleteEventEvent {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        COMPLETE_EVENT_EVENT_DISCM.serialize(writer)?;
        self.0.serialize(writer)
    }
}
impl CompleteEventEvent {
    pub fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let maybe_discm = <[u8; 8]>::deserialize(buf)?;
        if maybe_discm != COMPLETE_EVENT_EVENT_DISCM {
            return Err(std::io::Error::other(format!(
                "discm does not match. Expected: {:?}. Received: {:?}",
                COMPLETE_EVENT_EVENT_DISCM, maybe_discm
            )));
        }
        Ok(Self(CompleteEvent::deserialize(buf)?))
    }
}
pub const SET_PARAMS_EVENT_EVENT_DISCM: [u8; 8] = [223, 195, 159, 246, 62, 48, 143, 131];
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct SetParamsEvent {
    pub fee_recipient: Pubkey,
    pub initial_virtual_token_reserves: u64,
    pub initial_virtual_sol_reserves: u64,
    pub initial_real_token_reserves: u64,
    pub token_total_supply: u64,
    pub fee_basis_points: u64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SetParamsEventEvent(pub SetParamsEvent);
impl BorshSerialize for SetParamsEventEvent {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        SET_PARAMS_EVENT_EVENT_DISCM.serialize(writer)?;
        self.0.serialize(writer)
    }
}
impl SetParamsEventEvent {
    pub fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let maybe_discm = <[u8; 8]>::deserialize(buf)?;
        if maybe_discm != SET_PARAMS_EVENT_EVENT_DISCM {
            return Err(std::io::Error::other(format!(
                "discm does not match. Expected: {:?}. Received: {:?}",
                SET_PARAMS_EVENT_EVENT_DISCM, maybe_discm
            )));
        }
        Ok(Self(SetParamsEvent::deserialize(buf)?))
    }
}
