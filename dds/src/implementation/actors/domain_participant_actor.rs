use dust_dds_derive::actor_interface;
use tracing::warn;

use crate::{
    builtin_topics::{BuiltInTopicKey, ParticipantBuiltinTopicData, TopicBuiltinTopicData},
    dds::infrastructure,
    dds_async::{
        domain_participant::DomainParticipantAsync,
        domain_participant_listener::DomainParticipantListenerAsync,
        publisher_listener::PublisherListenerAsync, subscriber_listener::SubscriberListenerAsync,
        topic_listener::TopicListenerAsync,
    },
    domain::domain_participant_factory::DomainId,
    implementation::{
        actors::{
            data_reader_actor::DataReaderActor, subscriber_actor::SubscriberActor,
            topic_actor::TopicActor,
        },
        data_representation_builtin_endpoints::{
            discovered_reader_data::{DiscoveredReaderData, DCPS_SUBSCRIPTION},
            discovered_topic_data::{DiscoveredTopicData, DCPS_TOPIC},
            discovered_writer_data::{DiscoveredWriterData, DCPS_PUBLICATION},
            spdp_discovered_participant_data::{
                ParticipantProxy, SpdpDiscoveredParticipantData, DCPS_PARTICIPANT,
            },
        },
        rtps::{
            discovery_types::{BuiltinEndpointQos, BuiltinEndpointSet},
            endpoint::RtpsEndpoint,
            group::RtpsGroup,
            messages::{
                overall_structure::{RtpsMessageHeader, RtpsMessageRead},
                types::Count,
            },
            participant::RtpsParticipant,
            reader::{RtpsReader, RtpsReaderKind, RtpsStatefulReader, RtpsStatelessReader},
            reader_locator::RtpsReaderLocator,
            reader_proxy::RtpsReaderProxy,
            types::{
                EntityId, Guid, Locator, ReliabilityKind, TopicKind, BUILT_IN_READER_GROUP,
                BUILT_IN_READER_WITH_KEY, BUILT_IN_TOPIC, BUILT_IN_WRITER_GROUP,
                BUILT_IN_WRITER_WITH_KEY, ENTITYID_PARTICIPANT, ENTITYID_UNKNOWN,
                USER_DEFINED_READER_GROUP, USER_DEFINED_TOPIC, USER_DEFINED_WRITER_GROUP,
            },
            writer::RtpsWriter,
            writer_proxy::RtpsWriterProxy,
        },
        rtps_udp_psm::udp_transport::UdpTransportWrite,
        utils::{
            actor::{Actor, ActorAddress},
            instance_handle_from_key::get_instance_handle_from_key,
        },
    },
    infrastructure::{
        error::{DdsError, DdsResult},
        instance::InstanceHandle,
        qos::{
            DataReaderQos, DataWriterQos, DomainParticipantQos, PublisherQos, QosKind,
            SubscriberQos, TopicQos,
        },
        qos_policy::{
            DurabilityQosPolicy, DurabilityQosPolicyKind, HistoryQosPolicy, HistoryQosPolicyKind,
            LifespanQosPolicy, ReliabilityQosPolicy, ReliabilityQosPolicyKind,
            ResourceLimitsQosPolicy, TransportPriorityQosPolicy,
        },
        status::StatusKind,
        time::{Duration, DurationKind, DURATION_ZERO},
    },
    subscription::sample_info::{
        InstanceStateKind, SampleStateKind, ANY_INSTANCE_STATE, ANY_SAMPLE_STATE, ANY_VIEW_STATE,
    },
    topic_definition::type_support::{
        deserialize_rtps_classic_cdr, serialize_rtps_classic_cdr_le, DdsDeserialize, DdsHasKey,
        DdsKey, DdsSerialize, DdsTypeXml, DynamicTypeInterface,
    },
};

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    data_writer_actor::DataWriterActor,
    domain_participant_listener_actor::DomainParticipantListenerActor,
    publisher_actor::PublisherActor, status_condition_actor::StatusConditionActor,
    type_support_actor::TypeSupportActor,
};

pub const ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER: EntityId =
    EntityId::new([0x00, 0x01, 0x00], BUILT_IN_WRITER_WITH_KEY);

pub const ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER: EntityId =
    EntityId::new([0x00, 0x01, 0x00], BUILT_IN_READER_WITH_KEY);

pub const ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER: EntityId =
    EntityId::new([0, 0, 0x02], BUILT_IN_WRITER_WITH_KEY);

pub const ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR: EntityId =
    EntityId::new([0, 0, 0x02], BUILT_IN_READER_WITH_KEY);

pub const ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER: EntityId =
    EntityId::new([0, 0, 0x03], BUILT_IN_WRITER_WITH_KEY);

pub const ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR: EntityId =
    EntityId::new([0, 0, 0x03], BUILT_IN_READER_WITH_KEY);

pub const ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER: EntityId =
    EntityId::new([0, 0, 0x04], BUILT_IN_WRITER_WITH_KEY);

pub const ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR: EntityId =
    EntityId::new([0, 0, 0x04], BUILT_IN_READER_WITH_KEY);

pub const DEFAULT_HEARTBEAT_PERIOD: Duration = Duration::new(2, 0);
pub const DEFAULT_NACK_RESPONSE_DELAY: Duration = Duration::new(0, 200);
pub const DEFAULT_NACK_SUPPRESSION_DURATION: Duration = DURATION_ZERO;
pub const DEFAULT_HEARTBEAT_RESPONSE_DELAY: Duration = Duration::new(0, 500);
pub const DEFAULT_HEARTBEAT_SUPPRESSION_DURATION: Duration = DURATION_ZERO;

pub struct FooTypeSupport {
    has_key: bool,
    get_serialized_key_from_serialized_foo: fn(&[u8]) -> DdsResult<Vec<u8>>,
    instance_handle_from_serialized_foo: fn(&[u8]) -> DdsResult<InstanceHandle>,
    instance_handle_from_serialized_key: fn(&[u8]) -> DdsResult<InstanceHandle>,
    type_xml: String,
}

impl FooTypeSupport {
    pub fn new<Foo>() -> Self
    where
        Foo: DdsKey + DdsHasKey + DdsTypeXml,
    {
        // This function is a workaround due to an issue resolving
        // lifetimes of the closure.
        // See for more details: https://github.com/rust-lang/rust/issues/41078
        fn define_function_with_correct_lifetime<F, O>(closure: F) -> F
        where
            F: for<'a> Fn(&'a [u8]) -> DdsResult<O>,
        {
            closure
        }

        let get_serialized_key_from_serialized_foo =
            define_function_with_correct_lifetime(|serialized_foo| {
                let mut writer = Vec::new();
                let foo_key = Foo::get_key_from_serialized_data(serialized_foo)?;
                serialize_rtps_classic_cdr_le(&foo_key, &mut writer)?;
                Ok(writer)
            });

        let instance_handle_from_serialized_foo =
            define_function_with_correct_lifetime(|serialized_foo| {
                let foo_key = Foo::get_key_from_serialized_data(serialized_foo)?;
                get_instance_handle_from_key(&foo_key)
            });

        let instance_handle_from_serialized_key =
            define_function_with_correct_lifetime(|mut serialized_key| {
                let foo_key = deserialize_rtps_classic_cdr::<Foo::Key>(&mut serialized_key)?;
                get_instance_handle_from_key(&foo_key)
            });

        let type_xml = Foo::get_type_xml().unwrap_or(String::new());

        Self {
            has_key: Foo::HAS_KEY,
            get_serialized_key_from_serialized_foo,
            instance_handle_from_serialized_foo,
            instance_handle_from_serialized_key,
            type_xml,
        }
    }
}

impl DynamicTypeInterface for FooTypeSupport {
    fn has_key(&self) -> bool {
        self.has_key
    }

    fn get_serialized_key_from_serialized_foo(&self, serialized_foo: &[u8]) -> DdsResult<Vec<u8>> {
        (self.get_serialized_key_from_serialized_foo)(serialized_foo)
    }

    fn instance_handle_from_serialized_foo(
        &self,
        serialized_foo: &[u8],
    ) -> DdsResult<InstanceHandle> {
        (self.instance_handle_from_serialized_foo)(serialized_foo)
    }

    fn instance_handle_from_serialized_key(
        &self,
        serialized_key: &[u8],
    ) -> DdsResult<InstanceHandle> {
        (self.instance_handle_from_serialized_key)(serialized_key)
    }

    fn xml_type(&self) -> String {
        self.type_xml.clone()
    }
}

pub struct DomainParticipantActor {
    rtps_participant: RtpsParticipant,
    domain_id: DomainId,
    domain_tag: String,
    qos: DomainParticipantQos,
    builtin_subscriber: Actor<SubscriberActor>,
    builtin_publisher: Actor<PublisherActor>,
    builtin_topic_list: Vec<Actor<TopicActor>>,
    user_defined_subscriber_list: HashMap<InstanceHandle, Actor<SubscriberActor>>,
    user_defined_subscriber_counter: u8,
    default_subscriber_qos: SubscriberQos,
    user_defined_publisher_list: HashMap<InstanceHandle, Actor<PublisherActor>>,
    user_defined_publisher_counter: u8,
    default_publisher_qos: PublisherQos,
    user_defined_topic_list: HashMap<InstanceHandle, Actor<TopicActor>>,
    user_defined_topic_counter: u8,
    default_topic_qos: TopicQos,
    manual_liveliness_count: Count,
    lease_duration: Duration,
    discovered_participant_list: HashMap<InstanceHandle, SpdpDiscoveredParticipantData>,
    discovered_topic_list: HashMap<InstanceHandle, TopicBuiltinTopicData>,
    enabled: bool,
    ignored_participants: HashSet<InstanceHandle>,
    ignored_publications: HashSet<InstanceHandle>,
    ignored_subcriptions: HashSet<InstanceHandle>,
    ignored_topic_list: HashSet<InstanceHandle>,
    data_max_size_serialized: usize,
    udp_transport_write: Arc<UdpTransportWrite>,
    listener: Actor<DomainParticipantListenerActor>,
    status_kind: Vec<StatusKind>,
    type_support_actor: Actor<TypeSupportActor>,
    status_condition: Actor<StatusConditionActor>,
}

