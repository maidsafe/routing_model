// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use crate::{
    state::{MemberState, StartResourceProofState},
    utilities::{
        Candidate, CandidateInfo, LocalEvent, Name, ParsecVote, Proof, RelocatedInfo, Rpc,
        TryResult, WaitedEvent,
    },
};
use unwrap::unwrap;

#[derive(Debug, PartialEq)]
pub struct RespondToRelocateRequests<'a>(pub &'a mut MemberState);

impl<'a> RespondToRelocateRequests<'a> {
    pub fn try_next(&mut self, event: WaitedEvent) -> TryResult {
        match event {
            WaitedEvent::Rpc(rpc) => self.try_rpc(rpc),
            WaitedEvent::ParsecConsensus(vote) => self.try_consensus(vote),
            _ => TryResult::Unhandled,
        }
    }

    fn try_rpc(&mut self, rpc: Rpc) -> TryResult {
        match rpc {
            Rpc::ExpectCandidate(candidate) => {
                self.vote_parsec_expect_candidate(candidate);
                TryResult::Handled
            }
            _ => TryResult::Unhandled,
        }
    }

    fn try_consensus(&mut self, vote: ParsecVote) -> TryResult {
        match vote {
            ParsecVote::ExpectCandidate(candidate) => {
                self.consensused_expect_candidate(candidate);
                TryResult::Handled
            }

            // Delegate to other event loops
            _ => TryResult::Unhandled,
        }
    }

    fn consensused_expect_candidate(&mut self, candidate: Candidate) {
        if self.0.action.check_shortest_prefix().is_some() {
            self.send_expect_candidate_rpc(candidate);
            return;
        }

        if let Some(info) = self.0.action.get_waiting_candidate_info(candidate) {
            self.resend_relocate_response_rpc(info);
            return;
        }

        if 0 == self.0.action.count_waiting_proofing_or_hop() {
            self.add_node_and_send_relocate_response_rpc(candidate);
            return;
        }

        self.send_refuse_candidate_rpc(candidate);
    }

    fn add_node_and_send_relocate_response_rpc(&mut self, candidate: Candidate) {
        let relocated_info = self.0.action.add_node_waiting_candidate_info(candidate);
        self.0.action.send_relocate_response_rpc(relocated_info);
    }

    fn resend_relocate_response_rpc(&mut self, relocated_info: RelocatedInfo) {
        self.0.action.send_relocate_response_rpc(relocated_info);
    }

    fn send_refuse_candidate_rpc(&mut self, candidate: Candidate) {
        self.0.action.send_rpc(Rpc::RefuseCandidate(candidate));
    }

    fn send_expect_candidate_rpc(&mut self, candidate: Candidate) {
        self.0.action.send_rpc(Rpc::ExpectCandidate(candidate));
    }

    fn vote_parsec_expect_candidate(&mut self, candidate: Candidate) {
        self.0
            .action
            .vote_parsec(ParsecVote::ExpectCandidate(candidate));
    }
}

#[derive(Debug, PartialEq)]
pub struct StartResourceProof<'a>(pub &'a mut MemberState);

impl<'a> StartResourceProof<'a> {
    // TODO - remove the `allow` once we have a test for this method.
    #[allow(dead_code)]
    fn start_event_loop(&mut self) {
        self.0
            .action
            .schedule_event(LocalEvent::CheckResourceProofTimeout);
    }

    pub fn try_next(&mut self, event: WaitedEvent) -> TryResult {
        match event {
            WaitedEvent::Rpc(rpc) => self.try_rpc(rpc),
            WaitedEvent::ParsecConsensus(vote) => self.try_consensus(vote),
            WaitedEvent::LocalEvent(local_event) => self.try_local_event(local_event),
        }
    }

    fn try_rpc(&mut self, rpc: Rpc) -> TryResult {
        match rpc {
            Rpc::ResourceProofResponse {
                candidate, proof, ..
            } => {
                self.rpc_proof(candidate, proof);
                TryResult::Handled
            }
            Rpc::CandidateInfo(info) => {
                self.rpc_info(info);
                TryResult::Handled
            }
            _ => TryResult::Unhandled,
        }
    }

    fn try_consensus(&mut self, vote: ParsecVote) -> TryResult {
        let for_candidate = self.has_candidate() && vote.candidate() == Some(self.candidate());

        match vote {
            ParsecVote::CheckResourceProof => {
                self.set_resource_proof_candidate();
                self.check_request_resource_proof();
                TryResult::Handled
            }
            ParsecVote::Online(_, new_candidate) if for_candidate => {
                self.make_node_online(new_candidate);
                TryResult::Handled
            }
            ParsecVote::PurgeCandidate(_) if for_candidate => {
                self.purge_node_info();
                TryResult::Handled
            }
            ParsecVote::Online(_, _) | ParsecVote::PurgeCandidate(_) => {
                self.discard();
                TryResult::Handled
            }

            // Delegate to other event loops
            _ => TryResult::Unhandled,
        }
    }

