use std::iter::FromIterator;

use rust_rtps_pim::{
    behavior::types::{DurationType, ParticipantMessageDataType},
    messages::types::{
        CountType, FragmentNumberType, GroupDigestType, ParameterIdType, ProtocolIdType,
        SubmessageFlagType, SubmessageKindType, TimeType,
    },
    structure::types::{
        DataType, EntityIdType, GUIDType, GuidPrefixType, InstanceHandleType, LocatorType,
        ParameterListType, ProtocolVersionType, SequenceNumberType, VendorIdType,
    },
};

pub mod submessages;

pub struct RtpsUdpPsm;

impl GuidPrefixType for RtpsUdpPsm {
    type GuidPrefix = GuidPrefix;
    const GUIDPREFIX_UNKNOWN: Self::GuidPrefix = GuidPrefix([0; 12]);
}

impl EntityIdType for RtpsUdpPsm {
    type EntityId = EntityId;
    const ENTITYID_UNKNOWN: Self::EntityId = EntityId {
        entity_key: [0; 3],
        entity_kind: 0,
    };

    const ENTITYID_PARTICIPANT: Self::EntityId = EntityId {
        entity_key: [0, 0, 0x01],
        entity_kind: 0xc1,
    };
}

impl GUIDType<RtpsUdpPsm> for RtpsUdpPsm {
    type GUID = GUID;
    const GUID_UNKNOWN: Self::GUID = GUID {
        prefix: RtpsUdpPsm::GUIDPREFIX_UNKNOWN,
        entity_id: RtpsUdpPsm::ENTITYID_UNKNOWN,
    };
}

#[derive(Clone, Copy, PartialEq)]
pub struct GUID {
    pub prefix: GuidPrefix,
    pub entity_id: EntityId,
}

impl rust_rtps_pim::structure::types::GUID<RtpsUdpPsm> for GUID {
    fn new(prefix: GuidPrefix, entity_id: EntityId) -> Self {
        Self { prefix, entity_id }
    }

    fn prefix(&self) -> &GuidPrefix {
        todo!()
    }

    fn entity_id(&self) -> &EntityId {
        todo!()
    }
}

impl SequenceNumberType for RtpsUdpPsm {
    type SequenceNumber = SequenceNumber;
    const SEQUENCE_NUMBER_UNKNOWN: Self::SequenceNumber = SequenceNumber {
        high: core::i32::MIN,
        low: core::u32::MAX,
    };
}

impl LocatorType for RtpsUdpPsm {
    type Locator = Locator;
}

impl InstanceHandleType for RtpsUdpPsm {
    type InstanceHandle = InstanceHandle;
}

impl ProtocolVersionType for RtpsUdpPsm {
    type ProtocolVersion = ProtocolVersion;
    const PROTOCOLVERSION: Self::ProtocolVersion = Self::PROTOCOLVERSION_2_4;
    const PROTOCOLVERSION_1_0: Self::ProtocolVersion = ProtocolVersion { major: 1, minor: 0 };
    const PROTOCOLVERSION_1_1: Self::ProtocolVersion = ProtocolVersion { major: 1, minor: 1 };
    const PROTOCOLVERSION_2_0: Self::ProtocolVersion = ProtocolVersion { major: 2, minor: 0 };
    const PROTOCOLVERSION_2_1: Self::ProtocolVersion = ProtocolVersion { major: 2, minor: 1 };
    const PROTOCOLVERSION_2_2: Self::ProtocolVersion = ProtocolVersion { major: 2, minor: 2 };
    const PROTOCOLVERSION_2_3: Self::ProtocolVersion = ProtocolVersion { major: 2, minor: 3 };
    const PROTOCOLVERSION_2_4: Self::ProtocolVersion = ProtocolVersion { major: 2, minor: 4 };
}

impl VendorIdType for RtpsUdpPsm {
    type VendorId = VendorId;
    const VENDOR_ID_UNKNOWN: Self::VendorId = VendorId([0; 2]);
}