impl DomainParticipantActor {
    #[allow(clippy::too_many_arguments)]
    pub async fn new(
        rtps_participant: RtpsParticipant,
        domain_id: DomainId,
        domain_tag: String,
        domain_participant_qos: DomainParticipantQos,
        spdp_discovery_locator_list: &[Locator],
        data_max_size_serialized: usize,
        udp_transport_write: Arc<UdpTransportWrite>,
        listener: Option<Box<dyn DomainParticipantListenerAsync + Send>>,
        status_kind: Vec<StatusKind>,
        handle: &tokio::runtime::Handle,
    ) -> Self {
        let lease_duration = Duration::new(100, 0);
        let guid_prefix = rtps_participant.guid().prefix();

        let spdp_topic_entity_id = EntityId::new([0, 0, 0], BUILT_IN_TOPIC);
        let spdp_topic_guid = Guid::new(guid_prefix, spdp_topic_entity_id);
        let spdp_topic_participant = Actor::spawn(
            TopicActor::new(
                spdp_topic_guid,
                TopicQos::default(),
                "SpdpDiscoveredParticipantData".to_string(),
                DCPS_PARTICIPANT,
                None,
                handle,
            ),
            handle,
        );

        let sedp_topics_entity_id = EntityId::new([0, 0, 1], BUILT_IN_TOPIC);
        let sedp_topic_topics_guid = Guid::new(guid_prefix, sedp_topics_entity_id);
        let sedp_topic_topics = Actor::spawn(
            TopicActor::new(
                sedp_topic_topics_guid,
                TopicQos::default(),
                "DiscoveredTopicData".to_string(),
                DCPS_TOPIC,
                None,
                handle,
            ),
            handle,
        );

        let sedp_publications_entity_id = EntityId::new([0, 0, 2], BUILT_IN_TOPIC);
        let sedp_topic_publications_guid = Guid::new(guid_prefix, sedp_publications_entity_id);
        let sedp_topic_publications = Actor::spawn(
            TopicActor::new(
                sedp_topic_publications_guid,
                TopicQos::default(),
                "DiscoveredWriterData".to_string(),
                DCPS_PUBLICATION,
                None,
                handle,
            ),
            handle,
        );

        let sedp_subscriptions_entity_id = EntityId::new([0, 0, 3], BUILT_IN_TOPIC);
        let sedp_topic_subscriptions_guid = Guid::new(guid_prefix, sedp_subscriptions_entity_id);
        let sedp_topic_subscriptions = Actor::spawn(
            TopicActor::new(
                sedp_topic_subscriptions_guid,
                TopicQos::default(),
                "DiscoveredReaderData".to_string(),
                DCPS_SUBSCRIPTION,
                None,
                handle,
            ),
            handle,
        );

        // Built-in subscriber creation
        let spdp_reader_qos = DataReaderQos {
            durability: DurabilityQosPolicy {
                kind: DurabilityQosPolicyKind::TransientLocal,
            },
            history: HistoryQosPolicy {
                kind: HistoryQosPolicyKind::KeepLast(1),
            },
            reliability: ReliabilityQosPolicy {
                kind: ReliabilityQosPolicyKind::BestEffort,
                max_blocking_time: DurationKind::Finite(DURATION_ZERO),
            },
            ..Default::default()
        };
        let spdp_builtin_participant_reader_guid =
            Guid::new(guid_prefix, ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER);
        let spdp_builtin_participant_reader = Actor::spawn(
            DataReaderActor::new(
                create_builtin_stateless_reader(spdp_builtin_participant_reader_guid),
                "SpdpDiscoveredParticipantData".to_string(),
                String::from(DCPS_PARTICIPANT),
                spdp_reader_qos,
                None,
                vec![],
                handle,
                spdp_topic_participant.address(),
                spdp_topic_participant.get_statuscondition().await,
            ),
            handle,
        );

        let sedp_reader_qos = DataReaderQos {
            durability: DurabilityQosPolicy {
                kind: DurabilityQosPolicyKind::TransientLocal,
            },
            history: HistoryQosPolicy {
                kind: HistoryQosPolicyKind::KeepLast(1),
            },
            reliability: ReliabilityQosPolicy {
                kind: ReliabilityQosPolicyKind::Reliable,
                max_blocking_time: DurationKind::Finite(DURATION_ZERO),
            },
            ..Default::default()
        };

        let sedp_builtin_topics_reader_guid =
            Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR);
        let sedp_builtin_topics_reader = Actor::spawn(
            DataReaderActor::new(
                create_builtin_stateful_reader(sedp_builtin_topics_reader_guid),
                "DiscoveredTopicData".to_string(),
                String::from(DCPS_TOPIC),
                sedp_reader_qos.clone(),
                None,
                vec![],
                handle,
                sedp_topic_topics.address(),
                sedp_topic_topics.get_statuscondition().await,
            ),
            handle,
        );

