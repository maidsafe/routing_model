// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use crate::{
    state::{MemberState, ProcessElderChangeState, ProcessSplitState},
    utilities::{
        ChangeElder, LocalEvent, Node, ParsecVote, Rpc, SectionInfo, TryResult, WaitedEvent,
    },
};
use unwrap::unwrap;

#[derive(Debug, PartialEq)]
pub struct StartMergeSplitAndChangeElders<'a>(pub &'a mut MemberState);

impl<'a> StartMergeSplitAndChangeElders<'a> {
    // TODO - remove the `allow` once we have a test for this method.
    #[allow(dead_code)]
    fn start_event_loop(&mut self) {
        self.start_check_elder_timeout()
    }

    pub fn try_next(&mut self, event: WaitedEvent) -> TryResult {
        match event {
            WaitedEvent::ParsecConsensus(vote) => self.try_consensus(&vote),
            WaitedEvent::Rpc(rpc) => self.try_rpc(rpc),
            WaitedEvent::LocalEvent(LocalEvent::TimeoutCheckElder) => {
                self.vote_parsec_check_elder();
                TryResult::Handled
            }
            _ => TryResult::Unhandled,
        }
    }

    fn try_consensus(&mut self, vote: &ParsecVote) -> TryResult {
        match vote {
            ParsecVote::NeighbourMerge(merge_info) => {
                self.store_merge_infos(*merge_info);
                TryResult::Handled
            }
            ParsecVote::CheckElder => {
                self.check_merge();
                TryResult::Handled
            }
            _ => TryResult::Unhandled,
        }
    }

    fn try_rpc(&mut self, rpc: Rpc) -> TryResult {
        match rpc {
            Rpc::Merge(section_info) => {
                self.vote_parsec_neighbour_merge(section_info);
                TryResult::Handled
            }

            _ => TryResult::Unhandled,
        }
    }

    fn store_merge_infos(&mut self, merge_info: SectionInfo) {
        self.0.action.store_merge_infos(merge_info);
    }

    fn has_merge_infos(&mut self) -> bool {
        self.0.action.has_merge_infos()
    }

    fn merge_needed(&mut self) -> bool {
        self.0.action.merge_needed()
    }

    fn split_needed(&self) -> bool {
        self.0.action.split_needed()
    }

    fn check_merge(&mut self) {
        if self.has_merge_infos() || self.merge_needed() {
            self.concurrent_transition_to_process_merge();
        } else {
            self.check_elder();
        }
    }

    fn check_elder(&mut self) {
        match self.0.action.check_elder() {
            Some(change_elder) => self.concurrent_transition_to_process_elder_change(change_elder),
            None => {
                if self.split_needed() {
                    self.concurrent_transition_to_process_split();
                } else {
                    self.start_check_elder_timeout();
                }
            }
        }
    }

    fn concurrent_transition_to_process_merge(&mut self) {
        self.0.as_process_merge().start_event_loop()
    }

    fn concurrent_transition_to_process_split(&mut self) {
        self.0.as_process_split().start_event_loop()
    }

    fn concurrent_transition_to_process_elder_change(&mut self, change_elder: ChangeElder) {
        self.0
            .as_process_elder_change()
            .start_event_loop(change_elder)
    }

    fn transition_exit_process_elder_change(&mut self) {
        // TODO: ResourceProof_Cancel
        // TODO: RelocatedNodeConnection_Reset
        self.start_check_elder_timeout()
    }

    fn transition_exit_process_split(&self) {
        // TODO: ResourceProof_Cancel
        // TODO: RelocatedNodeConnection_Reset
        self.start_check_elder_timeout()
    }

    fn transition_exit_process_merge(&self) {
        // TODO: ResourceProof_Cancel
        // TODO: RelocatedNodeConnection_Reset
        self.start_check_elder_timeout()
    }

    fn vote_parsec_check_elder(&mut self) {
        self.0.action.vote_parsec(ParsecVote::CheckElder);
    }

    fn vote_parsec_neighbour_merge(&mut self, section_info: SectionInfo) {
        self.0
            .action
            .vote_parsec(ParsecVote::NeighbourMerge(section_info));
    }

