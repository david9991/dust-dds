use crate::{
    messages::{submessage_elements::Parameter, submessages::DataSubmessage},
    structure::{
        cache_change::RtpsCacheChangeConstructor,
        history_cache::RtpsHistoryCacheOperations,
        types::{ChangeKind, Guid, GuidPrefix},
    },
};

pub struct BestEffortStatelessReaderBehavior;

impl BestEffortStatelessReaderBehavior {
    pub fn receive_data<'a, CacheChange, P>(
        reader_cache: &mut impl RtpsHistoryCacheOperations<CacheChangeType = CacheChange>,
        source_guid_prefix: GuidPrefix,
        data: &DataSubmessage<'_, P>,
    ) where
        for<'b> CacheChange: RtpsCacheChangeConstructor<
            'b,
            DataType = &'b [u8],
            ParameterListType = &'b [Parameter<'b>],
        >,
        P: AsRef<[Parameter<'a>]>,
    {
        let kind = match (data.data_flag, data.key_flag) {
            (true, false) => ChangeKind::Alive,
            (false, true) => ChangeKind::NotAliveDisposed,
            _ => todo!(),
        };
        let writer_guid = Guid::new(source_guid_prefix, data.writer_id.value);
        let instance_handle = 0;
        let sequence_number = data.writer_sn.value;
        let data_value = data.serialized_payload.value;
        let inline_qos = data.inline_qos.parameter.as_ref();
        let a_change = CacheChange::new(
            kind,
            writer_guid,
            instance_handle,
            sequence_number,
            data_value,
            inline_qos,
        );
        reader_cache.add_change(a_change);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        messages::submessage_elements::{
            EntityIdSubmessageElement, ParameterListSubmessageElement,
            SequenceNumberSubmessageElement, SerializedDataSubmessageElement,
        },
        structure::types::{InstanceHandle, SequenceNumber, ENTITYID_UNKNOWN},
    };

    use super::*;

    use mockall::mock;

    // Cache change is not mocked with the mocking framework since
    // both the constructor and the attributes don't need to be defined as part of the test run
    #[derive(Debug, PartialEq)]
    struct MockCacheChange;

    impl<'a> RtpsCacheChangeConstructor<'a> for MockCacheChange {
        type DataType = &'a [u8];
        type ParameterListType = &'a [Parameter<'a>];

        fn new(
            _kind: ChangeKind,
            _writer_guid: Guid,
            _instance_handle: InstanceHandle,
            _sequence_number: SequenceNumber,
            _data_value: Self::DataType,
            _inline_qos: Self::ParameterListType,
        ) -> Self {
            Self
        }
    }

    mock! {
        HistoryCache{
            fn add_change_(&mut self, change: MockCacheChange);
        }
    }

    impl RtpsHistoryCacheOperations for MockHistoryCache {
        type CacheChangeType = MockCacheChange;

        fn add_change(&mut self, change: Self::CacheChangeType) {
            self.add_change_(change)
        }

        fn remove_change<F>(&mut self, _f: F)
        where
            F: FnMut(&Self::CacheChangeType) -> bool,
        {
            todo!()
        }

        fn get_seq_num_min(&self) -> Option<SequenceNumber> {
            todo!()
        }

        fn get_seq_num_max(&self) -> Option<SequenceNumber> {
            todo!()
        }
    }

    #[test]
    fn best_effort_stateless_reader_receive_data() {
        let mut reader_cache = MockHistoryCache::new();
        let source_guid_prefix = GuidPrefix([1; 12]);
        let data = DataSubmessage {
            endianness_flag: true,
            inline_qos_flag: true,
            data_flag: true,
            key_flag: false,
            non_standard_payload_flag: false,
            reader_id: EntityIdSubmessageElement {
                value: ENTITYID_UNKNOWN,
            },
            writer_id: EntityIdSubmessageElement {
                value: ENTITYID_UNKNOWN,
            },
            writer_sn: SequenceNumberSubmessageElement { value: 1 },
            inline_qos: ParameterListSubmessageElement { parameter: vec![] },
            serialized_payload: SerializedDataSubmessageElement {
                value: &[1, 2, 3, 4],
            },
        };
        reader_cache.expect_add_change_().once().return_const(());

        BestEffortStatelessReaderBehavior::receive_data(
            &mut reader_cache,
            source_guid_prefix,
            &data,
        );
    }
}
