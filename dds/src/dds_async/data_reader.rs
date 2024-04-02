use crate::{
    builtin_topics::PublicationBuiltinTopicData,
    implementation::{
        actors::{
            any_data_reader_listener::AnyDataReaderListener, data_reader_actor::DataReaderActor,
            domain_participant_actor::DomainParticipantActor,
            status_condition_actor::StatusConditionActor, subscriber_actor::SubscriberActor,
        },
        utils::actor::ActorAddress,
    },
    infrastructure::{
        error::{DdsError, DdsResult},
        instance::InstanceHandle,
        qos::{DataReaderQos, QosKind, TopicQos},
        status::{
            LivelinessChangedStatus, RequestedDeadlineMissedStatus, RequestedIncompatibleQosStatus,
            SampleLostStatus, SampleRejectedStatus, StatusKind, SubscriptionMatchedStatus,
        },
        time::Duration,
    },
    subscription::{
        data_reader::Sample,
        sample_info::{
            InstanceStateKind, SampleStateKind, ViewStateKind, ANY_INSTANCE_STATE, ANY_VIEW_STATE,
        },
    },
};

use std::marker::PhantomData;

use super::{
    condition::StatusConditionAsync, data_reader_listener::DataReaderListenerAsync,
    subscriber::SubscriberAsync, topic::TopicAsync,
};

/// Async version of [`DataReader`](crate::subscription::data_reader::DataReader).
pub struct DataReaderAsync<Foo> {
    reader_address: ActorAddress<DataReaderActor>,
    status_condition_address: ActorAddress<StatusConditionActor>,
    subscriber: SubscriberAsync,
    topic: TopicAsync,
    phantom: PhantomData<Foo>,
}

impl<Foo> DataReaderAsync<Foo> {
    pub(crate) fn new(
        reader_address: ActorAddress<DataReaderActor>,
        status_condition_address: ActorAddress<StatusConditionActor>,
        subscriber: SubscriberAsync,
        topic: TopicAsync,
    ) -> Self {
        Self {
            reader_address,
            status_condition_address,
            subscriber,
            topic,
            phantom: PhantomData,
        }
    }

    pub(crate) fn participant_address(&self) -> &ActorAddress<DomainParticipantActor> {
        self.subscriber.participant_address()
    }

    pub(crate) fn subscriber_address(&self) -> &ActorAddress<SubscriberActor> {
        self.subscriber.subscriber_address()
    }

    pub(crate) fn runtime_handle(&self) -> &tokio::runtime::Handle {
        self.subscriber.runtime_handle()
    }

    async fn announce_reader(&self) -> DdsResult<()> {
        let type_name = self.reader_address.get_type_name().await?;
        let type_support = self
            .participant_address()
            .get_type_support(type_name.clone())
            .await?
            .ok_or_else(|| {
                DdsError::PreconditionNotMet(format!(
                    "Type with name {} not registered with parent domain participant",
                    type_name
                ))
            })?;
        let discovered_reader_data = self
            .reader_address
            .as_discovered_reader_data(
                TopicQos::default(),
                self.subscriber_address().get_qos().await?,
                self.participant_address()
                    .get_default_unicast_locator_list()
                    .await?,
                self.participant_address()
                    .get_default_multicast_locator_list()
                    .await?,
                type_support.xml_type(),
            )
            .await?;
        self.participant_address()
            .announce_created_or_modified_data_reader(discovered_reader_data)
            .await
    }
}

impl<Foo> Clone for DataReaderAsync<Foo> {
    fn clone(&self) -> Self {
        Self {
            reader_address: self.reader_address.clone(),
            status_condition_address: self.status_condition_address.clone(),
            subscriber: self.subscriber.clone(),
            topic: self.topic.clone(),
            phantom: self.phantom,
        }
    }
}

impl<Foo> DataReaderAsync<Foo> {
    /// Async version of [`read`](crate::subscription::data_reader::DataReader::read).
    #[tracing::instrument(skip(self))]
    pub async fn read(
        &self,
        max_samples: i32,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
    ) -> DdsResult<Vec<Sample<Foo>>> {
        let samples = self
            .reader_address
            .read(
                max_samples,
                sample_states.to_vec(),
                view_states.to_vec(),
                instance_states.to_vec(),
                None,
            )
            .await??;

        Ok(samples
            .into_iter()
            .map(|(data, sample_info)| Sample::new(data, sample_info))
            .collect())
    }