        let sedp_builtin_publications_reader_guid =
            Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR);
        let sedp_builtin_publications_reader = Actor::spawn(
            DataReaderActor::new(
                create_builtin_stateful_reader(sedp_builtin_publications_reader_guid),
                "DiscoveredWriterData".to_string(),
                String::from(DCPS_PUBLICATION),
                sedp_reader_qos.clone(),
                None,
                vec![],
                handle,
                sedp_topic_publications.address(),
                sedp_topic_publications.get_statuscondition().await,
            ),
            handle,
        );

        let sedp_builtin_subscriptions_reader_guid =
            Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
        let sedp_builtin_subscriptions_reader = Actor::spawn(
            DataReaderActor::new(
                create_builtin_stateful_reader(sedp_builtin_subscriptions_reader_guid),
                "DiscoveredReaderData".to_string(),
                String::from(DCPS_SUBSCRIPTION),
                sedp_reader_qos,
                None,
                vec![],
                handle,
                sedp_topic_subscriptions.address(),
                sedp_topic_subscriptions.get_statuscondition().await,
            ),
            handle,
        );

        let builtin_subscriber = Actor::spawn(
            SubscriberActor::new(
                SubscriberQos::default(),
                RtpsGroup::new(Guid::new(
                    guid_prefix,
                    EntityId::new([0, 0, 0], BUILT_IN_READER_GROUP),
                )),
                None,
                vec![],
                handle,
            ),
            handle,
        );

        builtin_subscriber
            .address()
            .data_reader_add(
                InstanceHandle::new(spdp_builtin_participant_reader_guid.into()),
                spdp_builtin_participant_reader,
            )
            .await
            .unwrap();
        builtin_subscriber
            .address()
            .data_reader_add(
                InstanceHandle::new(sedp_builtin_topics_reader_guid.into()),
                sedp_builtin_topics_reader,
            )
            .await
            .unwrap();
        builtin_subscriber
            .address()
            .data_reader_add(
                InstanceHandle::new(sedp_builtin_publications_reader_guid.into()),
                sedp_builtin_publications_reader,
            )
            .await
            .unwrap();
        builtin_subscriber
            .address()
            .data_reader_add(
                InstanceHandle::new(sedp_builtin_subscriptions_reader_guid.into()),
                sedp_builtin_subscriptions_reader,
            )
            .await
            .unwrap();

        // Built-in publisher creation
        let spdp_writer_qos = DataWriterQos {
            durability: DurabilityQosPolicy {
                kind: DurabilityQosPolicyKind::TransientLocal,
            },
            history: HistoryQosPolicy {
                kind: HistoryQosPolicyKind::KeepLast(1),
            },
            reliability: ReliabilityQosPolicy {
                kind: ReliabilityQosPolicyKind::BestEffort,
                max_blocking_time: DurationKind::Finite(DURATION_ZERO),
            },
            ..Default::default()
        };
        let spdp_builtin_participant_writer_guid =
            Guid::new(guid_prefix, ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER);
        let spdp_builtin_participant_writer = Actor::spawn(
            DataWriterActor::new(
                create_builtin_stateless_writer(spdp_builtin_participant_writer_guid),
                "SpdpDiscoveredParticipantData".to_string(),
                String::from(DCPS_PARTICIPANT),
                None,
                vec![],
                spdp_writer_qos,
                handle,
                spdp_topic_participant.address(),
                spdp_topic_participant.get_statuscondition().await,
            ),
            handle,
        );

        for reader_locator in spdp_discovery_locator_list
            .iter()
            .map(|&locator| RtpsReaderLocator::new(locator, false))
        {
            spdp_builtin_participant_writer
                .address()
                .reader_locator_add(reader_locator)
                .await
                .unwrap();
        }

        let sedp_writer_qos = DataWriterQos {
            durability: DurabilityQosPolicy {
                kind: DurabilityQosPolicyKind::TransientLocal,
            },
            history: HistoryQosPolicy {
                kind: HistoryQosPolicyKind::KeepLast(1),
            },
            reliability: ReliabilityQosPolicy {
                kind: ReliabilityQosPolicyKind::Reliable,
                max_blocking_time: DurationKind::Finite(DURATION_ZERO),
            },
            ..Default::default()
        };
        let sedp_builtin_topics_writer_guid =
            Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER);
        let sedp_builtin_topics_writer = DataWriterActor::new(
            create_builtin_stateful_writer(sedp_builtin_topics_writer_guid),
            "DiscoveredTopicData".to_string(),
            String::from(DCPS_TOPIC),
            None,
            vec![],
            sedp_writer_qos.clone(),
            handle,
            sedp_topic_topics.address(),
            sedp_topic_topics.get_statuscondition().await,
        );
        let sedp_builtin_topics_writer_actor = Actor::spawn(sedp_builtin_topics_writer, handle);

        let sedp_builtin_publications_writer_guid =
            Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER);
        let sedp_builtin_publications_writer = DataWriterActor::new(
            create_builtin_stateful_writer(sedp_builtin_publications_writer_guid),
            "DiscoveredWriterData".to_string(),
            String::from(DCPS_PUBLICATION),
            None,
            vec![],
            sedp_writer_qos.clone(),
            handle,
            sedp_topic_publications.address(),
            sedp_topic_publications.get_statuscondition().await,
        );
        let sedp_builtin_publications_writer_actor =
            Actor::spawn(sedp_builtin_publications_writer, handle);

        let sedp_builtin_subscriptions_writer_guid =
            Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
        let sedp_builtin_subscriptions_writer = DataWriterActor::new(
            create_builtin_stateful_writer(sedp_builtin_subscriptions_writer_guid),
            "DiscoveredReaderData".to_string(),
            String::from(DCPS_SUBSCRIPTION),
            None,
            vec![],
            sedp_writer_qos,
            handle,
            sedp_topic_subscriptions.address(),
            sedp_topic_subscriptions.get_statuscondition().await,
        );
        let sedp_builtin_subscriptions_writer_actor =
            Actor::spawn(sedp_builtin_subscriptions_writer, handle);

        let builtin_publisher = Actor::spawn(
            PublisherActor::new(
                PublisherQos::default(),
                RtpsGroup::new(Guid::new(
                    guid_prefix,
                    EntityId::new([0, 0, 0], BUILT_IN_WRITER_GROUP),
                )),
                None,
                vec![],
                handle,
            ),
            handle,
        );

        builtin_publisher
            .address()
            .datawriter_add(
                InstanceHandle::new(spdp_builtin_participant_writer_guid.into()),
                spdp_builtin_participant_writer,
            )
            .await
            .unwrap();
        builtin_publisher
            .address()
            .datawriter_add(
                InstanceHandle::new(sedp_builtin_topics_writer_guid.into()),
                sedp_builtin_topics_writer_actor,
            )
            .await
            .unwrap();
        builtin_publisher
            .address()
            .datawriter_add(
                InstanceHandle::new(sedp_builtin_publications_writer_guid.into()),
                sedp_builtin_publications_writer_actor,
            )
            .await
            .unwrap();
        builtin_publisher
            .address()
            .datawriter_add(
                InstanceHandle::new(sedp_builtin_subscriptions_writer_guid.into()),
                sedp_builtin_subscriptions_writer_actor,
            )
            .await
            .unwrap();

        let mut type_support_list: HashMap<String, Arc<dyn DynamicTypeInterface + Send + Sync>> =
            HashMap::new();
        type_support_list.insert(
            "SpdpDiscoveredParticipantData".to_string(),
            Arc::new(FooTypeSupport::new::<SpdpDiscoveredParticipantData>()),
        );
        type_support_list.insert(
            "DiscoveredReaderData".to_string(),
            Arc::new(FooTypeSupport::new::<DiscoveredReaderData>()),
        );
        type_support_list.insert(
            "DiscoveredWriterData".to_string(),
            Arc::new(FooTypeSupport::new::<DiscoveredWriterData>()),
        );
        type_support_list.insert(
            "DiscoveredTopicData".to_string(),
            Arc::new(FooTypeSupport::new::<DiscoveredTopicData>()),
        );

        let type_support_actor = Actor::spawn(TypeSupportActor::new(type_support_list), handle);

        let builtin_topic_list = vec![
            spdp_topic_participant,
            sedp_topic_topics,
            sedp_topic_publications,
            sedp_topic_subscriptions,
        ];

        Self {
            rtps_participant,
            domain_id,
            domain_tag,
            qos: domain_participant_qos,
            builtin_subscriber,
            builtin_publisher,
            builtin_topic_list,
            user_defined_subscriber_list: HashMap::new(),
            user_defined_subscriber_counter: 0,
            default_subscriber_qos: SubscriberQos::default(),
            user_defined_publisher_list: HashMap::new(),
            user_defined_publisher_counter: 0,
            default_publisher_qos: PublisherQos::default(),
            user_defined_topic_list: HashMap::new(),
            user_defined_topic_counter: 0,
            default_topic_qos: TopicQos::default(),
            manual_liveliness_count: 0,
            lease_duration,
            discovered_participant_list: HashMap::new(),
            discovered_topic_list: HashMap::new(),
            enabled: false,
            ignored_participants: HashSet::new(),
            ignored_publications: HashSet::new(),
            ignored_subcriptions: HashSet::new(),
            ignored_topic_list: HashSet::new(),
            data_max_size_serialized,
            udp_transport_write,
            listener: Actor::spawn(DomainParticipantListenerActor::new(listener), handle),
            status_kind,
            type_support_actor,
            status_condition: Actor::spawn(StatusConditionActor::default(), handle),
        }
    }

    async fn lookup_discovered_topic(
        &mut self,
        topic_name: String,
        type_support: Arc<dyn DynamicTypeInterface + Send + Sync>,
        runtime_handle: tokio::runtime::Handle,
    ) -> Option<(
        ActorAddress<TopicActor>,
        ActorAddress<StatusConditionActor>,
        String,
    )> {
        for discovered_topic_data in self.discovered_topic_list.values() {
            if discovered_topic_data.name() == topic_name {
                let qos = TopicQos {
                    topic_data: discovered_topic_data.topic_data().clone(),
                    durability: discovered_topic_data.durability().clone(),
                    deadline: discovered_topic_data.deadline().clone(),
                    latency_budget: discovered_topic_data.latency_budget().clone(),
                    liveliness: discovered_topic_data.liveliness().clone(),
                    reliability: discovered_topic_data.reliability().clone(),
                    destination_order: discovered_topic_data.destination_order().clone(),
                    history: discovered_topic_data.history().clone(),
                    resource_limits: discovered_topic_data.resource_limits().clone(),
                    transport_priority: discovered_topic_data.transport_priority().clone(),
                    lifespan: discovered_topic_data.lifespan().clone(),
                    ownership: discovered_topic_data.ownership().clone(),
                };
                let type_name = discovered_topic_data.get_type_name().to_owned();
                let (topic_address, status_condition_address) = self
                    .create_user_defined_topic(
                        topic_name,
                        type_name.clone(),
                        QosKind::Specific(qos),
                        None,
                        vec![],
                        type_support,
                        runtime_handle,
                    )
                    .await;
                return Some((topic_address, status_condition_address, type_name));
            }
        }
        None
    }
}

#[actor_interface]
impl DomainParticipantActor {
    fn create_user_defined_publisher(
        &mut self,
        qos: QosKind<PublisherQos>,
        a_listener: Option<Box<dyn PublisherListenerAsync + Send>>,
        mask: Vec<StatusKind>,
        runtime_handle: tokio::runtime::Handle,
    ) -> (
        ActorAddress<PublisherActor>,
        ActorAddress<StatusConditionActor>,
    ) {
        let publisher_qos = match qos {
            QosKind::Default => self.default_publisher_qos.clone(),
            QosKind::Specific(q) => q,
        };
        let publisher_counter = self.create_unique_publisher_id();
        let entity_id = EntityId::new([publisher_counter, 0, 0], USER_DEFINED_WRITER_GROUP);
        let guid = Guid::new(self.rtps_participant.guid().prefix(), entity_id);
        let rtps_group = RtpsGroup::new(guid);
        let status_kind = mask.to_vec();
        let publisher = PublisherActor::new(
            publisher_qos,
            rtps_group,
            a_listener,
            status_kind,
            &runtime_handle,
        );

        let publisher_status_condition = publisher.get_statuscondition();

        let publisher_actor = Actor::spawn(publisher, &runtime_handle);
        let publisher_address = publisher_actor.address();
        self.user_defined_publisher_list
            .insert(InstanceHandle::new(guid.into()), publisher_actor);

        (publisher_address, publisher_status_condition)
    }

    async fn delete_user_defined_publisher(&mut self, handle: InstanceHandle) -> DdsResult<()> {
        if let Some(p) = self.user_defined_publisher_list.get(&handle) {
            if !p.data_writer_list().await.is_empty() {
                Err(DdsError::PreconditionNotMet(
                    "Publisher still contains data writers".to_string(),
                ))
            } else {
                self.user_defined_publisher_list.remove(&handle);
                Ok(())
            }
        } else {
            Err(DdsError::PreconditionNotMet(
                "Publisher can only be deleted from its parent participant".to_string(),
            ))
        }
    }

