// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use crate::{
    state::{MemberState, StartRelocateSrcState},
    utilities::{Candidate, LocalEvent, ParsecVote, RelocatedInfo, Rpc, TryResult, WaitedEvent},
};
use unwrap::unwrap;

#[derive(Debug, PartialEq)]
pub struct StartDecidesOnNodeToRelocate<'a>(pub &'a mut MemberState);

impl<'a> StartDecidesOnNodeToRelocate<'a> {
    // TODO - remove the `allow` once we have a test for this method.
    #[allow(dead_code)]
    fn start_event_loop(&mut self) {
        self.start_work_unit_timeout()
    }

    pub fn try_next(&mut self, event: WaitedEvent) -> TryResult {
        match event {
            WaitedEvent::LocalEvent(local_event) => self.try_local_event(local_event),
            WaitedEvent::ParsecConsensus(vote) => self.try_consensus(vote),

            WaitedEvent::Rpc(_) => TryResult::Unhandled,
        }
    }

    fn try_local_event(&mut self, local_event: LocalEvent) -> TryResult {
        match local_event {
            LocalEvent::TimeoutWorkUnit => {
                self.vote_parsec_work_unit_increment();
                self.start_work_unit_timeout();
                TryResult::Handled
            }
            _ => TryResult::Unhandled,
        }
    }

    fn try_consensus(&mut self, vote: ParsecVote) -> TryResult {
        match vote {
            ParsecVote::WorkUnitIncrement => {
                self.increment_nodes_work_units();
                self.check_get_node_to_relocate();
                TryResult::Handled
            }

            // Delegate to other event loops
            _ => TryResult::Unhandled,
        }
    }

    fn check_get_node_to_relocate(&mut self) {
        if self.0.action.has_relocating_node() {
            return;
        }
        if let Some(candidate) = self.0.action.get_node_to_relocate() {
            self.set_relocating_candidate(candidate)
        }
    }

    //
    // Actions
    //
    fn increment_nodes_work_units(&mut self) {
        self.0.action.increment_nodes_work_units();
    }

    fn set_relocating_candidate(&mut self, candidate: Candidate) {
        self.0.action.set_candidate_relocating_state(candidate);
    }

    fn start_work_unit_timeout(&mut self) {
        self.0.action.schedule_event(LocalEvent::TimeoutWorkUnit);
    }

    //
    // Votes
    //

    fn vote_parsec_work_unit_increment(&mut self) {
        self.0.action.vote_parsec(ParsecVote::WorkUnitIncrement);
    }
}

#[derive(Debug, PartialEq)]
pub struct StartRelocateSrc<'a>(pub &'a mut MemberState);

// StartRelocateSrc Sub Routine
impl<'a> StartRelocateSrc<'a> {
    // TODO - remove the `allow` once we have a test for this method.
    #[allow(dead_code)]
    fn start_event_loop(&mut self) {
        self.start_check_relocate_timeout()
    }

    pub fn try_next(&mut self, event: WaitedEvent) -> TryResult {
        match event {
            WaitedEvent::LocalEvent(local_event) => self.try_local_event(local_event),
            WaitedEvent::Rpc(rpc) => self.try_rpc(rpc),
            WaitedEvent::ParsecConsensus(vote) => self.try_consensus(vote),
        }
    }

    fn try_local_event(&mut self, local_event: LocalEvent) -> TryResult {
        match local_event {
            LocalEvent::TimeoutCheckRelocate => {
                self.vote_parsec_check_relocate();
                self.start_check_relocate_timeout();
                TryResult::Handled
            }
            _ => TryResult::Unhandled,
        }
    }

    fn try_rpc(&mut self, rpc: Rpc) -> TryResult {
        match rpc {
            Rpc::RefuseCandidate(candidate) => {
                self.vote_parsec_refuse_candidate(candidate);
                TryResult::Handled
            }
            Rpc::RelocateResponse(info) => {
                self.vote_parsec_relocation_response(info);
                TryResult::Handled
            }
            _ => TryResult::Unhandled,
        }
    }

