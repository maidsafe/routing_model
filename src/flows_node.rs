// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use crate::{
    state::*,
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

        self.send_candidate_info();
        self.start_resend_info_timeout();
        self.start_refused_connect_timeout();
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
            Rpc::NodeConnected(_, _) => {
                self.complete_connected();
                TryResult::Handled
            }
            Rpc::NodeApproval(_, info) => {
                self.exit(info);
                TryResult::Handled
            }
            Rpc::ConnectionInfoRequest {
                source,
                connection_info,
                ..
            } => {
                self.send_connection_info_response(source, connection_info);
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
                self.check_connected_and_resend_info();
                TryResult::Handled
            }
            _ => TryResult::Unhandled,
        }
    }

    fn check_connected_and_resend_info(&mut self) {
        if self.0.join_routine.connected {
            self.resend_proofs();
        } else {
            self.send_candidate_info();
        }
        self.start_resend_info_timeout();
    }

    fn exit(&mut self, info: GenesisPfxInfo) {
        self.0.join_routine.routine_complete_output = Some(info);
    }

    fn discard(&mut self) {}

    fn send_connection_info_response(&mut self, source: Name, _connect_info: i32) {
        self.0.action.send_connection_info_response(source);
    }

    fn send_next_proof_response(&mut self, source: Name) {
        if let Some(next_part) = self.0.action.get_next_resource_proof_part(source) {
            self.0
                .action
                .send_resource_proof_response(source, next_part);
        }
    }

    fn resend_proof_response(&mut self, source: Name) {
        if let Some(next_part) = self.0.action.get_resend_resource_proof_part(source) {
            self.0
                .action
                .send_resource_proof_response(source, next_part);
        }
    }

    fn send_candidate_info(&mut self) {
        self.0
            .action
            .send_candidate_info(unwrap!(self.0.join_routine.relocated_info));
    }

    fn start_resend_info_timeout(&mut self) {
        self.0
            .action
            .schedule_event(LocalEvent::JoiningTimeoutResendInfo);
    }

    fn start_refused_connect_timeout(&mut self) {
        self.0
            .action
            .schedule_event(LocalEvent::JoiningTimeoutConnectRefused);
    }

    fn complete_connected(&mut self) {
        self.0.join_routine.connected = true;
        self.0
            .action
            .kill_scheduled_event(LocalEvent::JoiningTimeoutConnectRefused);
    }

    fn start_compute_resource_proof(&mut self, source: Name, proof: ProofRequest) {
        self.0
            .action
            .schedule_event(LocalEvent::JoiningTimeoutProofRefused);
        self.0.action.start_compute_resource_proof(source, proof);
    }

    fn resend_proofs(&mut self) {
        self.0
            .join_routine
            .need_resend_proofs
            .clone()
            .iter()
            .for_each(|name| self.resend_proof_response(*name));

        self.0.join_routine.need_resend_proofs = self
            .0
            .action
            .get_resource_proof_elders()
            .into_iter()
            .collect();
    }
}