impl DataType for RtpsUdpPsm {
    type Data = Data;
}

impl ProtocolIdType for RtpsUdpPsm {
    type ProtocolId = ProtocolId;
    const PROTOCOL_RTPS: Self::ProtocolId = [b'R', b'T', b'P', b'S'];
}

impl ParameterListType<RtpsUdpPsm> for RtpsUdpPsm {
    type ParameterList = ParameterList;
}

impl SubmessageFlagType for RtpsUdpPsm {
    type SubmessageFlag = SubmessageFlag;
}

type SubmessageKind = u8;

impl SubmessageKindType for RtpsUdpPsm {
    type SubmessageKind = SubmessageKind;
    const DATA: Self::SubmessageKind = 0x15;
    const GAP: Self::SubmessageKind = 0x08;
    const HEARTBEAT: Self::SubmessageKind = 0x07;
    const ACKNACK: Self::SubmessageKind = 0x06;
    const PAD: Self::SubmessageKind = 0x01;
    const INFO_TS: Self::SubmessageKind = 0x09;
    const INFO_REPLY: Self::SubmessageKind = 0x0f;
    const INFO_DST: Self::SubmessageKind = 0x0e;
    const INFO_SRC: Self::SubmessageKind = 0x0c;
    const DATA_FRAG: Self::SubmessageKind = 0x16;
    const NACK_FRAG: Self::SubmessageKind = 0x12;
    const HEARTBEAT_FRAG: Self::SubmessageKind = 0x13;
}

impl TimeType for RtpsUdpPsm {
    type Time = Time;
    const TIME_ZERO: Self::Time = Time {
        seconds: 0,
        fraction: 0,
    };
    const TIME_INVALID: Self::Time = Time {
        seconds: 0xffffffff,
        fraction: 0xffffffff,
    };
    const TIME_INFINITE: Self::Time = Time {
        seconds: 0xffffffff,
        fraction: 0xfffffffe,
    };
}

impl CountType for RtpsUdpPsm {
    type Count = Count;
}

impl ParameterIdType for RtpsUdpPsm {
    type ParameterId = ParameterId;
}

impl FragmentNumberType for RtpsUdpPsm {
    type FragmentNumber = FragmentNumber;
}

impl GroupDigestType for RtpsUdpPsm {
    type GroupDigest = GroupDigest;
}

impl DurationType for RtpsUdpPsm {
    type Duration = Duration;
}