    /// Async version of [`take`](crate::subscription::data_reader::DataReader::take).
    #[tracing::instrument(skip(self))]
    pub async fn take(
        &self,
        max_samples: i32,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
    ) -> DdsResult<Vec<Sample<Foo>>> {
        let samples = self
            .reader_address
            .take(
                max_samples,
                sample_states.to_vec(),
                view_states.to_vec(),
                instance_states.to_vec(),
                None,
            )
            .await??;

        Ok(samples
            .into_iter()
            .map(|(data, sample_info)| Sample::new(data, sample_info))
            .collect())
    }

    /// Async version of [`read_next_sample`](crate::subscription::data_reader::DataReader::read_next_sample).
    #[tracing::instrument(skip(self))]
    pub async fn read_next_sample(&self) -> DdsResult<Sample<Foo>> {
        let mut samples = {
            self.reader_address
                .read(
                    1,
                    vec![SampleStateKind::NotRead],
                    ANY_VIEW_STATE.to_vec(),
                    ANY_INSTANCE_STATE.to_vec(),
                    None,
                )
                .await??
        };
        let (data, sample_info) = samples.pop().expect("Would return NoData if empty");
        Ok(Sample::new(data, sample_info))
    }

    /// Async version of [`take_next_sample`](crate::subscription::data_reader::DataReader::take_next_sample).
    #[tracing::instrument(skip(self))]
    pub async fn take_next_sample(&self) -> DdsResult<Sample<Foo>> {
        let mut samples = self
            .reader_address
            .take(
                1,
                vec![SampleStateKind::NotRead],
                ANY_VIEW_STATE.to_vec(),
                ANY_INSTANCE_STATE.to_vec(),
                None,
            )
            .await??;
        let (data, sample_info) = samples.pop().expect("Would return NoData if empty");
        Ok(Sample::new(data, sample_info))
    }

    /// Async version of [`read_instance`](crate::subscription::data_reader::DataReader::read_instance).
    #[tracing::instrument(skip(self))]
    pub async fn read_instance(
        &self,
        max_samples: i32,
        a_handle: InstanceHandle,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
    ) -> DdsResult<Vec<Sample<Foo>>> {
        let samples = self
            .reader_address
            .read(
                max_samples,
                sample_states.to_vec(),
                view_states.to_vec(),
                instance_states.to_vec(),
                Some(a_handle),
            )
            .await??;
        Ok(samples
            .into_iter()
            .map(|(data, sample_info)| Sample::new(data, sample_info))
            .collect())
    }

    /// Async version of [`take_instance`](crate::subscription::data_reader::DataReader::take_instance).
    #[tracing::instrument(skip(self))]
    pub async fn take_instance(
        &self,
        max_samples: i32,
        a_handle: InstanceHandle,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
    ) -> DdsResult<Vec<Sample<Foo>>> {
        let samples = self
            .reader_address
            .take(
                max_samples,
                sample_states.to_vec(),
                view_states.to_vec(),
                instance_states.to_vec(),
                Some(a_handle),
            )
            .await??;
        Ok(samples
            .into_iter()
            .map(|(data, sample_info)| Sample::new(data, sample_info))
            .collect())
    }

    /// Async version of [`read_next_instance`](crate::subscription::data_reader::DataReader::read_next_instance).
    #[tracing::instrument(skip(self))]
    pub async fn read_next_instance(
        &self,
        max_samples: i32,
        previous_handle: Option<InstanceHandle>,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
    ) -> DdsResult<Vec<Sample<Foo>>> {
        let samples = self
            .reader_address
            .read_next_instance(
                max_samples,
                previous_handle,
                sample_states.to_vec(),
                view_states.to_vec(),
                instance_states.to_vec(),
            )
            .await??;
        Ok(samples
            .into_iter()
            .map(|(data, sample_info)| Sample::new(data, sample_info))
            .collect())
    }