    fn create_user_defined_subscriber(
        &mut self,
        qos: QosKind<SubscriberQos>,
        a_listener: Option<Box<dyn SubscriberListenerAsync + Send>>,
        mask: Vec<StatusKind>,
        runtime_handle: tokio::runtime::Handle,
    ) -> (
        ActorAddress<SubscriberActor>,
        ActorAddress<StatusConditionActor>,
    ) {
        let subscriber_qos = match qos {
            QosKind::Default => self.default_subscriber_qos.clone(),
            QosKind::Specific(q) => q,
        };
        let subcriber_counter = self.create_unique_subscriber_id();
        let entity_id = EntityId::new([subcriber_counter, 0, 0], USER_DEFINED_READER_GROUP);
        let guid = Guid::new(self.rtps_participant.guid().prefix(), entity_id);
        let rtps_group = RtpsGroup::new(guid);
        let status_kind = mask.to_vec();

        let subscriber = SubscriberActor::new(
            subscriber_qos,
            rtps_group,
            a_listener,
            status_kind,
            &runtime_handle,
        );

        let subscriber_status_condition = subscriber.get_statuscondition();

        let subscriber_actor = Actor::spawn(subscriber, &runtime_handle);
        let subscriber_address = subscriber_actor.address();

        self.user_defined_subscriber_list
            .insert(InstanceHandle::new(guid.into()), subscriber_actor);

        (subscriber_address, subscriber_status_condition)
    }

