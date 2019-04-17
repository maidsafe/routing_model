// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use crate::{
    state::{
        AcceptAsCandidateState, MemberState, ProcessElderChangeState,
        StartRelocatedNodeConnectionState,
    },
    utilities::{
        Candidate, CandidateInfo, ChangeElder, Event, LocalEvent, MergeInfo, Name, Node,
        ParsecVote, Proof, RelocatedInfo, Rpc,
    },
};
use unwrap::unwrap;

#[derive(Debug, PartialEq, Default, Clone)]
pub struct TopLevelDst(pub MemberState);

impl TopLevelDst {
    pub fn try_next(&self, event: Event) -> Option<MemberState> {
        match event {
            Event::Rpc(rpc) => self.try_rpc(rpc),
            Event::ParsecConsensus(vote) => self.try_consensus(vote),
            _ => None,
        }
        .map(|state| state.0)
    }

    fn try_rpc(&self, rpc: Rpc) -> Option<Self> {
        match rpc {
            Rpc::ExpectCandidate(candidate) => Some(self.vote_parsec_expect_candidate(candidate)),
            _ => None,
        }
    }

    fn try_consensus(&self, vote: ParsecVote) -> Option<Self> {
        match vote {
            ParsecVote::ExpectCandidate(candidate) => {
                Some(self.try_consensused_expect_candidate(candidate))
            }

            // Delegate to other event loops
            _ => None,
        }
    }

    fn try_consensused_expect_candidate(&self, candidate: Candidate) -> Self {
        match (
            self.0.action.get_waiting_candidate_info(candidate),
            self.0.action.count_waiting_proofing_or_hop(),
        ) {
            (Some(info), _) => self.resend_relocate_response_rpc(info),
            (_, 0) => self.add_node_and_send_relocate_response_rpc(candidate),
            (_, _) => self.send_refuse_candidate_rpc(candidate),
        }
    }

    fn add_node_and_send_relocate_response_rpc(&self, candidate: Candidate) -> Self {
        let relocated_info = self.0.action.add_node_waiting_candidate_info(candidate);
        self.0.action.send_relocate_response_rpc(relocated_info);
        self.clone()
    }

    fn resend_relocate_response_rpc(&self, relocated_info: RelocatedInfo) -> Self {
        self.0.action.send_relocate_response_rpc(relocated_info);
        self.clone()
    }

    fn send_refuse_candidate_rpc(&self, candidate: Candidate) -> Self {
        self.0.action.send_rpc(Rpc::RefuseCandidate(candidate));
        self.clone()
    }