impl ParticipantMessageDataType for RtpsUdpPsm {
    type ParticipantMessageData = ();
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UShort(u16);

impl rust_rtps_pim::messages::submessage_elements::UShort for UShort {
    fn value(&self) -> &u16 {
        &self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Long(i32);

impl rust_rtps_pim::messages::submessage_elements::Long for Long {
    fn value(&self) -> &i32 {
        &self.0
    }
}

impl From<[u8; 4]> for Long {
    fn from(value: [u8; 4]) -> Self {
        Self(i32::from_le_bytes(value))
    }
}

impl Into<[u8; 4]> for Long {
    fn into(self) -> [u8; 4] {
        self.0.to_le_bytes()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ULong(u32);

impl rust_rtps_pim::messages::submessage_elements::ULong for ULong {
    fn value(&self) -> &u32 {
        &self.0
    }
}

impl From<[u8; 4]> for ULong {
    fn from(value: [u8; 4]) -> Self {
        Self(u32::from_le_bytes(value))
    }
}

impl Into<[u8; 4]> for ULong {
    fn into(self) -> [u8; 4] {
        self.0.to_le_bytes()
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct GuidPrefix(pub [u8; 12]);

impl From<[u8; 12]> for GuidPrefix {
    fn from(value: [u8; 12]) -> Self {
        Self(value)
    }
}

impl Into<[u8; 12]> for GuidPrefix {
    fn into(self) -> [u8; 12] {
        self.0
    }
}

impl rust_rtps_pim::messages::submessage_elements::GuidPrefix<RtpsUdpPsm> for GuidPrefix {
    fn value(&self) -> &GuidPrefix {
        self
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct EntityId {
    pub entity_key: [u8; 3],
    pub entity_kind: u8,
}

impl Into<[u8; 4]> for EntityId {
    fn into(self) -> [u8; 4] {
        [
            self.entity_key[0],
            self.entity_key[1],
            self.entity_key[2],
            self.entity_kind,
        ]
    }
}

impl From<[u8; 4]> for EntityId {
    fn from(value: [u8; 4]) -> Self {
        Self {
            entity_key: [value[0], value[1], value[2]],
            entity_kind: value[3],
        }
    }
}

impl rust_rtps_pim::messages::submessage_elements::EntityId<RtpsUdpPsm> for EntityId {
    fn value(&self) -> &EntityId {
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SequenceNumber {
    pub high: i32,
    pub low: u32,
}
impl PartialOrd for SequenceNumber {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Into::<i64>::into(*self).partial_cmp(&(*other).into())
    }
}
impl Ord for SequenceNumber {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Into::<i64>::into(*self).cmp(&(*other).into())
    }
}

impl Into<i64> for SequenceNumber {
    fn into(self) -> i64 {
        ((self.high as i64) << 32) + self.low as i64
    }
}

impl From<i64> for SequenceNumber {
    fn from(value: i64) -> Self {
        Self {
            high: (value >> 32) as i32,
            low: value as u32,
        }
    }
}

impl rust_rtps_pim::messages::submessage_elements::SequenceNumber<RtpsUdpPsm> for SequenceNumber {
    fn value(&self) -> &SequenceNumber {
        self
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Locator {
    pub kind: Long,
    pub port: ULong,
    pub address: [u8; 16],
}

impl rust_rtps_pim::structure::types::LocatorSubTypes for Locator {
    type LocatorKind = Long;
    type LocatorPort = ULong;
    type LocatorAddress = [u8; 16];

    const LOCATOR_KIND_INVALID: Self::LocatorKind = Long(-1);
    const LOCATOR_KIND_RESERVED: Self::LocatorKind = Long(0);
    #[allow(non_upper_case_globals)]
    const LOCATOR_KIND_UDPv4: Self::LocatorKind = Long(1);
    #[allow(non_upper_case_globals)]
    const LOCATOR_KIND_UDPv6: Self::LocatorKind = Long(2);
    const LOCATOR_ADDRESS_INVALID: Self::LocatorAddress = [0; 16];
    const LOCATOR_PORT_INVALID: Self::LocatorPort = ULong(0);

    const LOCATOR_INVALID: Self = Locator {
        kind: Self::LOCATOR_KIND_INVALID,
        port: Self::LOCATOR_PORT_INVALID,
        address: Self::LOCATOR_ADDRESS_INVALID,
    };

    fn kind(&self) -> &Self::LocatorKind {
        &self.kind
    }

    fn port(&self) -> &Self::LocatorPort {
        &self.port
    }

    fn address(&self) -> &Self::LocatorAddress {
        &self.address
    }
}

#[derive(Clone, Copy)]
pub struct SequenceNumberSet {
    base: SequenceNumber,
    bitmap: [i32; 8],
}

impl IntoIterator for SequenceNumberSet {
    type Item = SequenceNumber;
    type IntoIter = SequenceNumberSetIterator;

    fn into_iter(self) -> Self::IntoIter {
        SequenceNumberSetIterator {
            set: self,
            index: 0,
        }
    }
}

impl FromIterator<SequenceNumber> for SequenceNumberSet {
    fn from_iter<T: IntoIterator<Item = SequenceNumber>>(iter: T) -> Self {
        let mut iterator = iter.into_iter();
        if let Some(base) = iterator.next() {
            // The base is always present
            let mut bitmap = [1, 0, 0, 0, 0, 0, 0, 0];
            while let Some(value) = iterator.next() {
                let offset = Into::<i64>::into(value) - Into::<i64>::into(base);
                let array_index = offset / 32;
                let bit_position = offset - array_index * 32;
                bitmap[array_index as usize] |= 1 << bit_position;
            }
            Self { base, bitmap }
        } else {
            Self {
                base: 0.into(),
                bitmap: [0; 8],
            }
        }
    }
}

impl rust_rtps_pim::messages::submessage_elements::SequenceNumberSet<RtpsUdpPsm>
    for SequenceNumberSet
{
    type SequenceNumberVector = Self;

    fn base(&self) -> &SequenceNumber {
        &self.base
    }

    fn set(&self) -> &Self::SequenceNumberVector {
        self
    }
}

pub struct SequenceNumberSetIterator {
    set: SequenceNumberSet,
    index: u32,
}

impl Iterator for SequenceNumberSetIterator {
    type Item = SequenceNumber;

    fn next(&mut self) -> Option<Self::Item> {
        for index in self.index..256 {
            // First determine which of the 32 bit parts of the array needs to be used
            let array_index = (index / 32) as usize;
            // Then get the bit position we are looking at inside the array
            let bit_position = index - array_index as u32 * 32;
            // If that bit is 1 then return it as a sequence number value
            if self.set.bitmap[array_index] & (1 << bit_position) == 1 << bit_position {
                let next_seq_num = Some(
                    (Into::<i64>::into(self.set.base)
                        + array_index as i64 * 32
                        + bit_position as i64)
                        .into(),
                );
                self.index = index + 1;
                return next_seq_num;
            }
        }
        self.index = 256;
        None
    }
}

pub type InstanceHandle = i32;

#[derive(Clone, Copy)]
pub struct ProtocolVersion {
    pub major: u8,
    pub minor: u8,
}

impl rust_rtps_pim::messages::submessage_elements::ProtocolVersion<RtpsUdpPsm> for ProtocolVersion {
    fn value(&self) -> &ProtocolVersion {
        self
    }
}

pub struct Data(Vec<u8>);

pub struct SerializedData<'a>(&'a [u8]);

impl<'a> rust_rtps_pim::messages::submessage_elements::SerializedData for SerializedData<'a> {
    fn value(&self) -> &[u8] {
        self.0
    }
}

impl<'a> rust_rtps_pim::messages::submessage_elements::SerializedDataFragment
    for SerializedData<'a>
{
    fn value(&self) -> &[u8] {
        self.0
    }
}

#[derive(Clone, Copy)]
pub struct VendorId([u8; 2]);

impl rust_rtps_pim::messages::submessage_elements::VendorId<RtpsUdpPsm> for VendorId {
    fn value(&self) -> &VendorId {
        self
    }
}

pub type ProtocolId = [u8; 4];
pub type SubmessageFlag = bool;

#[derive(Clone, Copy)]
pub struct Time {
    pub seconds: u32,
    pub fraction: u32,
}

impl rust_rtps_pim::messages::submessage_elements::Timestamp<RtpsUdpPsm> for Time {
    fn value(&self) -> &Time {
        self
    }
}

#[derive(Clone, Copy)]
pub struct Count(i32);

impl rust_rtps_pim::messages::submessage_elements::Count<RtpsUdpPsm> for Count {
    fn value(&self) -> &Count {
        self
    }
}

pub type ParameterId = i16;
#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct FragmentNumber(u32);

impl rust_rtps_pim::messages::submessage_elements::FragmentNumber<RtpsUdpPsm> for FragmentNumber {
    fn value(&self) -> &FragmentNumber {
        self
    }
}

pub struct FragmentNumberSet(Vec<FragmentNumber>);

impl rust_rtps_pim::messages::submessage_elements::FragmentNumberSet<RtpsUdpPsm>
    for FragmentNumberSet
{
    type FragmentNumberVector = Self;

    fn base(&self) -> &FragmentNumber {
        &FragmentNumber(0)
    }

    fn set(&self) -> &Self::FragmentNumberVector {
        self
    }
}

pub type GroupDigest = [u8; 4];

#[derive(Clone, Copy)]
pub struct Duration {
    pub seconds: i32,
    pub fraction: u32,
}

#[derive(Clone)]
pub struct Parameter {
    pub parameter_id: ParameterId,
    pub length: i16,
    pub value: Vec<u8>,
}

impl rust_rtps_pim::messages::submessage_elements::Parameter<RtpsUdpPsm> for Parameter {
    fn parameter_id(&self) -> ParameterId {
        self.parameter_id
    }

    fn length(&self) -> i16 {
        self.length
    }

    fn value(&self) -> &[u8] {
        &self.value
    }
}

pub struct ParameterList {
    pub parameter: Vec<Parameter>,
}

impl rust_rtps_pim::messages::submessage_elements::ParameterList<RtpsUdpPsm> for ParameterList {
    type Parameter = Parameter;

    fn parameter(&self) -> &[Self::Parameter] {
        &self.parameter
    }
}

pub struct LocatorList(Vec<Locator>);

impl rust_rtps_pim::messages::submessage_elements::LocatorList<RtpsUdpPsm> for LocatorList {
    fn value(&self) -> &[Locator] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_number_set_iterator() {
        let mut sequence_number_iterator = SequenceNumberSetIterator {
            set: SequenceNumberSet {
                base: 1234.into(),
                bitmap: [3, 1, 0, 0, 0, 0, 0, 1],
            },
            index: 0,
        };

        assert_eq!(sequence_number_iterator.next().unwrap(), 1234.into());
        assert_eq!(sequence_number_iterator.next().unwrap(), 1235.into());
        assert_eq!(sequence_number_iterator.next().unwrap(), 1266.into());
        assert_eq!(sequence_number_iterator.next().unwrap(), 1458.into());
        assert_eq!(sequence_number_iterator.next(), None);
        assert_eq!(sequence_number_iterator.next(), None);
    }

    #[test]
    fn sequence_number_set_from_iterator() {
        let sequence_numbers: [SequenceNumber; 3] = [2.into(), 4.into(), 66.into()];
        let sequence_number_set: SequenceNumberSet = sequence_numbers.iter().copied().collect();
        assert_eq!(sequence_number_set.base, 2.into());
        assert_eq!(sequence_number_set.bitmap[0], 5);
        assert_eq!(sequence_number_set.bitmap[1], 0);
        assert_eq!(sequence_number_set.bitmap[2], 1);
    }

    #[test]
    fn sequence_number_set_from_empty_iterator() {
        let sequence_number_set: SequenceNumberSet = core::iter::empty().collect();
        assert_eq!(sequence_number_set.base, 0.into());
        assert_eq!(sequence_number_set.bitmap[0], 0);
        assert_eq!(sequence_number_set.bitmap[1], 0);
        assert_eq!(sequence_number_set.bitmap[2], 0);
    }

    #[test]
    #[should_panic]
    fn sequence_number_set_from_iterator_unordered_input() {
        let sequence_numbers: [SequenceNumber; 3] = [66.into(), 2.into(), 4.into()];
        let sequence_number_set: SequenceNumberSet = sequence_numbers.iter().copied().collect();
        assert_eq!(sequence_number_set.base, 2.into());
        assert_eq!(sequence_number_set.bitmap[0], 5);
        assert_eq!(sequence_number_set.bitmap[1], 0);
        assert_eq!(sequence_number_set.bitmap[2], 1);
    }

    #[test]
    #[should_panic]
    fn sequence_number_set_from_iterator_above_capacity() {
        let sequence_numbers: [SequenceNumber; 3] = [2.into(), 4.into(), 500.into()];
        let sequence_number_set: SequenceNumberSet = sequence_numbers.iter().copied().collect();
        assert_eq!(sequence_number_set.base, 2.into());
        assert_eq!(sequence_number_set.bitmap[0], 5);
        assert_eq!(sequence_number_set.bitmap[1], 0);
        assert_eq!(sequence_number_set.bitmap[2], 1);
    }
}

// impl EntityId {
//     pub const ENTITY_KIND_USER_DEFINED_UNKNOWN: u8 = 0x00;
//     pub const ENTITY_KIND_USER_DEFINED_WRITER_WITH_KEY: u8 = 0x02;
//     pub const ENTITY_KIND_USER_DEFINED_WRITER_NO_KEY: u8 = 0x03;
//     pub const ENTITY_KIND_USER_DEFINED_READER_WITH_KEY: u8 = 0x04;
//     pub const ENTITY_KIND_USER_DEFINED_READER_NO_KEY: u8 = 0x07;
//     pub const ENTITY_KIND_USER_DEFINED_WRITER_GROUP: u8 = 0x08;
//     pub const ENTITY_KIND_USER_DEFINED_READER_GROUP: u8 = 0x09;
//     pub const ENTITY_KIND_BUILT_IN_UNKNOWN: u8 = 0xc0;
//     pub const ENTITY_KIND_BUILT_IN_PARTICIPANT: u8 = 0xc1;
//     pub const ENTITY_KIND_BUILT_IN_WRITER_WITH_KEY: u8 = 0xc2;
//     pub const ENTITY_KIND_BUILT_IN_WRITER_NO_KEY: u8 = 0xc3;
//     pub const ENTITY_KIND_BUILT_IN_READER_WITH_KEY: u8 = 0xc4;
//     pub const ENTITY_KIND_BUILT_IN_READER_NO_KEY: u8 = 0xc7;
//     pub const ENTITY_KIND_BUILT_IN_WRITER_GROUP: u8 = 0xc8;
//     pub const ENTITY_KIND_BUILT_IN_READER_GROUP: u8 = 0xc9;

//     pub const ENTITYID_PARTICIPANT: EntityId = EntityId {
//         entity_key: [0, 0, 0x01],
//         entity_kind: 0xc1,
//     };

//     pub const ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER: EntityId = EntityId {
//         entity_key: [0, 0, 0x02],
//         entity_kind: 0xc2,
//     };
//     pub const ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR: EntityId = EntityId {
//         entity_key: [0, 0, 0x02],
//         entity_kind: 0xc7,
//     };

//     pub const ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER: EntityId = EntityId {
//         entity_key: [0, 0, 0x03],
//         entity_kind: 0xc2,
//     };
//     pub const ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR: EntityId = EntityId {
//         entity_key: [0, 0, 0x03],
//         entity_kind: 0xc7,
//     };

//     pub const ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER: EntityId = EntityId {
//         entity_key: [0, 0, 0x04],
//         entity_kind: 0xc2,
//     };
//     pub const ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR: EntityId = EntityId {
//         entity_key: [0, 0, 0x04],
//         entity_kind: 0xc7,
//     };

//     pub const ENTITYID_SPDP_BUILTIN_PARTICIPANT_ANNOUNCER: EntityId = EntityId {
//         entity_key: [0, 0x01, 0x00],
//         entity_kind: 0xc2,
//     };

//     pub const ENTITYID_SPDP_BUILTIN_PARTICIPANT_DETECTOR: EntityId = EntityId {
//         entity_key: [0, 0x01, 0x00],
//         entity_kind: 0xc7,
//     };

//     pub const ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_WRITER: EntityId = EntityId {
//         entity_key: [0, 0x02, 0x00],
//         entity_kind: 0xc2,
//     };
//     pub const ENTITYID_BUILTIN_PARTICIPANT_MESSAGE_READER: EntityId = EntityId {
//         entity_key: [0, 0x02, 0x00],
//         entity_kind: 0xc7,
//     };
// }

// impl rust_rtps_pim::types::EntityId for EntityId {
//     const ENTITYID_UNKNOWN: Self = Self {
//         entity_key: [0; 3],
//         entity_kind: 0,
//     };
// }
