// Copyright 2020 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under the MIT license <LICENSE-MIT
// http://opensource.org/licenses/MIT> or the Modified BSD license <LICENSE-BSD
// https://opensource.org/licenses/BSD-3-Clause>, at your option. This file may not be copied,
// modified, or distributed except according to those terms. Please review the Licences for the
// specific language governing permissions and limitations relating to use of the SAFE Network
// Software.

use crate::{
    actions::Action,
    flows_dst::{RespondToRelocateRequests, StartResourceProof},
    flows_elder::{
        CheckOnlineOffline, ProcessElderChange, ProcessMerge, ProcessSplit,
        StartMergeSplitAndChangeElders,
    },
    flows_node::JoiningRelocateCandidate,
    flows_src::{StartDecidesOnNodeToRelocate, StartRelocateSrc},
    utilities::{
        ActionTriggered, Candidate, CandidateInfo, ChangeElder, Event, GenesisPfxInfo, Name,
        ParsecVote, RelocatedInfo, Rpc, TryResult, WaitedEvent,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Display, Formatter},
};
use unwrap::unwrap;

#[derive(Debug, PartialEq, Default, Clone)]
pub struct ProcessElderChangeState {
    pub is_active: bool,
    pub wait_votes: Vec<ParsecVote>,
    pub change_elder: Option<ChangeElder>,
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct ProcessSplitState {
    pub is_active: bool,
    pub wait_votes: Vec<ParsecVote>,
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct StartMergeSplitAndChangeEldersState {
    pub sub_routine_process_split: ProcessSplitState,
    pub sub_routine_process_elder_change: ProcessElderChangeState,
    pub sub_routine_process_merge_active: bool,
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct StartResourceProofState {
    pub candidate_info: Option<CandidateInfo>,
    pub candidate: Option<(Name, Candidate)>,
    pub voted_online: bool,
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct StartRelocateSrcState {
    pub already_relocating: BTreeMap<Candidate, i32>,
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct StartRelocatedNodeConnectionState {
    pub candidates: BTreeSet<Name>,
    pub candidates_info: BTreeMap<Name, CandidateInfo>,
    pub candidates_voted: BTreeSet<Name>,
}

// The very top level event loop deciding how the sub event loops are processed
#[derive(PartialEq, Default, Clone, Debug)]
pub struct MemberState {
    pub action: Action,
    pub failure: Option<Event>,
    pub start_resource_proof: StartResourceProofState,
    pub start_relocated_node_connection_state: StartRelocatedNodeConnectionState,
    pub start_relocate_src: StartRelocateSrcState,
    pub start_merge_split_and_change_elders: StartMergeSplitAndChangeEldersState,
}

impl MemberState {
    pub fn try_next(&mut self, event: Event) -> TryResult {
        if let Some(test_event) = event.to_test_event() {
            self.action.process_test_events(test_event);
            return TryResult::Handled;
        }

        let event = unwrap!(event.to_waited_event());

        if let TryResult::Handled = self.as_check_online_offline().try_next(event) {
            return TryResult::Handled;
        }

        if self
            .start_merge_split_and_change_elders
            .sub_routine_process_split
            .is_active
        {
            if let TryResult::Handled = self.as_process_split().try_next(event) {
                return TryResult::Handled;
            }
        }

        if self
            .start_merge_split_and_change_elders
            .sub_routine_process_merge_active
        {
            if let TryResult::Handled = self.as_process_merge().try_next(event) {
                return TryResult::Handled;
            }
        }

        if self
            .start_merge_split_and_change_elders
            .sub_routine_process_elder_change
            .is_active
        {
            if let TryResult::Handled = self.as_process_elder_change().try_next(event) {
                return TryResult::Handled;
            }
        }

        if let TryResult::Handled = self
            .as_start_merge_split_and_change_elders()
            .try_next(event)
        {
            return TryResult::Handled;
        }

        if let TryResult::Handled = self.as_start_relocate_src().try_next(event) {
            return TryResult::Handled;
        }

        if let TryResult::Handled = self.as_start_decides_on_node_to_relocate().try_next(event) {
            return TryResult::Handled;
        }

        if let TryResult::Handled = self.as_start_resource_proof().try_next(event) {
            return TryResult::Handled;
        }

        if let TryResult::Handled = self.as_respond_to_relocate_requests().try_next(event) {
            return TryResult::Handled;
        }

        match event {
            WaitedEvent::Rpc(Rpc::ConnectionInfoResponse { .. }) => {
                self.action
                    .action_triggered(ActionTriggered::NotYetImplementedErrorTriggered);
                TryResult::Handled
            }
            // These should only happen if a routine started them, so it should have
            // handled them too, but other routine are not there yet and we want to test
            // these do not fail.
            WaitedEvent::ParsecConsensus(ParsecVote::RemoveElderNode(_))
            | WaitedEvent::ParsecConsensus(ParsecVote::AddElderNode(_))
            | WaitedEvent::ParsecConsensus(ParsecVote::NewSectionInfo(_)) => {
                self.action
                    .action_triggered(ActionTriggered::UnexpectedEventErrorTriggered);
                TryResult::Handled
            }

            _ => TryResult::Unhandled,
        }
    }

    pub fn as_respond_to_relocate_requests(&mut self) -> RespondToRelocateRequests {
        RespondToRelocateRequests(self)
    }

    pub fn as_start_resource_proof(&mut self) -> StartResourceProof {
        StartResourceProof(self)
    }

    pub fn as_start_merge_split_and_change_elders(&mut self) -> StartMergeSplitAndChangeElders {
        StartMergeSplitAndChangeElders(self)
    }

    pub fn as_check_online_offline(&mut self) -> CheckOnlineOffline {
        CheckOnlineOffline(self)
    }

    pub fn as_start_decides_on_node_to_relocate(&mut self) -> StartDecidesOnNodeToRelocate {
        StartDecidesOnNodeToRelocate(self)
    }

    pub fn as_start_relocate_src(&mut self) -> StartRelocateSrc {
        StartRelocateSrc(self)
    }

    pub fn as_process_merge(&mut self) -> ProcessMerge {
        ProcessMerge(self)
    }

    pub fn as_process_split(&mut self) -> ProcessSplit {
        ProcessSplit(self)
    }

    pub fn as_process_elder_change(&mut self) -> ProcessElderChange {
        ProcessElderChange(self)
    }

    pub fn failure_event(&mut self, event: Event) {
        self.failure = Some(event);
    }
}

impl Display for MemberState {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        let action = self.action.inner();
        writeln!(formatter, "MemberState {{")?;
        writeln!(formatter, "    Action {{")?;
        writeln!(
            formatter,
            "        our_attributes: {:?}",
            action.our_attributes
        )?;
        writeln!(formatter, "        our_section: {:?}", action.our_section)?;
        writeln!(formatter, "        our_current_nodes: {{")?;
        for node in action.our_current_nodes.values() {
            writeln!(
                formatter,
                "            NodeState {{ {}({:?}), work_units_done: {}, state: {:?} }}",
                if node.is_elder { "Elder" } else { "Adult" },
                node.node.0,
                node.work_units_done,
                node.state
            )?;
        }
        writeln!(formatter, "        }}")?;
        writeln!(formatter, "        our_events: {{")?;
        for event in &action.our_events {
            writeln!(formatter, "            {:?}", event)?;
        }
        writeln!(formatter, "        }}")?;
        writeln!(
            formatter,
            "        shortest_prefix: {:?}",
            action.shortest_prefix
        )?;
        writeln!(
            formatter,
            "        section_members: {:?}",
            action.section_members
        )?;
        writeln!(
            formatter,
            "        next_target_interval: {:?}",
            action.next_target_interval
        )?;
        writeln!(formatter, "        merge_infos: {:?}", action.merge_infos)?;
        writeln!(formatter, "        churn_needed: {:?}", action.churn_needed)?;
        writeln!(
            formatter,
            "        resource_proofs_for_elder: {:?}",
            action.resource_proofs_for_elder
        )?;
        writeln!(formatter, "    }}")?;
        writeln!(formatter, "    failure: {:?}", self.failure)?;
        writeln!(formatter, "    {:?}", self.start_resource_proof)?;
        writeln!(
            formatter,
            "    {:?}",
            self.start_relocated_node_connection_state
        )?;
        writeln!(formatter, "    {:?}", self.start_relocate_src)?;
        writeln!(
            formatter,
            "    {:?}",
            self.start_merge_split_and_change_elders
        )?;
        write!(formatter, "}}")
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct JoiningRelocateCandidateState {
    pub relocated_info: Option<RelocatedInfo>,
    pub connected: bool,
    pub need_resend_proofs: BTreeSet<Name>,

    pub routine_complete_output: Option<GenesisPfxInfo /*output*/>,
}

// The very top level event loop deciding how the sub event loops are processed
#[derive(Debug, PartialEq, Default, Clone)]
pub struct JoiningState {
    pub action: Action,
    pub failure: Option<Event>,
    pub join_routine: JoiningRelocateCandidateState,
}

impl JoiningState {
    pub fn start(&mut self, relocated_info: RelocatedInfo) {
        self.as_joining_relocate_candidate()
            .start_event_loop(relocated_info)
    }

    pub fn try_next(&mut self, event: Event) -> TryResult {
        if let Some(test_event) = event.to_test_event() {
            self.action.process_test_events(test_event);
            return TryResult::Handled;
        }

        let event = unwrap!(event.to_waited_event());

        if let TryResult::Handled = self.as_joining_relocate_candidate().try_next(event) {
            return TryResult::Handled;
        }

        TryResult::Unhandled
    }

    pub fn as_joining_relocate_candidate(&mut self) -> JoiningRelocateCandidate {
        JoiningRelocateCandidate(self)
    }

    pub fn failure_event(&mut self, event: Event) {
        self.failure = Some(event);
    }
}