    fn try_consensus(&mut self, vote: ParsecVote) -> TryResult {
        match vote {
            ParsecVote::CheckRelocate => {
                self.check_need_relocate();
                self.update_wait_and_allow_resend();
                TryResult::Handled
            }
            ParsecVote::RefuseCandidate(candidate)
            | ParsecVote::RelocateResponse(RelocatedInfo { candidate, .. }) => {
                self.check_is_our_relocating_node(vote, candidate);
                TryResult::Handled
            }
            ParsecVote::RelocatedInfo(info) => {
                self.send_candidate_relocated_info_rpc(info);
                self.purge_node_info(info);
                TryResult::Handled
            }
            // Delegate to other event loops
            _ => TryResult::Unhandled,
        }
    }

    fn check_need_relocate(&mut self) {
        if let Some((candidate, _)) = self
            .0
            .action
            .get_best_relocating_node_and_target(&self.routine_state().already_relocating)
        {
            self.0.action.send_rpc(Rpc::ExpectCandidate(candidate));
            let inserted = self
                .mut_routine_state()
                .already_relocating
                .insert(candidate, 0);
            assert!(inserted.is_none());
        }
    }

    fn update_wait_and_allow_resend(&mut self) {
        let new_already_relocating = self
            .routine_state()
            .already_relocating
            .iter()
            .map(|(node, count)| (*node, *count + 1))
            .filter(|(_, count)| *count < 3)
            .collect();
        self.mut_routine_state().already_relocating = new_already_relocating;
    }

    fn check_is_our_relocating_node(&mut self, vote: ParsecVote, candidate: Candidate) {
        if self.0.action.is_our_relocating_node(candidate) {
            match vote {
                ParsecVote::RefuseCandidate(candidate) => self.allow_resend(candidate),
                ParsecVote::RelocateResponse(info) => self.set_relocated_and_prepare_info(info),
                _ => panic!("Unexpected vote"),
            }
        } else {
            self.discard()
        }
    }

    fn allow_resend(&mut self, candidate: Candidate) {
        unwrap!(self
            .mut_routine_state()
            .already_relocating
            .remove(&candidate));
    }

    fn set_relocated_and_prepare_info(&mut self, info: RelocatedInfo) {
        self.0.action.set_candidate_relocated_state(info);
        self.0.action.vote_parsec(ParsecVote::RelocatedInfo(info));
    }

    //
    // Routine state
    //

    fn routine_state(&self) -> &StartRelocateSrcState {
        &self.0.start_relocate_src
    }

    fn mut_routine_state(&mut self) -> &mut StartRelocateSrcState {
        &mut self.0.start_relocate_src
    }

    //
    // Actions
    //

    fn start_check_relocate_timeout(&mut self) {
        self.0
            .action
            .schedule_event(LocalEvent::TimeoutCheckRelocate);
    }

    fn purge_node_info(&mut self, info: RelocatedInfo) {
        self.0.action.purge_node_info(info.candidate.name());
    }

    fn discard(&mut self) {}

    //
    // RPCs
    //

    fn send_candidate_relocated_info_rpc(&mut self, info: RelocatedInfo) {
        self.0.action.send_rpc(Rpc::RelocatedInfo(info));
    }

    //
    // Votes
    //

    fn vote_parsec_check_relocate(&mut self) {
        self.0.action.vote_parsec(ParsecVote::CheckRelocate);
    }

    fn vote_parsec_refuse_candidate(&mut self, candidate: Candidate) {
        self.0
            .action
            .vote_parsec(ParsecVote::RefuseCandidate(candidate));
    }

    fn vote_parsec_relocation_response(&mut self, info: RelocatedInfo) {
        self.0
            .action
            .vote_parsec(ParsecVote::RelocateResponse(info));
    }
}