    /// Async version of [`take_next_instance`](crate::subscription::data_reader::DataReader::take_next_instance).
    #[tracing::instrument(skip(self))]
    pub async fn take_next_instance(
        &self,
        max_samples: i32,
        previous_handle: Option<InstanceHandle>,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
    ) -> DdsResult<Vec<Sample<Foo>>> {
        let samples = self
            .reader_address
            .take_next_instance(
                max_samples,
                previous_handle,
                sample_states.to_vec(),
                view_states.to_vec(),
                instance_states.to_vec(),
            )
            .await??;
        Ok(samples
            .into_iter()
            .map(|(data, sample_info)| Sample::new(data, sample_info))
            .collect())
    }

    /// Async version of [`get_key_value`](crate::subscription::data_reader::DataReader::get_key_value).
    #[tracing::instrument(skip(self, _key_holder))]
    pub async fn get_key_value(
        &self,
        _key_holder: &mut Foo,
        _handle: InstanceHandle,
    ) -> DdsResult<()> {
        todo!()
    }

    /// Async version of [`lookup_instance`](crate::subscription::data_reader::DataReader::lookup_instance).
    #[tracing::instrument(skip(self, _instance))]
    pub async fn lookup_instance(&self, _instance: &Foo) -> DdsResult<Option<InstanceHandle>> {
        todo!()
    }
}

impl<Foo> DataReaderAsync<Foo> {
    /// Async version of [`get_liveliness_changed_status`](crate::subscription::data_reader::DataReader::get_liveliness_changed_status).
    #[tracing::instrument(skip(self))]
    pub async fn get_liveliness_changed_status(&self) -> DdsResult<LivelinessChangedStatus> {
        todo!()
    }

    /// Async version of [`get_requested_deadline_missed_status`](crate::subscription::data_reader::DataReader::get_requested_deadline_missed_status).
    #[tracing::instrument(skip(self))]
    pub async fn get_requested_deadline_missed_status(
        &self,
    ) -> DdsResult<RequestedDeadlineMissedStatus> {
        todo!()
    }

    /// Async version of [`get_requested_incompatible_qos_status`](crate::subscription::data_reader::DataReader::get_requested_incompatible_qos_status).
    #[tracing::instrument(skip(self))]
    pub async fn get_requested_incompatible_qos_status(
        &self,
    ) -> DdsResult<RequestedIncompatibleQosStatus> {
        todo!()
    }

    /// Async version of [`get_sample_lost_status`](crate::subscription::data_reader::DataReader::get_sample_lost_status).
    #[tracing::instrument(skip(self))]
    pub async fn get_sample_lost_status(&self) -> DdsResult<SampleLostStatus> {
        todo!()
    }

    /// Async version of [`get_sample_rejected_status`](crate::subscription::data_reader::DataReader::get_sample_rejected_status).
    #[tracing::instrument(skip(self))]
    pub async fn get_sample_rejected_status(&self) -> DdsResult<SampleRejectedStatus> {
        todo!()
    }

    /// Async version of [`get_subscription_matched_status`](crate::subscription::data_reader::DataReader::get_subscription_matched_status).
    #[tracing::instrument(skip(self))]
    pub async fn get_subscription_matched_status(&self) -> DdsResult<SubscriptionMatchedStatus> {
        self.reader_address.get_subscription_matched_status().await
    }

    /// Async version of [`get_topicdescription`](crate::subscription::data_reader::DataReader::get_topicdescription).
    #[tracing::instrument(skip(self))]
    pub fn get_topicdescription(&self) -> TopicAsync {
        self.topic.clone()
    }

    /// Async version of [`get_subscriber`](crate::subscription::data_reader::DataReader::get_subscriber).
    #[tracing::instrument(skip(self))]
    pub fn get_subscriber(&self) -> SubscriberAsync {
        self.subscriber.clone()
    }