    async fn delete_user_defined_subscriber(&mut self, handle: InstanceHandle) -> DdsResult<()> {
        if let Some(subscriber) = self.user_defined_subscriber_list.get(&handle) {
            if !subscriber.is_empty().await {
                Err(DdsError::PreconditionNotMet(
                    "Subscriber still contains data readers".to_string(),
                ))
            } else {
                self.user_defined_subscriber_list.remove(&handle);
                Ok(())
            }
        } else {
            Err(DdsError::PreconditionNotMet(
                "Subscriber can only be deleted from its parent participant".to_string(),
            ))
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn create_user_defined_topic(
        &mut self,
        topic_name: String,
        type_name: String,
        qos: QosKind<TopicQos>,
        a_listener: Option<Box<dyn TopicListenerAsync + Send>>,
        _mask: Vec<StatusKind>,
        type_support: Arc<dyn DynamicTypeInterface + Send + Sync>,
        runtime_handle: tokio::runtime::Handle,
    ) -> (ActorAddress<TopicActor>, ActorAddress<StatusConditionActor>) {
        let qos = match qos {
            QosKind::Default => self.default_topic_qos.clone(),
            QosKind::Specific(q) => q,
        };
        let topic_counter = self.create_unique_topic_id();
        let entity_id = EntityId::new([topic_counter, 0, 0], USER_DEFINED_TOPIC);
        let guid = Guid::new(self.rtps_participant.guid().prefix(), entity_id);

        self.type_support_actor
            .register_type(type_name.clone(), type_support)
            .await;

        let topic = TopicActor::new(
            guid,
            qos,
            type_name,
            &topic_name,
            a_listener,
            &runtime_handle,
        );

        let topic_status_condition = topic.get_statuscondition();
        let topic_actor: crate::implementation::utils::actor::Actor<TopicActor> =
            Actor::spawn(topic, &runtime_handle);
        let topic_address = topic_actor.address();
        self.user_defined_topic_list
            .insert(InstanceHandle::new(guid.into()), topic_actor);

        (topic_address, topic_status_condition)
    }

    async fn delete_user_defined_topic(&mut self, handle: InstanceHandle) -> DdsResult<()> {
        if let Some(topic) = self.user_defined_topic_list.get(&handle) {
            let topic_name = topic.get_name().await;
            for publisher in self.user_defined_publisher_list.values() {
                if publisher
                    .lookup_datawriter(topic_name.clone())
                    .await
                    .is_some()
                {
                    return Err(DdsError::PreconditionNotMet(
                        "Topic still attached to some data writer".to_string(),
                    ));
                }
            }

            for subscriber in self.user_defined_subscriber_list.values() {
                if subscriber
                    .lookup_datareader(topic_name.clone())
                    .await
                    .is_some()
                {
                    return Err(DdsError::PreconditionNotMet(
                        "Topic still attached to some data reader".to_string(),
                    ));
                }
            }

            self.user_defined_topic_list.remove(&handle);
            Ok(())
        } else {
            Err(DdsError::PreconditionNotMet(
                "Topic can only be deleted from its parent participant".to_string(),
            ))
        }
    }

    async fn find_topic(
        &mut self,
        topic_name: String,
        type_support: Arc<dyn DynamicTypeInterface + Send + Sync>,
        runtime_handle: tokio::runtime::Handle,
    ) -> Option<(
        ActorAddress<TopicActor>,
        ActorAddress<StatusConditionActor>,
        String,
    )> {
        if let Some(r) = self.lookup_topicdescription(topic_name.clone()).await {
            Some(r)
        } else {
            self.lookup_discovered_topic(
                topic_name.clone(),
                type_support.clone(),
                runtime_handle.clone(),
            )
            .await
        }
    }

    async fn lookup_topicdescription(
        &self,
        topic_name: String,
    ) -> Option<(
        ActorAddress<TopicActor>,
        ActorAddress<StatusConditionActor>,
        String,
    )> {
        for topic in self
            .builtin_topic_list
            .iter()
            .chain(self.user_defined_topic_list.values())
        {
            if topic.get_name().await == topic_name {
                return Some((
                    topic.address(),
                    topic.get_statuscondition().await,
                    topic.get_type_name().await,
                ));
            }
        }
        None
    }

    fn get_instance_handle(&self) -> InstanceHandle {
        InstanceHandle::new(self.rtps_participant.guid().into())
    }

    #[allow(clippy::unused_unit)]
    fn enable(&mut self) -> () {
        self.enabled = true;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn ignore_participant(&mut self, handle: InstanceHandle) -> DdsResult<()> {
        if self.enabled {
            self.ignored_participants.insert(handle);
            Ok(())
        } else {
            Err(DdsError::NotEnabled)
        }
    }

    fn ignore_subscription(&mut self, handle: InstanceHandle) -> DdsResult<()> {
        if self.enabled {
            self.ignored_subcriptions.insert(handle);
            Ok(())
        } else {
            Err(DdsError::NotEnabled)
        }
    }

    fn ignore_publication(&mut self, handle: InstanceHandle) -> DdsResult<()> {
        if self.enabled {
            self.ignored_publications.insert(handle);
            Ok(())
        } else {
            Err(DdsError::NotEnabled)
        }
    }

    fn ignore_topic(&self, _handle: InstanceHandle) -> DdsResult<()> {
        todo!()
    }

    fn is_empty(&self) -> bool {
        self.user_defined_publisher_list.len() == 0
            && self.user_defined_subscriber_list.len() == 0
            && self.user_defined_topic_list.len() == 0
    }

    fn get_qos(&self) -> DomainParticipantQos {
        self.qos.clone()
    }

    fn get_default_unicast_locator_list(&self) -> Vec<Locator> {
        self.rtps_participant
            .default_unicast_locator_list()
            .to_vec()
    }

    fn get_default_multicast_locator_list(&self) -> Vec<Locator> {
        self.rtps_participant
            .default_multicast_locator_list()
            .to_vec()
    }

    fn get_metatraffic_unicast_locator_list(&self) -> Vec<Locator> {
        self.rtps_participant
            .metatraffic_unicast_locator_list()
            .to_vec()
    }

    fn get_metatraffic_multicast_locator_list(&self) -> Vec<Locator> {
        self.rtps_participant
            .metatraffic_multicast_locator_list()
            .to_vec()
    }

    fn data_max_size_serialized(&self) -> usize {
        self.data_max_size_serialized
    }

    async fn delete_contained_entities(&mut self) -> DdsResult<()> {
        for user_defined_publisher in self.user_defined_publisher_list.values() {
            let deleted_data_writer_handle_list =
                user_defined_publisher.delete_contained_entities().await;
            for writer_handle in deleted_data_writer_handle_list {
                self.announce_deleted_data_writer(writer_handle).await?;
            }
        }
        self.user_defined_publisher_list.clear();

        for user_defined_subscriber in self.user_defined_subscriber_list.values() {
            let deleted_data_reader_handle_list =
                user_defined_subscriber.delete_contained_entities().await;
            for reader_handle in deleted_data_reader_handle_list {
                self.announce_deleted_data_reader(reader_handle).await?;
            }
        }
        self.user_defined_subscriber_list.clear();

        self.user_defined_topic_list.clear();

        Ok(())
    }

    fn set_default_publisher_qos(&mut self, qos: QosKind<PublisherQos>) -> DdsResult<()> {
        let qos = match qos {
            QosKind::Default => PublisherQos::default(),
            QosKind::Specific(q) => q,
        };

        self.default_publisher_qos = qos;

        Ok(())
    }

    fn get_default_publisher_qos(&self) -> PublisherQos {
        self.default_publisher_qos.clone()
    }

    fn set_default_subscriber_qos(&mut self, qos: QosKind<SubscriberQos>) -> DdsResult<()> {
        let qos = match qos {
            QosKind::Default => SubscriberQos::default(),
            QosKind::Specific(q) => q,
        };

        self.default_subscriber_qos = qos;

        Ok(())
    }

    fn get_default_subscriber_qos(&self) -> SubscriberQos {
        self.default_subscriber_qos.clone()
    }

    fn set_default_topic_qos(&mut self, qos: QosKind<TopicQos>) -> DdsResult<()> {
        let qos = match qos {
            QosKind::Default => TopicQos::default(),
            QosKind::Specific(q) => {
                q.is_consistent()?;
                q
            }
        };

        self.default_topic_qos = qos;

        Ok(())
    }

    fn get_default_topic_qos(&self) -> TopicQos {
        self.default_topic_qos.clone()
    }

    fn get_discovered_participants(&self) -> Vec<InstanceHandle> {
        self.discovered_participant_list.keys().cloned().collect()
    }

    fn get_discovered_participant_data(
        &self,
        participant_handle: InstanceHandle,
    ) -> DdsResult<ParticipantBuiltinTopicData> {
        Ok(self
            .discovered_participant_list
            .get(&participant_handle)
            .ok_or(DdsError::PreconditionNotMet(
                "Participant with this instance handle not discovered".to_owned(),
            ))?
            .dds_participant_data()
            .clone())
    }

    fn get_discovered_topics(&self) -> Vec<InstanceHandle> {
        self.discovered_topic_list.keys().cloned().collect()
    }

    fn get_discovered_topic_data(
        &self,
        topic_handle: InstanceHandle,
    ) -> DdsResult<TopicBuiltinTopicData> {
        self.discovered_topic_list
            .get(&topic_handle)
            .cloned()
            .ok_or(DdsError::PreconditionNotMet(
                "Topic with this handle not discovered".to_owned(),
            ))
    }

    fn set_qos(&mut self, qos: DomainParticipantQos) -> DdsResult<()> {
        self.qos = qos;
        Ok(())
    }

    fn get_domain_id(&self) -> DomainId {
        self.domain_id
    }

    fn get_built_in_subscriber(&self) -> ActorAddress<SubscriberActor> {
        self.builtin_subscriber.address()
    }

    fn as_spdp_discovered_participant_data(&self) -> SpdpDiscoveredParticipantData {
        SpdpDiscoveredParticipantData::new(
            ParticipantBuiltinTopicData::new(
                BuiltInTopicKey {
                    value: self.rtps_participant.guid().into(),
                },
                self.qos.user_data.clone(),
            ),
            ParticipantProxy::new(
                Some(self.domain_id),
                self.domain_tag.clone(),
                self.rtps_participant.protocol_version(),
                self.rtps_participant.guid().prefix(),
                self.rtps_participant.vendor_id(),
                false,
                self.rtps_participant
                    .metatraffic_unicast_locator_list()
                    .to_vec(),
                self.rtps_participant
                    .metatraffic_multicast_locator_list()
                    .to_vec(),
                self.rtps_participant
                    .default_unicast_locator_list()
                    .to_vec(),
                self.rtps_participant
                    .default_multicast_locator_list()
                    .to_vec(),
                BuiltinEndpointSet::default(),
                self.manual_liveliness_count,
                BuiltinEndpointQos::default(),
            ),
            self.lease_duration,
        )
    }

    fn get_status_kind(&self) -> Vec<StatusKind> {
        self.status_kind.clone()
    }

    fn get_current_time(&self) -> infrastructure::time::Time {
        let now_system_time = SystemTime::now();
        let unix_time = now_system_time
            .duration_since(UNIX_EPOCH)
            .expect("Clock time is before Unix epoch start");
        infrastructure::time::Time::new(unix_time.as_secs() as i32, unix_time.subsec_nanos())
    }

    fn get_builtin_publisher(&self) -> ActorAddress<PublisherActor> {
        self.builtin_publisher.address()
    }

    async fn send_message(&self) {
        let now = self.get_current_time();
        let header = RtpsMessageHeader::new(
            self.rtps_participant.protocol_version(),
            self.rtps_participant.vendor_id(),
            self.rtps_participant.guid().prefix(),
        );
        self.builtin_publisher
            .send_message(header, self.udp_transport_write.clone(), now)
            .await;
        self.builtin_subscriber
            .send_message(header, self.udp_transport_write.clone())
            .await;

        for publisher in self.user_defined_publisher_list.values() {
            publisher
                .send_message(header, self.udp_transport_write.clone(), now)
                .await;
        }

        for subscriber in self.user_defined_subscriber_list.values() {
            subscriber
                .send_message(header, self.udp_transport_write.clone())
                .await;
        }
    }

    async fn process_metatraffic_rtps_message(
        &mut self,
        message: RtpsMessageRead,
        participant: DomainParticipantAsync,
    ) -> DdsResult<()> {
        tracing::trace!(
            rtps_message = ?message,
            "Received metatraffic RTPS message"
        );
        let reception_timestamp = self.get_current_time().into();
        let participant_mask_listener = (self.listener.address(), self.status_kind.clone());
        self.builtin_subscriber
            .process_rtps_message(
                message.clone(),
                reception_timestamp,
                self.builtin_subscriber.address(),
                participant.clone(),
                participant_mask_listener,
                self.type_support_actor.address(),
            )
            .await;

        self.builtin_publisher.process_rtps_message(message).await;

        self.process_builtin_discovery(participant).await;

        Ok(())
    }

    async fn process_user_defined_rtps_message(
        &self,
        message: RtpsMessageRead,
        participant: DomainParticipantAsync,
    ) {
        let participant_mask_listener = (self.listener.address(), self.status_kind.clone());
        for user_defined_subscriber_address in self
            .user_defined_subscriber_list
            .values()
            .map(|a| a.address())
        {
            user_defined_subscriber_address
                .process_rtps_message(
                    message.clone(),
                    self.get_current_time().into(),
                    user_defined_subscriber_address.clone(),
                    participant.clone(),
                    participant_mask_listener.clone(),
                    self.type_support_actor.address(),
                )
                .await
                .expect("Should not fail to send command");

            user_defined_subscriber_address
                .send_message(
                    RtpsMessageHeader::new(
                        self.rtps_participant.protocol_version(),
                        self.rtps_participant.vendor_id(),
                        self.rtps_participant.guid().prefix(),
                    ),
                    self.udp_transport_write.clone().clone(),
                )
                .await
                .expect("Should not fail to send command");
        }

        for user_defined_publisher_address in self
            .user_defined_publisher_list
            .values()
            .map(|a| a.address())
        {
            user_defined_publisher_address
                .process_rtps_message(message.clone())
                .await
                .expect("Should not fail to send command");
            user_defined_publisher_address
                .send_message(
                    RtpsMessageHeader::new(
                        self.rtps_participant.protocol_version(),
                        self.rtps_participant.vendor_id(),
                        self.rtps_participant.guid().prefix(),
                    ),
                    self.udp_transport_write.clone(),
                    self.get_current_time(),
                )
                .await
                .expect("Should not fail to send command");
        }
    }

    async fn announce_created_or_modified_data_writer(
        &self,
        discovered_writer_data: DiscoveredWriterData,
    ) {
        if let Some(sedp_publications_announcer) = self
            .builtin_publisher
            .lookup_datawriter(DCPS_PUBLICATION.to_string())
            .await
        {
            let timestamp = self.get_current_time();
            let mut serialized_data = Vec::new();
            discovered_writer_data
                .serialize_data(&mut serialized_data)
                .expect("Shouldn't fail to serialize builtin type");
            let instance_handle =
                get_instance_handle_from_key(&discovered_writer_data.get_key().unwrap())
                    .expect("Shouldn't fail to serialize key of builtin type");
            sedp_publications_announcer
                .write_w_timestamp(serialized_data, instance_handle, None, timestamp)
                .await
                .expect("Shouldn't fail to send to built-in data writer")
                .expect("Shouldn't fail to write to built-in data writer");

            self.send_message().await;
        }
    }

    async fn announce_created_or_modified_data_reader(
        &self,
        discovered_reader_data: DiscoveredReaderData,
    ) {
        if let Some(sedp_subscriptions_announcer) = self
            .builtin_publisher
            .lookup_datawriter(DCPS_SUBSCRIPTION.to_string())
            .await
        {
            let timestamp = self.get_current_time();
            let mut serialized_data = Vec::new();
            discovered_reader_data
                .serialize_data(&mut serialized_data)
                .expect("Shouldn't fail to serialize builtin type");
            let instance_handle =
                get_instance_handle_from_key(&discovered_reader_data.get_key().unwrap())
                    .expect("Shouldn't fail to serialize key of builtin type");
            sedp_subscriptions_announcer
                .write_w_timestamp(serialized_data, instance_handle, None, timestamp)
                .await
                .expect("Shouldn't fail to send to built-in data writer")
                .expect("Shouldn't fail to write to built-in data writer");

            self.send_message().await;
        }
    }

    async fn announce_deleted_data_writer(&self, writer_handle: InstanceHandle) -> DdsResult<()> {
        if let Some(sedp_publications_announcer) = self
            .builtin_publisher
            .lookup_datawriter(DCPS_PUBLICATION.to_string())
            .await
        {
            let timestamp = self.get_current_time();
            let mut instance_serialized_key = Vec::new();
            serialize_rtps_classic_cdr_le(writer_handle.as_ref(), &mut instance_serialized_key)
                .expect("Failed to serialize data");

            sedp_publications_announcer
                .dispose_w_timestamp(instance_serialized_key, writer_handle, timestamp)
                .await??;

            self.send_message().await;

            Ok(())
        } else {
            Ok(())
        }
    }

    #[allow(clippy::unused_unit)]
    fn set_listener(
        &mut self,
        listener: Option<Box<dyn DomainParticipantListenerAsync + Send>>,
        status_kind: Vec<StatusKind>,
        runtime_handle: tokio::runtime::Handle,
    ) -> () {
        self.listener = Actor::spawn(
            DomainParticipantListenerActor::new(listener),
            &runtime_handle,
        );
        self.status_kind = status_kind;
    }

    async fn announce_deleted_data_reader(&self, reader_handle: InstanceHandle) -> DdsResult<()> {
        if let Some(sedp_subscriptions_announcer) = self
            .builtin_publisher
            .lookup_datawriter(DCPS_SUBSCRIPTION.to_string())
            .await
        {
            let timestamp = self.get_current_time();
            let mut instance_serialized_key = Vec::new();
            serialize_rtps_classic_cdr_le(reader_handle.as_ref(), &mut instance_serialized_key)
                .expect("Failed to serialize data");
            sedp_subscriptions_announcer
                .dispose_w_timestamp(instance_serialized_key, reader_handle, timestamp)
                .await??;

            self.send_message().await;

            Ok(())
        } else {
            Ok(())
        }
    }

    async fn get_type_support(
        &mut self,
        type_name: String,
    ) -> Option<Arc<dyn DynamicTypeInterface + Send + Sync>> {
        self.type_support_actor.get_type_support(type_name).await
    }

    fn get_statuscondition(&self) -> ActorAddress<StatusConditionActor> {
        self.status_condition.address()
    }
}

impl DomainParticipantActor {
    fn create_unique_publisher_id(&mut self) -> u8 {
        let counter = self.user_defined_publisher_counter;
        self.user_defined_publisher_counter += 1;
        counter
    }

    fn create_unique_subscriber_id(&mut self) -> u8 {
        let counter = self.user_defined_subscriber_counter;
        self.user_defined_subscriber_counter += 1;
        counter
    }

    fn create_unique_topic_id(&mut self) -> u8 {
        let counter = self.user_defined_topic_counter;
        self.user_defined_topic_counter += 1;
        counter
    }

    async fn process_builtin_discovery(&mut self, participant: DomainParticipantAsync) {
        self.process_spdp_participant_discovery().await;
        self.process_sedp_publications_discovery(participant.clone())
            .await;
        self.process_sedp_subscriptions_discovery(participant.clone())
            .await;
        self.process_sedp_topics_discovery().await;
    }

    async fn process_spdp_participant_discovery(&mut self) {
        if let Some(spdp_participant_reader) = self
            .builtin_subscriber
            .lookup_datareader(DCPS_PARTICIPANT.to_string())
            .await
        {
            if let Ok(spdp_data_sample_list) = spdp_participant_reader
                .read(
                    i32::MAX,
                    vec![SampleStateKind::NotRead],
                    ANY_VIEW_STATE.to_vec(),
                    ANY_INSTANCE_STATE.to_vec(),
                    None,
                )
                .await
                .expect("Can not fail to send mail to builtin reader")
            {
                for (spdp_data_sample, spdp_sample_info) in spdp_data_sample_list {
                    if let Some(spdp_data) = spdp_data_sample.as_ref() {
                        match SpdpDiscoveredParticipantData::deserialize_data(spdp_data.as_ref()) {
                            Ok(discovered_participant_data) => {
                                self.process_discovered_participant_data(
                                    discovered_participant_data,
                                )
                                .await
                            }
                            Err(e) => warn!(
                                "Received invalid SpdpDiscoveredParticipantData. Error {:?}",
                                e
                            ),
                        }
                    } else {
                        warn!(
                            "Received empty sample on spdp. Sample info: {:?}",
                            spdp_sample_info
                        )
                    }
                }
            }
        }
    }

    async fn process_discovered_participant_data(
        &mut self,
        discovered_participant_data: SpdpDiscoveredParticipantData,
    ) {
        // Check that the domainId of the discovered participant equals the local one.
        // If it is not equal then there the local endpoints are not configured to
        // communicate with the discovered participant.
        // AND
        // Check that the domainTag of the discovered participant equals the local one.
        // If it is not equal then there the local endpoints are not configured to
        // communicate with the discovered participant.
        // IN CASE no domain id was transmitted the a local domain id is assumed
        // (as specified in Table 9.19 - ParameterId mapping and default values)
        let is_domain_id_matching = discovered_participant_data
            .participant_proxy()
            .domain_id()
            .unwrap_or(self.domain_id)
            == self.domain_id;
        let is_domain_tag_matching =
            discovered_participant_data.participant_proxy().domain_tag() == self.domain_tag;
        let discovered_participant_handle = InstanceHandle::new(
            discovered_participant_data
                .dds_participant_data()
                .key()
                .value,
        );
        let is_participant_ignored = self
            .ignored_participants
            .contains(&discovered_participant_handle);
        if is_domain_id_matching && is_domain_tag_matching && !is_participant_ignored {
            self.add_matched_publications_detector(&discovered_participant_data)
                .await;
            self.add_matched_publications_announcer(&discovered_participant_data)
                .await;
            self.add_matched_subscriptions_detector(&discovered_participant_data)
                .await;
            self.add_matched_subscriptions_announcer(&discovered_participant_data)
                .await;
            self.add_matched_topics_detector(&discovered_participant_data)
                .await;
            self.add_matched_topics_announcer(&discovered_participant_data)
                .await;

            self.discovered_participant_list.insert(
                InstanceHandle::new(
                    discovered_participant_data
                        .dds_participant_data()
                        .key()
                        .value,
                ),
                discovered_participant_data,
            );
        }
    }

    async fn add_matched_publications_detector(
        &self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if let Some(sedp_publications_announcer) = self
            .builtin_publisher
            .lookup_datawriter(DCPS_PUBLICATION.to_string())
            .await
        {
            if discovered_participant_data
                .participant_proxy()
                .available_builtin_endpoints()
                .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_PUBLICATIONS_DETECTOR)
            {
                let remote_reader_guid = Guid::new(
                    discovered_participant_data
                        .participant_proxy()
                        .guid_prefix(),
                    ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR,
                );
                let remote_group_entity_id = ENTITYID_UNKNOWN;
                let expects_inline_qos = false;
                let proxy = RtpsReaderProxy::new(
                    remote_reader_guid,
                    remote_group_entity_id,
                    discovered_participant_data
                        .participant_proxy()
                        .metatraffic_unicast_locator_list(),
                    discovered_participant_data
                        .participant_proxy()
                        .metatraffic_multicast_locator_list(),
                    expects_inline_qos,
                    true,
                    ReliabilityKind::Reliable,
                    0,
                );
                sedp_publications_announcer
                    .matched_reader_add(proxy)
                    .await
                    .unwrap();
            }
        }
    }

    async fn add_matched_publications_announcer(
        &self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if let Some(sedp_publications_detector) = self
            .builtin_subscriber
            .lookup_datareader(DCPS_PUBLICATION.to_string())
            .await
        {
            if discovered_participant_data
                .participant_proxy()
                .available_builtin_endpoints()
                .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_PUBLICATIONS_ANNOUNCER)
            {
                let remote_writer_guid = Guid::new(
                    discovered_participant_data
                        .participant_proxy()
                        .guid_prefix(),
                    ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER,
                );
                let remote_group_entity_id = ENTITYID_UNKNOWN;
                let data_max_size_serialized = None;

                let proxy = RtpsWriterProxy::new(
                    remote_writer_guid,
                    discovered_participant_data
                        .participant_proxy()
                        .metatraffic_unicast_locator_list(),
                    discovered_participant_data
                        .participant_proxy()
                        .metatraffic_multicast_locator_list(),
                    data_max_size_serialized,
                    remote_group_entity_id,
                );

                sedp_publications_detector
                    .matched_writer_add(proxy)
                    .await
                    .unwrap();
            }
        }
    }

    async fn add_matched_subscriptions_detector(
        &self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if let Some(sedp_subscriptions_announcer) = self
            .builtin_publisher
            .lookup_datawriter(DCPS_SUBSCRIPTION.to_string())
            .await
        {
            if discovered_participant_data
                .participant_proxy()
                .available_builtin_endpoints()
                .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_SUBSCRIPTIONS_DETECTOR)
            {
                let remote_reader_guid = Guid::new(
                    discovered_participant_data
                        .participant_proxy()
                        .guid_prefix(),
                    ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
                );
                let remote_group_entity_id = ENTITYID_UNKNOWN;
                let expects_inline_qos = false;
                let proxy = RtpsReaderProxy::new(
                    remote_reader_guid,
                    remote_group_entity_id,
                    discovered_participant_data
                        .participant_proxy()
                        .metatraffic_unicast_locator_list(),
                    discovered_participant_data
                        .participant_proxy()
                        .metatraffic_multicast_locator_list(),
                    expects_inline_qos,
                    true,
                    ReliabilityKind::Reliable,
                    0,
                );
                sedp_subscriptions_announcer
                    .matched_reader_add(proxy)
                    .await
                    .unwrap();
            }
        }
    }

