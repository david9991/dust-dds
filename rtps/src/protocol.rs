use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::participant::Participant;
use crate::transport::udp::UdpTransport;

use rust_dds_interface::protocol::{
    ProtocolEntity, ProtocolParticipant, ProtocolReader, ProtocolWriter,
};
use rust_dds_interface::qos::{DataReaderQos, DataWriterQos};
use rust_dds_interface::types::{DomainId, InstanceHandle, ReturnCode, TopicKind};

pub struct RtpsProtocol {
    participant: Arc<Mutex<Participant>>,
    thread_handle: RefCell<Vec<JoinHandle<()>>>,
}

impl RtpsProtocol {
    pub fn new(domain_id: DomainId) -> Self {
        let interface = "Ethernet";
        let userdata_transport =
            UdpTransport::default_userdata_transport(domain_id, interface).unwrap();
        let metatraffic_transport =
            UdpTransport::default_metatraffic_transport(domain_id, interface).unwrap();
        let domain_tag = "".to_string();
        let lease_duration = rust_dds_interface::types::Duration {
            sec: 30,
            nanosec: 0,
        };

        let participant = Arc::new(Mutex::new(Participant::new(
            domain_id,
            userdata_transport,
            metatraffic_transport,
            domain_tag,
            lease_duration,
        )));

        Self {
            participant,
            thread_handle: RefCell::new(Vec::new()),
        }
    }
}

impl ProtocolEntity for RtpsProtocol {
    fn get_instance_handle(&self) -> InstanceHandle {
        self.participant.lock().unwrap().get_instance_handle()
    }
}

impl ProtocolParticipant for RtpsProtocol {
    fn create_publisher(&self) -> ReturnCode<InstanceHandle> {
        self.participant.lock().unwrap().create_publisher()
    }

    fn create_subscriber(&self) -> ReturnCode<InstanceHandle> {
        self.participant.lock().unwrap().create_subscriber()
    }

    fn create_writer(
        &self,
        parent_publisher: InstanceHandle,
        topic_kind: TopicKind,
        data_writer_qos: &DataWriterQos,
    ) -> ReturnCode<Box<dyn ProtocolWriter>> {
        Ok(Box::new(self.participant.lock().unwrap().create_writer(
            &parent_publisher,
            topic_kind,
            data_writer_qos,
        )?))
    }

    fn create_reader(
        &self,
        parent_subscriber: InstanceHandle,
        topic_kind: TopicKind,
        data_reader_qos: &DataReaderQos,
    ) -> ReturnCode<Box<dyn ProtocolReader>> {
        Ok(Box::new(self.participant.lock().unwrap().create_reader(
            &parent_subscriber,
            topic_kind,
            data_reader_qos,
        )?))
    }

    fn get_builtin_subscriber(&self) -> ReturnCode<InstanceHandle> {
        todo!()
        // Box::new(Subscriber::new(self.builtin_subscriber.clone()))
    }

    fn enable(&self) {
        let participant = self.participant.clone();

        let handle = std::thread::spawn(move || loop {
            participant.lock().unwrap().send_metatraffic();

            std::thread::sleep(std::time::Duration::from_millis(500));

            participant.lock().unwrap().reset_discovery()
        });

        self.thread_handle.borrow_mut().push(handle);
        // RtpsMessageReceiver::receive(
        //     self.participant.guid().prefix(),
        //     self.metatraffic_transport.as_ref(),
        //     self.builtin_publisher.lock().unwrap().iter()
        //     .chain(self.builtin_subscriber.lock().unwrap().iter()));

        // RtpsMessageSender::send(
        //         self.participant.guid().prefix(),
        //         self.metatraffic_transport.as_ref(),
        //         self.builtin_publisher.lock().unwrap().iter()
        //         .chain(self.builtin_subscriber.lock().unwrap().iter()));
    }

    // fn receive(
    //     &self,
    //     _publisher_list: &[&dyn ProtocolPublisher],
    //     _subscriber_list: &[&dyn ProtocolSubscriber],
    // ) {
    //     todo!()
    // }

    // fn send(
    //     &self,
    //     _publisher_list: &[&dyn ProtocolPublisher],
    //     _subscriber_list: &[&dyn ProtocolSubscriber],
    // ) {
    //     todo!()
    // }
}