    /// Async version of [`wait_for_historical_data`](crate::subscription::data_reader::DataReader::wait_for_historical_data).
    #[tracing::instrument(skip(self))]
    pub async fn wait_for_historical_data(&self, max_wait: Duration) -> DdsResult<()> {
        tokio::time::timeout(max_wait.into(), async {
            loop {
                if self.reader_address.is_historical_data_received().await?? {
                    return Ok(());
                }
            }
        })
        .await
        .map_err(|_| DdsError::Timeout)?
    }

    /// Async version of [`get_matched_publication_data`](crate::subscription::data_reader::DataReader::get_matched_publication_data).
    #[tracing::instrument(skip(self))]
    pub async fn get_matched_publication_data(
        &self,
        publication_handle: InstanceHandle,
    ) -> DdsResult<PublicationBuiltinTopicData> {
        self.reader_address
            .get_matched_publication_data(publication_handle)
            .await?
    }

    /// Async version of [`get_matched_publications`](crate::subscription::data_reader::DataReader::get_matched_publications).
    #[tracing::instrument(skip(self))]
    pub async fn get_matched_publications(&self) -> DdsResult<Vec<InstanceHandle>> {
        self.reader_address.get_matched_publications().await
    }
}

impl<Foo> DataReaderAsync<Foo> {
    /// Async version of [`set_qos`](crate::subscription::data_reader::DataReader::set_qos).
    pub async fn set_qos(&self, qos: QosKind<DataReaderQos>) -> DdsResult<()> {
        let q = match qos {
            QosKind::Default => {
                self.subscriber_address()
                    .get_default_datareader_qos()
                    .await?
            }
            QosKind::Specific(q) => {
                q.is_consistent()?;
                q
            }
        };

        if self.reader_address.is_enabled().await? {
            let current_qos = self.get_qos().await?;
            q.check_immutability(&current_qos)?;
            self.reader_address.set_qos(q).await?;

            self.announce_reader().await?;
        } else {
            self.reader_address.set_qos(q).await?;
        }

        Ok(())
    }

    /// Async version of [`get_qos`](crate::subscription::data_reader::DataReader::get_qos).
    #[tracing::instrument(skip(self))]
    pub async fn get_qos(&self) -> DdsResult<DataReaderQos> {
        self.reader_address.get_qos().await
    }

    /// Async version of [`get_statuscondition`](crate::subscription::data_reader::DataReader::get_statuscondition).
    #[tracing::instrument(skip(self))]
    pub fn get_statuscondition(&self) -> StatusConditionAsync {
        StatusConditionAsync::new(
            self.status_condition_address.clone(),
            self.runtime_handle().clone(),
        )
    }

    /// Async version of [`get_status_changes`](crate::subscription::data_reader::DataReader::get_status_changes).
    #[tracing::instrument(skip(self))]
    pub async fn get_status_changes(&self) -> DdsResult<Vec<StatusKind>> {
        todo!()
    }

    /// Async version of [`enable`](crate::subscription::data_reader::DataReader::enable).
    #[tracing::instrument(skip(self))]
    pub async fn enable(&self) -> DdsResult<()> {
        if !self.reader_address.is_enabled().await? {
            self.reader_address.enable().await?;

            self.announce_reader().await?;
        }
        Ok(())
    }

    /// Async version of [`get_instance_handle`](crate::subscription::data_reader::DataReader::get_instance_handle).
    #[tracing::instrument(skip(self))]
    pub async fn get_instance_handle(&self) -> DdsResult<InstanceHandle> {
        self.reader_address.get_instance_handle().await
    }
}

impl<'a, Foo> DataReaderAsync<Foo>
where
    Foo: 'a,
{
    /// Async version of [`set_listener`](crate::subscription::data_reader::DataReader::set_listener).
    #[tracing::instrument(skip(self, a_listener))]
    pub async fn set_listener(
        &self,
        a_listener: Option<Box<dyn DataReaderListenerAsync<Foo = Foo> + Send + 'a>>,
        mask: &[StatusKind],
    ) -> DdsResult<()> {
        self.reader_address
            .set_listener(
                a_listener.map::<Box<dyn AnyDataReaderListener + Send>, _>(|b| Box::new(b)),
                mask.to_vec(),
                self.runtime_handle().clone(),
            )
            .await
    }
}