    async fn add_matched_subscriptions_announcer(
        &self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if let Some(sedp_subscriptions_detector) = self
            .builtin_subscriber
            .lookup_datareader(DCPS_SUBSCRIPTION.to_string())
            .await
        {
            if discovered_participant_data
                .participant_proxy()
                .available_builtin_endpoints()
                .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_SUBSCRIPTIONS_ANNOUNCER)
            {
                let remote_writer_guid = Guid::new(
                    discovered_participant_data
                        .participant_proxy()
                        .guid_prefix(),
                    ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER,
                );
                let remote_group_entity_id = ENTITYID_UNKNOWN;
                let data_max_size_serialized = None;

                let proxy = RtpsWriterProxy::new(
                    remote_writer_guid,
                    discovered_participant_data
                        .participant_proxy()
                        .metatraffic_unicast_locator_list(),
                    discovered_participant_data
                        .participant_proxy()
                        .metatraffic_multicast_locator_list(),
                    data_max_size_serialized,
                    remote_group_entity_id,
                );
                sedp_subscriptions_detector
                    .matched_writer_add(proxy)
                    .await
                    .unwrap();
            }
        }
    }

    async fn add_matched_topics_detector(
        &self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if let Some(sedp_topics_announcer) = self
            .builtin_publisher
            .lookup_datawriter(DCPS_TOPIC.to_string())
            .await
        {
            if discovered_participant_data
                .participant_proxy()
                .available_builtin_endpoints()
                .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_TOPICS_DETECTOR)
            {
                let remote_reader_guid = Guid::new(
                    discovered_participant_data
                        .participant_proxy()
                        .guid_prefix(),
                    ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR,
                );
                let remote_group_entity_id = ENTITYID_UNKNOWN;
                let expects_inline_qos = false;
                let proxy = RtpsReaderProxy::new(
                    remote_reader_guid,
                    remote_group_entity_id,
                    discovered_participant_data
                        .participant_proxy()
                        .metatraffic_unicast_locator_list(),
                    discovered_participant_data
                        .participant_proxy()
                        .metatraffic_multicast_locator_list(),
                    expects_inline_qos,
                    true,
                    ReliabilityKind::Reliable,
                    0,
                );
                sedp_topics_announcer
                    .matched_reader_add(proxy)
                    .await
                    .unwrap();
            }
        }
    }

    async fn add_matched_topics_announcer(
        &self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if let Some(sedp_topics_detector) = self
            .builtin_subscriber
            .lookup_datareader(DCPS_TOPIC.to_string())
            .await
        {
            if discovered_participant_data
                .participant_proxy()
                .available_builtin_endpoints()
                .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_TOPICS_ANNOUNCER)
            {
                let remote_writer_guid = Guid::new(
                    discovered_participant_data
                        .participant_proxy()
                        .guid_prefix(),
                    ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER,
                );
                let remote_group_entity_id = ENTITYID_UNKNOWN;
                let data_max_size_serialized = None;

                let proxy = RtpsWriterProxy::new(
                    remote_writer_guid,
                    discovered_participant_data
                        .participant_proxy()
                        .metatraffic_unicast_locator_list(),
                    discovered_participant_data
                        .participant_proxy()
                        .metatraffic_multicast_locator_list(),
                    data_max_size_serialized,
                    remote_group_entity_id,
                );
                sedp_topics_detector
                    .matched_writer_add(proxy)
                    .await
                    .unwrap();
            }
        }
    }

    async fn process_sedp_publications_discovery(&mut self, participant: DomainParticipantAsync) {
        if let Some(sedp_publications_detector) = self
            .builtin_subscriber
            .lookup_datareader(DCPS_PUBLICATION.to_string())
            .await
        {
            if let Ok(mut discovered_writer_sample_list) = sedp_publications_detector
                .read(
                    i32::MAX,
                    ANY_SAMPLE_STATE.to_vec(),
                    ANY_VIEW_STATE.to_vec(),
                    ANY_INSTANCE_STATE.to_vec(),
                    None,
                )
                .await
                .expect("Can not fail to send mail to builtin reader")
            {
                for (discovered_writer_data, discovered_writer_sample_info) in
                    discovered_writer_sample_list.drain(..)
                {
                    match discovered_writer_sample_info.instance_state {
                        InstanceStateKind::Alive => {
                            match DiscoveredWriterData::deserialize_data(
                                discovered_writer_data
                                    .expect("Should contain data")
                                    .as_ref(),
                            ) {
                                Ok(discovered_writer_data) => {
                                    self.add_matched_writer(
                                        discovered_writer_data,
                                        participant.clone(),
                                    )
                                    .await;
                                }
                                Err(e) => warn!(
                                    "Received invalid DiscoveredWriterData sample. Error {:?}",
                                    e
                                ),
                            }
                        }
                        InstanceStateKind::NotAliveDisposed => {
                            self.remove_matched_writer(
                                discovered_writer_sample_info.instance_handle,
                                participant.clone(),
                            )
                            .await
                        }
                        InstanceStateKind::NotAliveNoWriters => {
                            todo!()
                        }
                    }
                }
            }
        }
    }

    async fn add_matched_writer(
        &mut self,
        discovered_writer_data: DiscoveredWriterData,
        participant: DomainParticipantAsync,
    ) {
        let is_participant_ignored = self.ignored_participants.contains(&InstanceHandle::new(
            Guid::new(
                discovered_writer_data
                    .writer_proxy()
                    .remote_writer_guid()
                    .prefix(),
                ENTITYID_PARTICIPANT,
            )
            .into(),
        ));
        let is_publication_ignored = self.ignored_publications.contains(&InstanceHandle::new(
            discovered_writer_data.dds_publication_data().key().value,
        ));
        if !is_publication_ignored && !is_participant_ignored {
            if let Some(discovered_participant_data) =
                self.discovered_participant_list.get(&InstanceHandle::new(
                    Guid::new(
                        discovered_writer_data
                            .writer_proxy()
                            .remote_writer_guid()
                            .prefix(),
                        ENTITYID_PARTICIPANT,
                    )
                    .into(),
                ))
            {
                let default_unicast_locator_list = discovered_participant_data
                    .participant_proxy()
                    .default_unicast_locator_list()
                    .to_vec();
                let default_multicast_locator_list = discovered_participant_data
                    .participant_proxy()
                    .default_multicast_locator_list()
                    .to_vec();
                for subscriber in self.user_defined_subscriber_list.values() {
                    let subscriber_address = subscriber.address();
                    let participant_mask_listener =
                        (self.listener.address(), self.status_kind.clone());
                    subscriber
                        .add_matched_writer(
                            discovered_writer_data.clone(),
                            default_unicast_locator_list.clone(),
                            default_multicast_locator_list.clone(),
                            subscriber_address,
                            participant.clone(),
                            participant_mask_listener,
                        )
                        .await;
                }

                // Add writer topic to discovered topic list using the writer instance handle
                let topic_instance_handle =
                    InstanceHandle::new(discovered_writer_data.dds_publication_data().key().value);
                let writer_topic = TopicBuiltinTopicData::new(
                    BuiltInTopicKey::default(),
                    discovered_writer_data
                        .dds_publication_data()
                        .topic_name()
                        .to_string(),
                    discovered_writer_data
                        .dds_publication_data()
                        .get_type_name()
                        .to_string(),
                    discovered_writer_data
                        .dds_publication_data()
                        .durability()
                        .clone(),
                    discovered_writer_data
                        .dds_publication_data()
                        .deadline()
                        .clone(),
                    discovered_writer_data
                        .dds_publication_data()
                        .latency_budget()
                        .clone(),
                    discovered_writer_data
                        .dds_publication_data()
                        .liveliness()
                        .clone(),
                    discovered_writer_data
                        .dds_publication_data()
                        .reliability()
                        .clone(),
                    TransportPriorityQosPolicy::default(),
                    discovered_writer_data
                        .dds_publication_data()
                        .lifespan()
                        .clone(),
                    discovered_writer_data
                        .dds_publication_data()
                        .destination_order()
                        .clone(),
                    HistoryQosPolicy::default(),
                    ResourceLimitsQosPolicy::default(),
                    discovered_writer_data
                        .dds_publication_data()
                        .ownership()
                        .clone(),
                    discovered_writer_data
                        .dds_publication_data()
                        .topic_data()
                        .clone(),
                );
                self.discovered_topic_list
                    .insert(topic_instance_handle, writer_topic);
            }
        }
    }

    async fn remove_matched_writer(
        &self,
        discovered_writer_handle: InstanceHandle,
        participant: DomainParticipantAsync,
    ) {
        for subscriber in self.user_defined_subscriber_list.values() {
            let subscriber_address = subscriber.address();
            let participant_mask_listener = (self.listener.address(), self.status_kind.clone());
            subscriber
                .remove_matched_writer(
                    discovered_writer_handle,
                    subscriber_address,
                    participant.clone(),
                    participant_mask_listener,
                )
                .await;
        }
    }

    async fn process_sedp_subscriptions_discovery(&mut self, participant: DomainParticipantAsync) {
        if let Some(sedp_subscriptions_detector) = self
            .builtin_subscriber
            .lookup_datareader(DCPS_SUBSCRIPTION.to_string())
            .await
        {
            if let Ok(mut discovered_reader_sample_list) = sedp_subscriptions_detector
                .read(
                    i32::MAX,
                    ANY_SAMPLE_STATE.to_vec(),
                    ANY_VIEW_STATE.to_vec(),
                    ANY_INSTANCE_STATE.to_vec(),
                    None,
                )
                .await
                .expect("Can not fail to send mail to builtin reader")
            {
                for (discovered_reader_data, discovered_reader_sample_info) in
                    discovered_reader_sample_list.drain(..)
                {
                    match discovered_reader_sample_info.instance_state {
                        InstanceStateKind::Alive => {
                            match DiscoveredReaderData::deserialize_data(
                                discovered_reader_data
                                    .expect("Should contain data")
                                    .as_ref(),
                            ) {
                                Ok(discovered_reader_data) => {
                                    self.add_matched_reader(
                                        discovered_reader_data,
                                        participant.clone(),
                                    )
                                    .await;
                                }
                                Err(e) => warn!(
                                    "Received invalid DiscoveredReaderData sample. Error {:?}",
                                    e
                                ),
                            }
                        }
                        InstanceStateKind::NotAliveDisposed => {
                            self.remove_matched_reader(
                                discovered_reader_sample_info.instance_handle,
                                participant.clone(),
                            )
                            .await
                        }
                        InstanceStateKind::NotAliveNoWriters => {
                            todo!()
                        }
                    }
                }
            }
        }
    }

    async fn add_matched_reader(
        &mut self,
        discovered_reader_data: DiscoveredReaderData,
        participant: DomainParticipantAsync,
    ) {
        let is_participant_ignored = self.ignored_participants.contains(&InstanceHandle::new(
            Guid::new(
                discovered_reader_data
                    .reader_proxy()
                    .remote_reader_guid()
                    .prefix(),
                ENTITYID_PARTICIPANT,
            )
            .into(),
        ));
        let is_subscription_ignored = self.ignored_subcriptions.contains(&InstanceHandle::new(
            discovered_reader_data
                .subscription_builtin_topic_data()
                .key()
                .value,
        ));
        if !is_subscription_ignored && !is_participant_ignored {
            if let Some(discovered_participant_data) =
                self.discovered_participant_list.get(&InstanceHandle::new(
                    Guid::new(
                        discovered_reader_data
                            .reader_proxy()
                            .remote_reader_guid()
                            .prefix(),
                        ENTITYID_PARTICIPANT,
                    )
                    .into(),
                ))
            {
                let default_unicast_locator_list = discovered_participant_data
                    .participant_proxy()
                    .default_unicast_locator_list()
                    .to_vec();
                let default_multicast_locator_list = discovered_participant_data
                    .participant_proxy()
                    .default_multicast_locator_list()
                    .to_vec();

                for publisher in self.user_defined_publisher_list.values() {
                    let publisher_address = publisher.address();

                    let participant_publication_matched_listener =
                        if self.status_kind.contains(&StatusKind::PublicationMatched) {
                            Some(self.listener.address())
                        } else {
                            None
                        };
                    let offered_incompatible_qos_participant_listener = if self
                        .status_kind
                        .contains(&StatusKind::OfferedIncompatibleQos)
                    {
                        Some(self.listener.address())
                    } else {
                        None
                    };
                    publisher
                        .add_matched_reader(
                            discovered_reader_data.clone(),
                            default_unicast_locator_list.clone(),
                            default_multicast_locator_list.clone(),
                            publisher_address,
                            participant.clone(),
                            participant_publication_matched_listener,
                            offered_incompatible_qos_participant_listener,
                        )
                        .await;
                }

                // Add reader topic to discovered topic list using the reader instance handle
                let topic_instance_handle = InstanceHandle::new(
                    discovered_reader_data
                        .subscription_builtin_topic_data()
                        .key()
                        .value,
                );
                let reader_topic = TopicBuiltinTopicData::new(
                    BuiltInTopicKey::default(),
                    discovered_reader_data
                        .subscription_builtin_topic_data()
                        .topic_name()
                        .to_string(),
                    discovered_reader_data
                        .subscription_builtin_topic_data()
                        .get_type_name()
                        .to_string(),
                    discovered_reader_data
                        .subscription_builtin_topic_data()
                        .durability()
                        .clone(),
                    discovered_reader_data
                        .subscription_builtin_topic_data()
                        .deadline()
                        .clone(),
                    discovered_reader_data
                        .subscription_builtin_topic_data()
                        .latency_budget()
                        .clone(),
                    discovered_reader_data
                        .subscription_builtin_topic_data()
                        .liveliness()
                        .clone(),
                    discovered_reader_data
                        .subscription_builtin_topic_data()
                        .reliability()
                        .clone(),
                    TransportPriorityQosPolicy::default(),
                    LifespanQosPolicy::default(),
                    discovered_reader_data
                        .subscription_builtin_topic_data()
                        .destination_order()
                        .clone(),
                    HistoryQosPolicy::default(),
                    ResourceLimitsQosPolicy::default(),
                    discovered_reader_data
                        .subscription_builtin_topic_data()
                        .ownership()
                        .clone(),
                    discovered_reader_data
                        .subscription_builtin_topic_data()
                        .topic_data()
                        .clone(),
                );
                self.discovered_topic_list
                    .insert(topic_instance_handle, reader_topic);
            }
        }
    }

    async fn remove_matched_reader(
        &self,
        discovered_reader_handle: InstanceHandle,
        participant: DomainParticipantAsync,
    ) {
        for publisher in self.user_defined_publisher_list.values() {
            let publisher_address = publisher.address();
            let participant_publication_matched_listener =
                if self.status_kind.contains(&StatusKind::PublicationMatched) {
                    Some(self.listener.address())
                } else {
                    None
                };
            publisher
                .remove_matched_reader(
                    discovered_reader_handle,
                    publisher_address,
                    participant.clone(),
                    participant_publication_matched_listener,
                )
                .await;
        }
    }

    async fn process_sedp_topics_discovery(&mut self) {
        if let Some(sedp_topics_detector) = self
            .builtin_subscriber
            .lookup_datareader(DCPS_TOPIC.to_string())
            .await
        {
            if let Ok(mut discovered_topic_sample_list) = sedp_topics_detector
                .read(
                    i32::MAX,
                    ANY_SAMPLE_STATE.to_vec(),
                    ANY_VIEW_STATE.to_vec(),
                    ANY_INSTANCE_STATE.to_vec(),
                    None,
                )
                .await
                .expect("Can not fail to send mail to builtin reader")
            {
                for (discovered_topic_data, discovered_topic_sample_info) in
                    discovered_topic_sample_list.drain(..)
                {
                    match discovered_topic_sample_info.instance_state {
                        InstanceStateKind::Alive => {
                            match DiscoveredTopicData::deserialize_data(
                                discovered_topic_data.expect("Should contain data").as_ref(),
                            ) {
                                Ok(discovered_topic_data) => {
                                    self.add_matched_topic(discovered_topic_data).await;
                                }
                                Err(e) => warn!(
                                    "Received invalid DiscoveredTopicData sample. Error {:?}",
                                    e
                                ),
                            }
                        }
                        InstanceStateKind::NotAliveDisposed => todo!(),
                        InstanceStateKind::NotAliveNoWriters => todo!(),
                    }
                }
            }
        }
    }

    async fn add_matched_topic(&mut self, discovered_topic_data: DiscoveredTopicData) {
        let handle =
            InstanceHandle::new(discovered_topic_data.topic_builtin_topic_data().key().value);
        let is_topic_ignored = self.ignored_topic_list.contains(&handle);
        if !is_topic_ignored {
            for topic in self.user_defined_topic_list.values() {
                topic
                    .process_discovered_topic(discovered_topic_data.clone())
                    .await;
            }
            self.discovered_topic_list.insert(
                handle,
                discovered_topic_data.topic_builtin_topic_data().clone(),
            );
        }
    }
}