    fn start_check_elder_timeout(&self) {
        self.0.action.schedule_event(LocalEvent::TimeoutCheckElder);
    }
}

#[derive(Debug, PartialEq)]
pub struct ProcessElderChange<'a>(pub &'a mut MemberState);

impl<'a> ProcessElderChange<'a> {
    pub fn start_event_loop(&mut self, change_elder: ChangeElder) {
        self.routine_state_mut().is_active = true;
        self.routine_state_mut().change_elder = Some(change_elder.clone());
        self.vote_for_elder_change(change_elder)
    }

    fn exit_event_loop(&mut self) {
        self.routine_state_mut().is_active = false;
        self.routine_state_mut().change_elder = None;
        self.0
            .as_start_merge_split_and_change_elders()
            .transition_exit_process_elder_change()
    }

    pub fn try_next(&mut self, event: WaitedEvent) -> TryResult {
        match event {
            WaitedEvent::ParsecConsensus(vote) => self.try_consensus(&vote),
            _ => TryResult::Unhandled,
        }
    }

    fn try_consensus(&mut self, vote: &ParsecVote) -> TryResult {
        if !self.routine_state().wait_votes.contains(&vote) {
            return TryResult::Unhandled;
        }

        let wait_votes = &mut self.routine_state_mut().wait_votes;
        wait_votes.retain(|wait_vote| wait_vote != vote);

        if wait_votes.is_empty() {
            self.mark_elder_change();
            self.exit_event_loop();
        }
        TryResult::Handled
    }

    fn vote_for_elder_change(&mut self, change_elder: ChangeElder) {
        let votes = self.0.action.get_elder_change_votes(&change_elder);
        self.routine_state_mut().change_elder = Some(change_elder);
        self.routine_state_mut().wait_votes = votes;

        for vote in &self.routine_state().wait_votes {
            self.0.action.vote_parsec(*vote);
        }
    }

    fn routine_state(&self) -> &ProcessElderChangeState {
        &self
            .0
            .start_merge_split_and_change_elders
            .sub_routine_process_elder_change
    }

    fn routine_state_mut(&mut self) -> &mut ProcessElderChangeState {
        &mut self
            .0
            .start_merge_split_and_change_elders
            .sub_routine_process_elder_change
    }

    fn mark_elder_change(&mut self) {
        let change_elder = unwrap!(self.routine_state_mut().change_elder.take());
        self.0.action.mark_elder_change(change_elder);
    }
}

#[derive(Debug, PartialEq)]
pub struct ProcessMerge<'a>(pub &'a mut MemberState);

impl<'a> ProcessMerge<'a> {
    pub fn start_event_loop(&mut self) {
        self.set_is_active(true);
        self.0.action.send_merge_rpc();
        self.check_sibling_merge_info();
    }

    fn exit_event_loop(&mut self) {
        self.set_is_active(false);
        self.0
            .as_start_merge_split_and_change_elders()
            .transition_exit_process_merge()
    }

    fn set_is_active(&mut self, is_active: bool) {
        self.0
            .start_merge_split_and_change_elders
            .sub_routine_process_merge_active = is_active;
    }

    fn check_sibling_merge_info(&self) {
        if self.0.action.has_sibling_merge_info() {
            let new_section = self.0.action.merge_sibling_info_to_new_section();
            self.0
                .action
                .vote_parsec(ParsecVote::NewSectionInfo(new_section));
        }
    }

    pub fn try_next(&mut self, event: WaitedEvent) -> TryResult {
        match event {
            WaitedEvent::ParsecConsensus(vote) => self.try_consensus(vote),
            WaitedEvent::Rpc(_) | WaitedEvent::LocalEvent(_) => TryResult::Unhandled,
        }
    }

    fn try_consensus(&mut self, vote: ParsecVote) -> TryResult {
        match vote {
            ParsecVote::NewSectionInfo(_) => {
                self.0.action.complete_merge();
                self.update_elder_status();
                self.exit_event_loop();
                TryResult::Handled
            }
            ParsecVote::NeighbourMerge(merge_info) => {
                self.0.action.store_merge_infos(merge_info);
                self.check_sibling_merge_info();
                TryResult::Handled
            }
            _ => TryResult::Unhandled,
        }
    }

