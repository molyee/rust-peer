/*
 * Copyright 2020 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use crate::event_polling;
use crate::peer_service::libp2p::connect_protocol::events::{InPeerEvent, OutPeerEvent};
use crate::peer_service::libp2p::notifications::OutPeerNotification;
use libp2p::{
    core::ConnectedPoint,
    core::Multiaddr,
    swarm::{NetworkBehaviour, NetworkBehaviourAction, OneShotHandler, ProtocolsHandler},
    PeerId,
};
use log::trace;
use std::collections::VecDeque;

#[derive(Default)]
pub struct PeerConnectProtocolBehaviour {
    /// Queue of received network messages from connected peers
    /// that need to be handled during polling.
    events: VecDeque<NetworkBehaviourAction<OutPeerEvent, OutPeerNotification>>,
}

impl PeerConnectProtocolBehaviour {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
        }
    }

    pub fn relay_message(&mut self, src: PeerId, dst: PeerId, message: Vec<u8>) {
        trace!(
            "peer_service/connect_protocol/behaviour: relaying message {:?} to {:?}",
            message,
            dst
        );

        self.events.push_back(NetworkBehaviourAction::SendEvent {
            peer_id: dst,
            event: OutPeerEvent::Relay {
                src_id: src.into_bytes(),
                data: message,
            },
        })
    }

    pub fn send_network_state(&mut self, dst: PeerId, state: Vec<PeerId>) {
        trace!(
            "peer_service/connect_protocol/behaviour: sending network state {:?} to {:?}",
            state,
            dst
        );

        self.events.push_back(NetworkBehaviourAction::SendEvent {
            peer_id: dst,
            event: OutPeerEvent::NetworkState {
                state: state
                    .into_iter()
                    .map(|node_id| node_id.into_bytes())
                    .collect::<Vec<Vec<u8>>>(),
            },
        })
    }
}

impl NetworkBehaviour for PeerConnectProtocolBehaviour {
    type ProtocolsHandler = OneShotHandler<InPeerEvent, OutPeerEvent, InnerMessage>;
    type OutEvent = OutPeerNotification;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Default::default()
    }

    fn addresses_of_peer(&mut self, _peer_id: &PeerId) -> Vec<Multiaddr> {
        Vec::new()
    }

    fn inject_connected(&mut self, peer_id: PeerId, _cp: ConnectedPoint) {
        trace!(
            "peer_service/connect_protocol/inject_connected: new peer {} joined",
            peer_id
        );

        self.events.push_back(NetworkBehaviourAction::GenerateEvent(
            OutPeerNotification::PeerConnected { peer_id },
        ));
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId, _cp: ConnectedPoint) {
        trace!(
            "peer_service/connect_protocol/inject_disconnected: peer {} disconnected",
            peer_id
        );

        self.events.push_back(NetworkBehaviourAction::GenerateEvent(
            OutPeerNotification::PeerDisconnected {
                peer_id: peer_id.clone(),
            },
        ));
    }

    fn inject_node_event(&mut self, source: PeerId, event: InnerMessage) {
        trace!(
            "peer_service/connect_protocol/inject_node_event: new event {:?} received",
            event
        );

        match event {
            InnerMessage::Rx(m) => match m {
                InPeerEvent::Relay { dst_id, data } => self.events.push_back(
                    NetworkBehaviourAction::GenerateEvent(OutPeerNotification::Relay {
                        src_id: source,
                        dst_id: PeerId::from_bytes(dst_id).unwrap(),
                        data,
                    }),
                ),
                InPeerEvent::GetNetworkState => {
                    self.events.push_back(NetworkBehaviourAction::GenerateEvent(
                        OutPeerNotification::GetNetworkState { src_id: source },
                    ))
                }
            },
            InnerMessage::Tx => {}
        }
    }

    // produces OutPeerNotification events
    event_polling!(
        poll,
        events,
        NetworkBehaviourAction<<Self::ProtocolsHandler as ProtocolsHandler>::InEvent, Self::OutEvent>
    );
}

/// Transmission between the OneShotHandler message type and the InNodeMessage message type.
#[derive(Debug)]
pub enum InnerMessage {
    /// Message has been received from a remote.
    Rx(InPeerEvent),

    /// RelayMessage has been sent
    Tx,
}

impl From<InPeerEvent> for InnerMessage {
    #[inline]
    fn from(in_node_message: InPeerEvent) -> InnerMessage {
        InnerMessage::Rx(in_node_message)
    }
}

impl From<()> for InnerMessage {
    #[inline]
    fn from(_: ()) -> InnerMessage {
        InnerMessage::Tx
    }
}