fn create_builtin_stateful_writer(guid: Guid) -> RtpsWriter {
    let unicast_locator_list = &[];
    let multicast_locator_list = &[];
    let topic_kind = TopicKind::WithKey;
    let push_mode = true;
    let heartbeat_period = DEFAULT_HEARTBEAT_PERIOD;
    let nack_response_delay = DEFAULT_NACK_RESPONSE_DELAY;
    let nack_suppression_duration = DEFAULT_NACK_SUPPRESSION_DURATION;
    let data_max_size_serialized = usize::MAX;

    RtpsWriter::new(
        RtpsEndpoint::new(
            guid,
            topic_kind,
            unicast_locator_list,
            multicast_locator_list,
        ),
        push_mode,
        heartbeat_period,
        nack_response_delay,
        nack_suppression_duration,
        data_max_size_serialized,
    )
}

fn create_builtin_stateless_writer(guid: Guid) -> RtpsWriter {
    let unicast_locator_list = &[];
    let multicast_locator_list = &[];

    RtpsWriter::new(
        RtpsEndpoint::new(
            guid,
            TopicKind::WithKey,
            unicast_locator_list,
            multicast_locator_list,
        ),
        true,
        DURATION_ZERO,
        DURATION_ZERO,
        DURATION_ZERO,
        usize::MAX,
    )
}