    fn update_elder_status(&self) {
        // TODO
    }
}

#[derive(Debug, PartialEq)]
pub struct ProcessSplit<'a>(pub &'a mut MemberState);

impl<'a> ProcessSplit<'a> {
    pub fn start_event_loop(&mut self) {
        self.routine_state_mut().is_active = true;
        self.vote_for_split_sections();
    }

    fn exit_event_loop(&mut self) {
        self.routine_state_mut().is_active = false;
        self.0
            .as_start_merge_split_and_change_elders()
            .transition_exit_process_split()
    }

    pub fn try_next(&mut self, event: WaitedEvent) -> TryResult {
        match event {
            WaitedEvent::ParsecConsensus(vote) => self.try_consensus(&vote),
            WaitedEvent::Rpc(_) | WaitedEvent::LocalEvent(_) => TryResult::Unhandled,
        }
    }

    fn try_consensus(&mut self, vote: &ParsecVote) -> TryResult {
        if !self.routine_state().wait_votes.contains(&vote) {
            return TryResult::Unhandled;
        }

        let wait_votes = &mut self.routine_state_mut().wait_votes;
        wait_votes.retain(|wait_vote| wait_vote != vote);

        if wait_votes.is_empty() {
            self.complete_split();
            self.mark_elder_change();
            self.exit_event_loop();
        }
        TryResult::Handled
    }

    fn vote_for_split_sections(&mut self) {
        let votes = self.0.action.get_section_split_votes();
        self.routine_state_mut().wait_votes = votes;

        for vote in &self.routine_state().wait_votes {
            self.0.action.vote_parsec(*vote);
        }
    }

    fn routine_state(&self) -> &ProcessSplitState {
        &self
            .0
            .start_merge_split_and_change_elders
            .sub_routine_process_split
    }

    fn routine_state_mut(&mut self) -> &mut ProcessSplitState {
        &mut self
            .0
            .start_merge_split_and_change_elders
            .sub_routine_process_split
    }

    fn complete_split(&self) {
        // TODO: start parsec with new genesis ...
        self.0.action.complete_split();
    }

    fn mark_elder_change(&mut self) {
        // TODO: update elder status
    }
}

#[derive(Debug, PartialEq)]
pub struct CheckOnlineOffline<'a>(pub &'a mut MemberState);

impl<'a> CheckOnlineOffline<'a> {
    pub fn try_next(&mut self, event: WaitedEvent) -> TryResult {
        match event {
            WaitedEvent::ParsecConsensus(vote) => self.try_consensus(&vote),
            WaitedEvent::LocalEvent(local_event) => self.try_local_event(local_event),
            // Delegate to other event loops
            _ => TryResult::Unhandled,
        }
    }

    fn try_consensus(&mut self, vote: &ParsecVote) -> TryResult {
        match vote {
            ParsecVote::Offline(node) => {
                self.make_node_offline(*node);
                TryResult::Handled
            }
            ParsecVote::BackOnline(node) => {
                self.make_node_back_online(*node);
                TryResult::Handled
            }
            // Delegate to other event loops
            _ => TryResult::Unhandled,
        }
    }

    fn try_local_event(&mut self, local_event: LocalEvent) -> TryResult {
        match local_event {
            LocalEvent::NodeDetectedOffline(node) => {
                self.vote_parsec_offline(node);
                TryResult::Handled
            }
            LocalEvent::NodeDetectedBackOnline(node) => {
                self.vote_parsec_back_online(node);
                TryResult::Handled
            }
            // Delegate to other event loops
            _ => TryResult::Unhandled,
        }
    }

    fn vote_parsec_offline(&mut self, node: Node) {
        self.0.action.vote_parsec(ParsecVote::Offline(node));
    }

    fn vote_parsec_back_online(&mut self, node: Node) {
        self.0.action.vote_parsec(ParsecVote::BackOnline(node));
    }

    fn make_node_offline(&mut self, node: Node) {
        self.0.action.set_node_offline_state(node);
    }

    /// A member of a section that was lost connection to became offline, but is now online again
    fn make_node_back_online(&mut self, node: Node) {
        self.0.action.set_node_back_online_state(node);
    }
}
