// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use crate::{
    state::JoiningState,
    utilities::{
        GenesisPfxInfo, LocalEvent, Name, ProofRequest, RelocatedInfo, Rpc, TryResult, WaitedEvent,
    },
};
use unwrap::unwrap;

#[derive(Debug, PartialEq)]
pub struct JoiningRelocateCandidate<'a>(pub &'a mut JoiningState);

impl<'a> JoiningRelocateCandidate<'a> {
    pub fn start_event_loop(&mut self, relocated_info: RelocatedInfo) {
        self.0.join_routine.relocated_info = Some(relocated_info);

        self.connect_or_send_candidate_info();
        self.start_refused_timeout();
    }

    pub fn try_next(&mut self, event: WaitedEvent) -> TryResult {
        let result = match event {
            WaitedEvent::Rpc(rpc) => self.try_rpc(rpc),
            WaitedEvent::LocalEvent(local_event) => self.try_local_event(local_event),
            _ => TryResult::Unhandled,
        };

        if result == TryResult::Unhandled {
            self.discard();
        }
        TryResult::Handled
    }

    fn try_rpc(&mut self, rpc: Rpc) -> TryResult {
        if !rpc
            .destination()
            .map(|name| self.0.action.is_our_name(name))
            .unwrap_or(false)
        {
            return TryResult::Unhandled;
        }

        match rpc {
            Rpc::NodeApproval(_, info) => {
                self.exit(info);
                TryResult::Handled
            }
            Rpc::ConnectionInfoResponse { source, .. } => {
                self.send_candidate_info(source);
                TryResult::Handled
            }
            Rpc::ResourceProof { proof, source, .. } => {
                self.start_compute_resource_proof(source, proof);
                TryResult::Handled
            }
            Rpc::ResourceProofReceipt { source, .. } => {
                self.send_next_proof_response(source);
                TryResult::Handled
            }
            _ => TryResult::Unhandled,
        }
    }

    fn try_local_event(&mut self, local_event: LocalEvent) -> TryResult {
        match local_event {
            LocalEvent::ResourceProofForElderReady(source) => {
                self.send_next_proof_response(source);
                TryResult::Handled
            }
            LocalEvent::JoiningTimeoutResendInfo => {
                self.connect_or_send_candidate_info();
                TryResult::Handled
            }
            _ => TryResult::Unhandled,
        }
    }

    fn exit(&mut self, info: GenesisPfxInfo) {
        self.0.join_routine.routine_complete_output = Some(info);
    }

    fn discard(&mut self) {}

    fn send_next_proof_response(&mut self, source: Name) {
        if let Some(next_part) = self.0.action.get_next_resource_proof_part(source) {
            self.0
                .action
                .send_resource_proof_response(source, next_part);
        }
    }

    fn send_candidate_info(&mut self, destination: Name) {
        self.0
            .action
            .send_candidate_info(destination, unwrap!(self.0.join_routine.relocated_info));
    }

    fn connect_or_send_candidate_info(&mut self) {
        let relocated_info = unwrap!(self.0.join_routine.relocated_info);

        let (connected, unconnected) = self.0.action.get_connected_and_unconnected(relocated_info);

        for name in unconnected {
            self.0.action.send_connection_info_request(name);
        }

        for name in connected {
            self.0.action.send_candidate_info(name, relocated_info);
        }

        self.0
            .action
            .schedule_event(LocalEvent::JoiningTimeoutResendInfo);
    }

    fn start_refused_timeout(&mut self) {
        self.0
            .action
            .schedule_event(LocalEvent::JoiningTimeoutProofRefused);
    }

    fn start_compute_resource_proof(&mut self, source: Name, proof: ProofRequest) {
        self.0.action.start_compute_resource_proof(source, proof);
    }
}