fn create_builtin_stateless_reader(guid: Guid) -> RtpsReaderKind {
    let unicast_locator_list = &[];
    let multicast_locator_list = &[];

    RtpsReaderKind::Stateless(RtpsStatelessReader::new(RtpsReader::new(
        RtpsEndpoint::new(
            guid,
            TopicKind::WithKey,
            unicast_locator_list,
            multicast_locator_list,
        ),
        DURATION_ZERO,
        DURATION_ZERO,
        false,
    )))
}

fn create_builtin_stateful_reader(guid: Guid) -> RtpsReaderKind {
    let topic_kind = TopicKind::WithKey;
    let heartbeat_response_delay = DEFAULT_HEARTBEAT_RESPONSE_DELAY;
    let heartbeat_suppression_duration = DEFAULT_HEARTBEAT_SUPPRESSION_DURATION;
    let expects_inline_qos = false;
    let unicast_locator_list = &[];
    let multicast_locator_list = &[];

    RtpsReaderKind::Stateful(RtpsStatefulReader::new(RtpsReader::new(
        RtpsEndpoint::new(
            guid,
            topic_kind,
            unicast_locator_list,
            multicast_locator_list,
        ),
        heartbeat_response_delay,
        heartbeat_suppression_duration,
        expects_inline_qos,
    )))
}