    fn vote_parsec_expect_candidate(&self, candidate: Candidate) -> Self {
        self.0
            .action
            .vote_parsec(ParsecVote::ExpectCandidate(candidate));
        self.clone()
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct StartRelocatedNodeConnection(pub MemberState);

impl StartRelocatedNodeConnection {
    // TODO - remove the `allow` once we have a test for this method.
    #[allow(dead_code)]
    fn start_event_loop(&self) -> Self {
        self.schedule_time_out()
    }

    pub fn try_next(&self, event: Event) -> Option<MemberState> {
        match event {
            Event::Rpc(rpc) => self.try_rpc(rpc),
            Event::ParsecConsensus(vote) => self.try_consensus(vote),
            Event::LocalEvent(local_event) => self.try_local_event(local_event),
        }
        .map(|state| state.0)
    }

    fn try_rpc(&self, rpc: Rpc) -> Option<Self> {
        match rpc {
            Rpc::CandidateInfo(info) => Some(self.rpc_info(info)),
            Rpc::ConnectionInfoResponse { .. } => {
                self.try_connect_and_vote_parsec_candidate_connected(rpc)
            }
            _ => None,
        }
    }

    fn try_consensus(&self, vote: ParsecVote) -> Option<Self> {
        match vote {
            ParsecVote::CandidateConnected(info) => Some(self.check_candidate_connnected(info)),
            ParsecVote::CheckRelocatedNodeConnection => Some(
                self.reject_candidates_that_took_too_long()
                    .schedule_time_out(),
            ),
            // Delegate to other event loops
            _ => None,
        }
    }

    fn try_local_event(&self, local_event: LocalEvent) -> Option<Self> {
        match local_event {
            LocalEvent::CheckRelocatedNodeConnectionTimeout => {
                Some(self.vote_parsec_check_relocated_node_connection())
            }
            _ => None,
        }
    }

    fn try_connect_and_vote_parsec_candidate_connected(&self, rpc: Rpc) -> Option<Self> {
        if let Rpc::ConnectionInfoResponse { source, .. } = rpc {
            if !self.routine_state().candidates_voted.contains(&source) {
                if let Some(info) = self.routine_state().candidates_info.get(&source) {
                    let mut state = self.clone();

                    state
                        .0
                        .action
                        .vote_parsec(ParsecVote::CandidateConnected(*info));
                    let _ = state.mut_routine_state().candidates_voted.insert(source);

                    return Some(state);
                }
            }
        }

        None
    }

    fn rpc_info(&self, info: CandidateInfo) -> Self {
        if self.0.action.is_valid_waited_info(info) {
            self.cache_candidate_info_and_send_connect_info(info)
        } else {
            self.discard()
        }
    }

    fn check_candidate_connnected(&self, info: CandidateInfo) -> Self {
        if self.0.action.is_valid_waited_info(info) {
            self.check_update_to_node(info)
                .send_node_connected_rpc(info)
        } else {
            self.discard()
        }
    }

    fn check_update_to_node(&self, info: CandidateInfo) -> Self {
        match self.0.action.check_shortest_prefix() {
            None => self.0.action.update_to_node_with_waiting_proof_state(info),
            Some(_) => self.0.action.update_to_node_with_relocating_hop_state(info),
        }
        self.clone()
    }

    fn routine_state(&self) -> &StartRelocatedNodeConnectionState {
        &self.0.start_relocated_node_connection_state
    }

    fn mut_routine_state(&mut self) -> &mut StartRelocatedNodeConnectionState {
        &mut self.0.start_relocated_node_connection_state
    }

    fn discard(&self) -> Self {
        self.clone()
    }

    fn reject_candidates_that_took_too_long(&self) -> Self {
        let mut state = self.clone();

        let new_connecting_nodes = state.0.action.waiting_node_connecting();
        let node_to_remove: Vec<Name> = new_connecting_nodes
            .intersection(&state.routine_state().candidates)
            .cloned()
            .collect();

        for name in node_to_remove {
            state.0.action.purge_node_info(name);
        }

        let candidates = state.0.action.waiting_node_connecting();
        let mut_routine_state = &mut state.mut_routine_state();

        mut_routine_state.candidates = candidates.clone();
        mut_routine_state.candidates_info = mut_routine_state
            .candidates_info
            .clone()
            .into_iter()
            .filter(|(name, _)| candidates.contains(name))
            .collect();
        mut_routine_state.candidates_voted = mut_routine_state
            .candidates_voted
            .clone()
            .into_iter()
            .filter(|name| candidates.contains(name))
            .collect();

        state
    }

    fn cache_candidate_info_and_send_connect_info(&self, info: CandidateInfo) -> Self {
        let mut state = self.clone();

        let _ = state
            .mut_routine_state()
            .candidates_info
            .insert(info.new_public_id.name(), info);
        state
            .0
            .action
            .send_connection_info_request(info.new_public_id.name());

        state
    }

    fn schedule_time_out(&self) -> Self {
        self.0
            .action
            .schedule_event(LocalEvent::CheckRelocatedNodeConnectionTimeout);
        self.clone()
    }

    fn send_node_connected_rpc(&self, info: CandidateInfo) -> Self {
        self.0.action.send_node_connected(info.new_public_id);
        self.clone()
    }

    fn vote_parsec_check_relocated_node_connection(&self) -> Self {
        self.0
            .action
            .vote_parsec(ParsecVote::CheckRelocatedNodeConnection);
        self.clone()
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct StartResourceProof(pub MemberState);

// AcceptAsCandidate Sub Routine
impl StartResourceProof {
    // TODO - remove the `allow` once we have a test for this method.
    #[allow(dead_code)]
    fn start_event_loop(&self) -> Self {
        self.0
            .action
            .schedule_event(LocalEvent::CheckResourceProofTimeout);
        self.clone()
    }

    pub fn try_next(&self, event: Event) -> Option<MemberState> {
        match event {
            Event::Rpc(Rpc::ResourceProofResponse {
                candidate, proof, ..
            }) => self.try_rpc_proof(candidate, proof),
            Event::ParsecConsensus(vote) => self.try_consensus(vote),
            Event::LocalEvent(local_event) => self.try_local_event(local_event),
            // Delegate to other event loops
            _ => None,
        }
        .map(|state| state.0)
    }

    fn try_rpc_proof(&self, candidate: Candidate, proof: Proof) -> Option<Self> {
        if !self.has_candidate()
            || candidate != self.candidate()
            || self.routine_state().voted_online
            || !proof.is_valid()
        {
            return Some(self.discard());
        }

        Some(match proof {
            Proof::ValidPart => self.send_resource_proof_receipt_rpc(),
            Proof::ValidEnd => self.set_voted_online(true).vote_parsec_online_candidate(),
            Proof::Invalid => panic!("Only valid proof"),
        })
    }

    fn try_consensus(&self, vote: ParsecVote) -> Option<Self> {
        let from_candidate = self.has_candidate() && vote.candidate() == Some(self.candidate());

        match vote {
            ParsecVote::CheckResourceProof => Some(
                self.set_resource_proof_candidate()
                    .check_request_resource_proof(),
            ),
            ParsecVote::Online(_) if from_candidate => Some(self.make_node_online()),
            ParsecVote::PurgeCandidate(_) if from_candidate => Some(self.purge_node_info()),
            ParsecVote::Online(_) | ParsecVote::PurgeCandidate(_) => Some(self.discard()),

            // Delegate to other event loops
            _ => None,
        }
    }

    fn try_local_event(&self, local_event: LocalEvent) -> Option<Self> {
        match local_event {
            LocalEvent::TimeoutAccept => Some(self.vote_parsec_purge_candidate()),
            LocalEvent::CheckResourceProofTimeout => Some(self.vote_parsec_check_resource_proof()),
            _ => None,
        }
    }

    fn routine_state(&self) -> &AcceptAsCandidateState {
        &self.0.sub_routine_accept_as_candidate
    }

    fn mut_routine_state(&mut self) -> &mut AcceptAsCandidateState {
        &mut self.0.sub_routine_accept_as_candidate
    }

    fn discard(&self) -> Self {
        self.clone()
    }

    fn set_resource_proof_candidate(&self) -> Self {
        let mut state = self.clone();
        state.mut_routine_state().candidate = state.0.action.resource_proof_candidate();
        state
    }

    // TODO - remove the `allow` once we have a test for this method.
    #[allow(dead_code)]
    fn set_got_candidate_info(&self, value: bool) -> Self {
        let mut state = self.clone();
        state.mut_routine_state().got_candidate_info = value;
        state
    }

    fn set_voted_online(&self, value: bool) -> Self {
        let mut state = self.clone();
        state.mut_routine_state().voted_online = value;
        state
    }

    fn vote_parsec_purge_candidate(&self) -> Self {
        self.0
            .action
            .vote_parsec(ParsecVote::PurgeCandidate(self.candidate()));
        self.clone()
    }

    fn vote_parsec_check_resource_proof(&self) -> Self {
        self.0.action.vote_parsec(ParsecVote::CheckResourceProof);
        self.clone()
    }

    fn vote_parsec_online_candidate(&self) -> Self {
        self.0
            .action
            .vote_parsec(ParsecVote::Online(self.candidate()));
        self.clone()
    }

    fn make_node_online(&self) -> Self {
        self.0.action.set_candidate_online_state(self.candidate());
        self.0.action.send_node_approval_rpc(self.candidate());
        self.finish_resource_proof()
    }

    fn purge_node_info(&self) -> Self {
        self.0.action.purge_node_info(self.candidate().name());
        self.finish_resource_proof()
    }

    fn finish_resource_proof(&self) -> Self {
        let mut state = self.clone();
        state.mut_routine_state().candidate = None;
        state.mut_routine_state().voted_online = false;
        state.mut_routine_state().got_candidate_info = false;

        self.0
            .action
            .schedule_event(LocalEvent::CheckResourceProofTimeout);

        state
    }

    fn check_request_resource_proof(&self) -> Self {
        if self.has_candidate() {
            self.send_resource_proof_rpc()
        } else {
            self.finish_resource_proof()
        }
    }

    fn send_resource_proof_rpc(&self) -> Self {
        self.0.action.send_candidate_proof_request(self.candidate());
        self.clone()
    }

    fn send_resource_proof_receipt_rpc(&self) -> Self {
        self.0.action.send_candidate_proof_receipt(self.candidate());
        self.clone()
    }

    fn candidate(&self) -> Candidate {
        unwrap!(self.routine_state().candidate)
    }

    fn has_candidate(&self) -> bool {
        self.routine_state().candidate.is_some()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct CheckAndProcessElderChange(pub MemberState);

// CheckAndProcessElderChange Sub Routine
impl CheckAndProcessElderChange {
    // TODO - remove the `allow` once we have a test for this method.
    #[allow(dead_code)]
    fn start_event_loop(&self) -> Self {
        self.start_check_elder_timeout()
    }

    pub fn try_next(&self, event: Event) -> Option<MemberState> {
        match event {
            Event::ParsecConsensus(vote) => self.try_consensus(&vote),
            Event::Rpc(rpc) => self.try_rpc(rpc),
            Event::LocalEvent(LocalEvent::TimeoutCheckElder) => {
                Some(self.vote_parsec_check_elder())
            }
            _ => None,
        }
        .map(|state| state.0)
    }

    fn try_consensus(&self, vote: &ParsecVote) -> Option<Self> {
        match vote {
            ParsecVote::NeighbourMerge(merge_info) => Some(self.store_merge_infos(*merge_info)),
            ParsecVote::CheckElder => Some(self.check_merge()),
            _ => None,
        }
    }

    fn try_rpc(&self, rpc: Rpc) -> Option<Self> {
        match rpc {
            Rpc::Merge => Some(self.vote_parsec_neighbour_merge()),
            _ => None,
        }
    }

    fn store_merge_infos(&self, merge_info: MergeInfo) -> Self {
        self.0.action.store_merge_infos(merge_info);
        self.clone()
    }

    fn merge_needed(&self) -> bool {
        self.0.action.merge_needed()
    }

    fn has_merge_infos(&self) -> bool {
        self.0.action.has_merge_infos()
    }

    fn check_merge(&self) -> Self {
        if self.has_merge_infos() || self.merge_needed() {
            // TODO: -> Concurrent to ProcessMerge
            self.0.action.send_rpc(Rpc::Merge);
            self.clone()
        } else {
            self.check_elder()
        }
    }

    fn check_elder(&self) -> Self {
        match self.0.action.check_elder() {
            Some(change_elder) => self.concurrent_transition_to_process_elder_change(change_elder),
            None => self.start_check_elder_timeout(),
        }
    }

    fn concurrent_transition_to_process_elder_change(&self, change_elder: ChangeElder) -> Self {
        self.0
            .as_process_elder_change()
            .start_event_loop(change_elder)
            .0
            .as_check_and_process_elder_change()
    }

    fn transition_exit_process_elder_change(&self) -> Self {
        self.start_check_elder_timeout()
    }

    fn vote_parsec_check_elder(&self) -> Self {
        self.0.action.vote_parsec(ParsecVote::CheckElder);
        self.clone()
    }

    fn vote_parsec_neighbour_merge(&self) -> Self {
        self.0
            .action
            .vote_parsec(ParsecVote::NeighbourMerge(MergeInfo));
        self.clone()
    }

    fn start_check_elder_timeout(&self) -> Self {
        self.0.action.schedule_event(LocalEvent::TimeoutCheckElder);
        self.clone()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ProcessElderChange(pub MemberState);

impl ProcessElderChange {
    pub fn start_event_loop(&self, change_elder: ChangeElder) -> Self {
        let mut state = self.clone();
        state.mut_routine_state().is_active = true;
        state.mut_routine_state().change_elder = Some(change_elder.clone());
        state.vote_for_elder_change(change_elder)
    }

    fn exit_event_loop(&self) -> Self {
        let mut state = self.clone();
        state.mut_routine_state().is_active = false;
        state.mut_routine_state().change_elder = None;
        state
            .0
            .as_check_and_process_elder_change()
            .transition_exit_process_elder_change()
            .0
            .as_process_elder_change()
    }

    pub fn try_next(&self, event: Event) -> Option<MemberState> {
        match event {
            Event::ParsecConsensus(vote) => self.try_consensus(&vote),
            _ => None,
        }
        .map(|state| state.0)
    }

    fn try_consensus(&self, vote: &ParsecVote) -> Option<Self> {
        if !self.routine_state().wait_votes.contains(&vote) {
            return None;
        }

        let mut state = self.clone();
        let wait_votes = &mut state.mut_routine_state().wait_votes;
        wait_votes.retain(|wait_vote| wait_vote != vote);

        if wait_votes.is_empty() {
            Some(state.mark_elder_change().exit_event_loop())
        } else {
            Some(state)
        }
    }

    fn vote_for_elder_change(&self, change_elder: ChangeElder) -> Self {
        let mut state = self.clone();

        let votes = state.0.action.get_elder_change_votes(&change_elder);
        state.mut_routine_state().change_elder = Some(change_elder);
        state.mut_routine_state().wait_votes = votes;

        for vote in &state.routine_state().wait_votes {
            state.0.action.vote_parsec(*vote);
        }

        state
    }

    fn routine_state(&self) -> &ProcessElderChangeState {
        &self
            .0
            .check_and_process_elder_change_routine
            .sub_routine_process_elder_change
    }

    fn mut_routine_state(&mut self) -> &mut ProcessElderChangeState {
        &mut self
            .0
            .check_and_process_elder_change_routine
            .sub_routine_process_elder_change
    }

    fn mark_elder_change(&self) -> Self {
        let mut state = self.clone();
        let change_elder = unwrap!(state.mut_routine_state().change_elder.take());
        state.0.action.mark_elder_change(change_elder);
        state
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct CheckOnlineOffline(pub MemberState);

impl CheckOnlineOffline {
    pub fn try_next(&self, event: Event) -> Option<MemberState> {
        match event {
            Event::ParsecConsensus(vote) => self.try_consensus(&vote),
            Event::LocalEvent(local_event) => self.try_local_event(local_event),
            // Delegate to other event loops
            _ => None,
        }
        .map(|state: CheckOnlineOffline| state.0)
    }

    fn try_consensus(&self, vote: &ParsecVote) -> Option<Self> {
        match vote {
            ParsecVote::Offline(node) => Some(self.make_node_offline(*node)),
            ParsecVote::BackOnline(node) => Some(self.make_node_back_online(*node)),
            // Delegate to other event loops
            _ => None,
        }
    }

    fn try_local_event(&self, local_event: LocalEvent) -> Option<Self> {
        match local_event {
            LocalEvent::NodeDetectedOffline(node) => Some(self.vote_parsec_offline(node)),
            LocalEvent::NodeDetectedBackOnline(node) => Some(self.vote_parsec_back_online(node)),
            // Delegate to other event loops
            _ => None,
        }
    }

    fn vote_parsec_offline(&self, node: Node) -> Self {
        self.0.action.vote_parsec(ParsecVote::Offline(node));
        self.clone()
    }

    fn vote_parsec_back_online(&self, node: Node) -> Self {
        self.0.action.vote_parsec(ParsecVote::BackOnline(node));
        self.clone()
    }

    fn make_node_offline(&self, node: Node) -> Self {
        self.0.action.set_node_offline_state(node);
        self.clone()
    }

    /// A member of a section that was lost connection to became offline, but is now online again
    fn make_node_back_online(&self, node: Node) -> Self {
        self.0.action.set_node_back_online_state(node);
        self.clone()
    }
}