    fn try_local_event(&mut self, local_event: LocalEvent) -> TryResult {
        match local_event {
            LocalEvent::TimeoutAccept => {
                self.vote_parsec_purge_candidate();
                TryResult::Handled
            }
            LocalEvent::CheckResourceProofTimeout => {
                self.vote_parsec_check_resource_proof();
                TryResult::Handled
            }
            _ => TryResult::Unhandled,
        }
    }

    fn rpc_info(&mut self, info: CandidateInfo) {
        if self.has_candidate()
            && self.candidate() == info.old_public_id
            && self.0.action.is_valid_waited_info(info)
        {
            self.cache_candidate_info_and_send_resource_proof(info)
        } else {
            self.discard()
        }
    }

    fn rpc_proof(&mut self, candidate: Candidate, proof: Proof) {
        let from_candidate = self.has_candidate_info() && candidate == self.new_candidate();

        if from_candidate && !self.routine_state().voted_online && proof.is_valid() {
            if proof == Proof::ValidEnd {
                self.set_voted_online(true);
                self.vote_parsec_online_candidate();
            }
            self.send_resource_proof_receipt_rpc();
        } else {
            self.discard()
        }
    }

    fn routine_state(&self) -> &StartResourceProofState {
        &self.0.start_resource_proof
    }

    fn routine_state_mut(&mut self) -> &mut StartResourceProofState {
        &mut self.0.start_resource_proof
    }

    fn discard(&mut self) {}

    fn set_resource_proof_candidate(&mut self) {
        self.routine_state_mut().candidate = self.0.action.resource_proof_candidate();
    }

    fn set_voted_online(&mut self, value: bool) {
        self.routine_state_mut().voted_online = value;
    }

    fn vote_parsec_purge_candidate(&mut self) {
        self.0
            .action
            .vote_parsec(ParsecVote::PurgeCandidate(self.candidate()));
    }

    fn vote_parsec_check_resource_proof(&mut self) {
        self.0.action.vote_parsec(ParsecVote::CheckResourceProof);
    }

    fn vote_parsec_online_candidate(&mut self) {
        self.0
            .action
            .vote_parsec(ParsecVote::Online(self.candidate(), self.new_candidate()));
    }

    fn make_node_online(&mut self, new_public_id: Candidate) {
        self.0
            .action
            .set_candidate_online_state(self.waiting_candidate_name(), new_public_id);
        self.0.action.send_node_approval_rpc(new_public_id);
        self.finish_resource_proof()
    }

    fn purge_node_info(&mut self) {
        self.0.action.purge_node_info(self.waiting_candidate_name());
        self.finish_resource_proof()
    }

    fn finish_resource_proof(&mut self) {
        self.routine_state_mut().candidate = None;
        self.routine_state_mut().candidate_info = None;
        self.routine_state_mut().voted_online = false;

        self.0
            .action
            .schedule_event(LocalEvent::CheckResourceProofTimeout);
    }

    fn check_request_resource_proof(&mut self) {
        if self.has_candidate() {
            self.schedule_proof_timeout()
        } else {
            self.finish_resource_proof()
        }
    }

    fn schedule_proof_timeout(&mut self) {
        self.0.action.schedule_event(LocalEvent::TimeoutAccept);
    }

    fn send_resource_proof_receipt_rpc(&mut self) {
        self.0
            .action
            .send_candidate_proof_receipt(self.new_candidate());
    }

    fn candidate(&self) -> Candidate {
        unwrap!(self.routine_state().candidate).1
    }

    fn waiting_candidate_name(&self) -> Name {
        unwrap!(self.routine_state().candidate).0
    }

    fn has_candidate(&self) -> bool {
        self.routine_state().candidate.is_some()
    }

    fn candidate_info(&self) -> CandidateInfo {
        unwrap!(self.routine_state().candidate_info)
    }

    fn has_candidate_info(&self) -> bool {
        self.routine_state().candidate_info.is_some()
    }

    fn new_candidate(&self) -> Candidate {
        self.candidate_info().new_public_id
    }

    fn cache_candidate_info_and_send_resource_proof(&mut self, info: CandidateInfo) {
        self.routine_state_mut().candidate_info = Some(info);
        self.0
            .action
            .send_candidate_proof_request(self.new_candidate());
    }
}
