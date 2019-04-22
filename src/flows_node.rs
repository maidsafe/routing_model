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
        GenesisPfxInfo, LocalEvent, Name, ProofRequest, ProofSource, Rpc, SectionInfo, TryResult,
        WaitedEvent,
    },
};
use unwrap::unwrap;

#[derive(Debug, PartialEq)]
pub struct JoiningRelocateCandidate<'a>(pub &'a mut JoiningState);

impl<'a> JoiningRelocateCandidate<'a> {
    pub fn start_event_loop(&mut self, new_section: SectionInfo) {
        self.store_destination_members(new_section);
        self.send_connection_info_requests();
        self.start_resend_info_timeout();
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
        if let Rpc::NodeApproval(candidate, info) = &rpc {
            if self.0.action.is_our_name(Name(candidate.0.name)) {
                self.exit(*info);
                return TryResult::Handled;
            } else {
                return TryResult::Unhandled;
            }
        }

        if !rpc
            .destination()
            .map(|name| self.0.action.is_our_name(name))
            .unwrap_or(false)
        {
            return TryResult::Unhandled;
        }

        match rpc {
            Rpc::ConnectionInfoResponse {
                source,
                connection_info,
                ..
            } => {
                self.connect_and_send_candidate_info(source, connection_info);
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
            LocalEvent::ComputeResourceProofForElder(source, proof) => {
                self.send_first_proof_response(source, proof);
                TryResult::Handled
            }
            LocalEvent::JoiningTimeoutResendCandidateInfo => {
                self.send_connection_info_requests();
                self.start_resend_info_timeout();
                TryResult::Handled
            }
            _ => TryResult::Unhandled,
        }
    }

    fn exit(&mut self, info: GenesisPfxInfo) {
        self.0.join_routine.has_resource_proofs.clear();
        self.0.join_routine.routine_complete = Some(info);
    }

    fn discard(&mut self) {}

    fn store_destination_members(&mut self, section: SectionInfo) {
        let members = self.0.action.get_section_members(section);
        self.0.join_routine.has_resource_proofs = members
            .iter()
            .map(|node| (Name(node.0.name), (false, None)))
            .collect();
    }

    fn send_connection_info_requests(&mut self) {
        let has_resource_proofs = &self.0.join_routine.has_resource_proofs;
        for (name, _) in has_resource_proofs.iter().filter(|(_, value)| !value.0) {
            self.0.action.send_connection_info_request(*name);
        }
    }

    fn send_first_proof_response(&mut self, source: Name, mut proof_source: ProofSource) {
        let proof = self
            .0
            .join_routine
            .has_resource_proofs
            .get_mut(&source)
            .unwrap();

        let next_part = proof_source.next_part();
        proof.1 = Some(proof_source);

        self.0
            .action
            .send_resource_proof_response(source, next_part);
    }

    fn send_next_proof_response(&mut self, source: Name) {
        let proof_source = &mut unwrap!(self
            .0
            .join_routine
            .has_resource_proofs
            .get_mut(&source)
            .unwrap()
            .1
            .as_mut());

        let next_part = proof_source.next_part();
        self.0
            .action
            .send_resource_proof_response(source, next_part);
    }

    fn connect_and_send_candidate_info(&mut self, source: Name, _connect_info: i32) {
        self.0.action.send_candidate_info(source);
    }

    fn start_resend_info_timeout(&mut self) {
        self.0
            .action
            .schedule_event(LocalEvent::JoiningTimeoutResendCandidateInfo);
    }

    fn start_refused_timeout(&mut self) {
        self.0
            .action
            .schedule_event(LocalEvent::JoiningTimeoutRefused);
    }

    fn start_compute_resource_proof(&mut self, source: Name, proof: ProofRequest) {
        self.0.action.start_compute_resource_proof(source, proof);
        let proof = self
            .0
            .join_routine
            .has_resource_proofs
            .get_mut(&source)
            .unwrap();
        if !proof.0 {
            *proof = (true, None);
        }
    }
}
