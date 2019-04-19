// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use crate::utilities::{
    ActionTriggered, Age, Attributes, Candidate, CandidateInfo, ChangeElder, Event, GenesisPfxInfo,
    LocalEvent, MergeInfo, Name, Node, NodeChange, NodeState, ParsecVote, Proof, ProofRequest,
    ProofSource, RelocatedInfo, Rpc, Section, SectionInfo, State, TestEvent,
};
use itertools::Itertools;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Debug, Formatter},
    rc::Rc,
};
use unwrap::unwrap;

#[derive(Debug, PartialEq, Clone)]
pub struct InnerAction {
    pub our_attributes: Attributes,
    pub our_section: SectionInfo,
    pub our_current_nodes: BTreeMap<Name, NodeState>,

    pub our_events: Vec<Event>,

    pub shortest_prefix: Option<Section>,
    pub section_members: BTreeMap<SectionInfo, Vec<Node>>,
    pub next_target_interval: Name,

    pub merge_infos: Option<MergeInfo>,
    pub merge_needed: bool,
}

impl InnerAction {
    pub fn new_with_our_attributes(name: Attributes) -> Self {
        Self {
            our_attributes: name,
            our_section: Default::default(),
            our_current_nodes: Default::default(),

            our_events: Default::default(),

            shortest_prefix: Default::default(),
            section_members: Default::default(),
            next_target_interval: Name(0),

            merge_infos: Default::default(),
            merge_needed: false,
        }
    }

    fn extend_current_nodes(mut self, nodes: &[NodeState]) -> Self {
        self.our_current_nodes.extend(
            nodes
                .iter()
                .map(|state| (Name(state.node.0.name), state.clone())),
        );
        self
    }

    pub fn extend_current_nodes_with(self, value: &NodeState, nodes: &[Node]) -> Self {
        let node_states = nodes
            .iter()
            .map(|node| NodeState {
                node: *node,
                ..value.clone()
            })
            .collect_vec();
        self.extend_current_nodes(&node_states)
    }

    pub fn with_section_members(mut self, section: SectionInfo, nodes: &[Node]) -> Self {
        let inserted = self.section_members.insert(section, nodes.to_vec());
        assert!(inserted.is_none());
        self
    }

    pub fn with_next_target_interval(mut self, target: Name) -> Self {
        self.next_target_interval = target;
        self
    }

    fn add_node(&mut self, node_state: NodeState) {
        self.our_events
            .push(NodeChange::AddWithState(node_state.node, node_state.state).to_event());
        let inserted = self
            .our_current_nodes
            .insert(node_state.node.name(), node_state);
        assert!(inserted.is_none());
    }

    fn remove_node(&mut self, name: Name) {
        self.our_events.push(NodeChange::Remove(name).to_event());
        unwrap!(self.our_current_nodes.remove(&name));
    }

    fn replace_node(&mut self, node_name: Name, node_state: NodeState) {
        self.our_events
            .push(NodeChange::ReplaceWith(node_name, node_state.node, node_state.state).to_event());

        let removed = self.our_current_nodes.remove(&node_name);
        let inserted = self
            .our_current_nodes
            .insert(node_state.node.name(), node_state);

        assert!(
            removed.is_some() && inserted.is_none(),
            "{:?} - {:?}",
            removed,
            inserted
        );
    }

    fn set_node_state(&mut self, name: Name, state: State) {
        let node = &mut self.our_current_nodes.get_mut(&name).unwrap();

        node.state = state;
        self.our_events
            .push(NodeChange::State(node.node, state).to_event());
    }

    fn set_elder_state(&mut self, name: Name, value: bool) {
        let node = &mut self.our_current_nodes.get_mut(&name).unwrap();

        node.is_elder = value;
        self.our_events
            .push(NodeChange::Elder(node.node, value).to_event());
    }

    fn set_section_info(&mut self, section: SectionInfo) {
        self.our_section = section;
    }

    fn store_merge_infos(&mut self, merge_info: MergeInfo) {
        self.merge_infos = Some(merge_info);
    }
}

#[derive(Clone)]
pub struct Action(Rc<RefCell<InnerAction>>);

impl Action {
    pub fn new(inner: InnerAction) -> Self {
        Action(Rc::new(RefCell::new(inner)))
    }

    pub fn inner(&self) -> InnerAction {
        (*self.0.borrow()).clone()
    }

    pub fn remove_processed_state(&self) {
        let inner = &mut self.0.borrow_mut();
        inner.our_events.clear();
    }

    pub fn process_test_events(&self, event: TestEvent) {
        let set_enough_work_to_relocate = |name: Name| {
            let _ = self
                .0
                .borrow_mut()
                .our_current_nodes
                .get_mut(&name)
                .map(|state| state.work_units_done = state.node.0.age);
        };

        match event {
            TestEvent::SetMergeNeeded(value) => self.0.borrow_mut().merge_needed = value,
            TestEvent::SetShortestPrefix(value) => self.0.borrow_mut().shortest_prefix = value,
            TestEvent::SetWorkUnitEnoughToRelocate(node) => {
                set_enough_work_to_relocate(node.name())
            }
        }
    }

    pub fn vote_parsec(&self, vote: ParsecVote) {
        self.0.borrow_mut().our_events.push(vote.to_event());
    }

    pub fn send_rpc(&self, rpc: Rpc) {
        self.0.borrow_mut().our_events.push(rpc.to_event());
    }

    pub fn schedule_event(&self, event: LocalEvent) {
        self.0.borrow_mut().our_events.push(event.to_event());
    }

    pub fn action_triggered(&self, event: ActionTriggered) {
        self.0.borrow_mut().our_events.push(event.to_event());
    }

    pub fn add_node_waiting_candidate_info(&self, candidate: Candidate) -> RelocatedInfo {
        let target_interval_centre = self.0.borrow().next_target_interval;
        self.0.borrow_mut().next_target_interval.0 += 1;

        let info = RelocatedInfo {
            candidate,
            expected_age: Age(candidate.0.age + 1),
            target_interval_centre,
            section_info: self.0.borrow().our_section,
        };

        let state = NodeState {
            node: Node(Attributes {
                name: info.target_interval_centre.0,
                age: info.expected_age.0,
            }),
            state: State::WaitingCandidateInfo(info),
            ..NodeState::default()
        };

        self.0.borrow_mut().add_node(state);
        info
    }

    pub fn update_to_node_with_waiting_proof_state(&self, info: CandidateInfo) {
        self.update_to_node(info, State::WaitingProofing);
    }

    pub fn update_to_node_with_relocating_hop_state(&self, info: CandidateInfo) {
        self.update_to_node(info, State::RelocatingHop);
    }

    fn update_to_node(&self, info: CandidateInfo, state: State) {
        let state = NodeState {
            node: Node(info.new_public_id.0),
            state,
            ..NodeState::default()
        };

        self.0.borrow_mut().replace_node(info.destination, state);
    }

    pub fn set_candidate_online_state(&self, candidate: Candidate) {
        self.0
            .borrow_mut()
            .set_node_state(candidate.name(), State::Online);
    }

    pub fn set_node_offline_state(&self, node: Node) {
        self.0
            .borrow_mut()
            .set_node_state(node.name(), State::Offline);
    }

    pub fn set_node_back_online_state(&self, node: Node) {
        self.0
            .borrow_mut()
            .set_node_state(node.name(), State::RelocatingBackOnline);
    }

    pub fn set_candidate_relocating_state(&self, candidate: Candidate) {
        self.0
            .borrow_mut()
            .set_node_state(candidate.name(), State::RelocatingAgeIncrease);
    }

    pub fn set_candidate_relocated_state(&self, info: RelocatedInfo) {
        self.0
            .borrow_mut()
            .set_node_state(info.candidate.name(), State::Relocated(info));
    }

    pub fn purge_node_info(&self, name: Name) {
        self.0.borrow_mut().remove_node(name);
    }

    pub fn check_shortest_prefix(&self) -> Option<Section> {
        self.0.borrow().shortest_prefix
    }

    pub fn check_elder(&self) -> Option<ChangeElder> {
        let inner = &self.0.borrow();
        let our_current_nodes = &inner.our_current_nodes;

        let (new_elders, ex_elders, _elders) = {
            let mut sorted_values = our_current_nodes
                .values()
                .cloned()
                .sorted_by(|left, right| {
                    left.state
                        .cmp(&right.state)
                        .then(left.node.0.age.cmp(&right.node.0.age).reverse())
                        .then(left.node.0.name.cmp(&right.node.0.name))
                })
                .collect_vec();
            let elder_size = std::cmp::min(3, sorted_values.len());
            let adults = sorted_values.split_off(elder_size);

            let new_elders = sorted_values
                .iter()
                .filter(|elder| !elder.is_elder)
                .cloned()
                .collect_vec();
            let ex_elders = adults
                .iter()
                .filter(|elder| elder.is_elder)
                .cloned()
                .collect_vec();

            (new_elders, ex_elders, sorted_values)
        };

        let changes = new_elders
            .iter()
            .map(|elder| (elder, true))
            .chain(ex_elders.iter().map(|elder| (elder, false)))
            .map(|(elder, new_is_elder)| (elder.node, new_is_elder))
            .collect_vec();

        if changes.is_empty() {
            None
        } else {
            Some(ChangeElder {
                changes,
                new_section: SectionInfo(inner.our_section.0, inner.our_section.1 + 1),
            })
        }
    }

    pub fn get_elder_change_votes(&self, change_elder: &ChangeElder) -> Vec<ParsecVote> {
        change_elder
            .changes
            .iter()
            .map(|(node, new_is_elder)| match new_is_elder {
                true => ParsecVote::AddElderNode(*node),
                false => ParsecVote::RemoveElderNode(*node),
            })
            .chain(std::iter::once(ParsecVote::NewSectionInfo(
                change_elder.new_section,
            )))
            .collect_vec()
    }

    pub fn mark_elder_change(&self, change_elder: ChangeElder) {
        for (node, new_is_elder) in &change_elder.changes {
            self.0
                .borrow_mut()
                .set_elder_state(Name(node.0.name), *new_is_elder);
        }
        self.0
            .borrow_mut()
            .set_section_info(change_elder.new_section);
    }

    pub fn get_node_to_relocate(&self) -> Option<Candidate> {
        self.0
            .borrow()
            .our_current_nodes
            .values()
            .find(|state| !state.state.is_relocating() && state.work_units_done >= state.node.0.age)
            .map(|state| Candidate(state.node.0))
    }

    pub fn get_best_relocating_node_and_target(
        &self,
        already_relocating: &BTreeMap<Candidate, i32>,
    ) -> Option<(Candidate, Section)> {
        self.0
            .borrow()
            .our_current_nodes
            .values()
            .filter(|state| !already_relocating.contains_key(&Candidate(state.node.0)))
            .filter(|state| state.state.is_relocating() && !state.is_elder)
            .max_by_key(|state| {
                (
                    state.state == State::RelocatingAgeIncrease,
                    state.state == State::RelocatingHop,
                    state.state == State::RelocatingBackOnline,
                    state.node.0.age(),
                    state.node.0.name(),
                )
            })
            .map(|state| (Candidate(state.node.0), Section::default()))
    }

    pub fn is_our_relocating_node(&self, candidate: Candidate) -> bool {
        self.0
            .borrow()
            .our_current_nodes
            .get(&Name(candidate.0.name))
            .map(|state| state.state.is_relocating())
            .unwrap_or(false)
    }

    pub fn waiting_nodes_connecting(&self) -> BTreeSet<Name> {
        self.0
            .borrow()
            .our_current_nodes
            .iter()
            .filter_map(|(name, state)| match state.state {
                State::WaitingCandidateInfo(_) => Some(*name),
                _ => None,
            })
            .collect()
    }

    pub fn get_waiting_candidate_info(&self, candidate: Candidate) -> Option<RelocatedInfo> {
        self.0
            .borrow()
            .our_current_nodes
            .values()
            .filter_map(|state| match state.state {
                State::WaitingCandidateInfo(info) => Some(info),
                _ => None,
            })
            .find(|info| info.candidate == candidate)
    }

    pub fn count_waiting_proofing_or_hop(&self) -> usize {
        self.0
            .borrow()
            .our_current_nodes
            .values()
            .filter(|state| state.state.is_not_yet_full_node())
            .count()
    }

    pub fn resource_proof_candidate(&self) -> Option<Candidate> {
        self.0
            .borrow()
            .our_current_nodes
            .values()
            .filter(|state| state.state.is_resource_proofing())
            .map(|state| Candidate(state.node.0))
            .next()
    }

    pub fn is_valid_waited_info(&self, info: CandidateInfo) -> bool {
        if !info.valid {
            return false;
        }

        self.0
            .borrow()
            .our_current_nodes
            .get(&info.destination)
            .map(|state| state.state.is_waiting_candidate_info())
            .unwrap_or(false)
    }

    pub fn is_our_name(&self, name: Name) -> bool {
        self.our_name() == name
    }

    pub fn our_name(&self) -> Name {
        Name(self.0.borrow().our_attributes.name)
    }

    pub fn send_node_approval_rpc(&self, candidate: Candidate) {
        let section = GenesisPfxInfo(self.0.borrow().our_section);
        self.send_rpc(Rpc::NodeApproval(candidate, section));
    }

    pub fn send_relocate_response_rpc(&self, info: RelocatedInfo) {
        self.send_rpc(Rpc::RelocateResponse(info));
    }

    pub fn send_node_connected(&self, candidate: Candidate) {
        let section = GenesisPfxInfo(self.0.borrow().our_section);
        self.send_rpc(Rpc::NodeConnected(candidate, section));
    }

    pub fn send_candidate_proof_request(&self, candidate: Candidate) {
        let source = self.our_name();
        let proof = ProofRequest { value: source.0 };
        self.send_rpc(Rpc::ResourceProof {
            candidate,
            proof,
            source,
        });
    }

    pub fn send_candidate_proof_receipt(&self, candidate: Candidate) {
        let source = self.our_name();
        self.send_rpc(Rpc::ResourceProofReceipt { candidate, source });
    }

    pub fn start_compute_resource_proof(&self, source: Name, _proof: ProofRequest) {
        self.schedule_event(LocalEvent::ComputeResourceProofForElder(
            source,
            ProofSource(2),
        ));
    }

    pub fn get_section_members(&self, section_info: SectionInfo) -> Vec<Node> {
        self.0.borrow().section_members[&section_info].clone()
    }

    pub fn send_connection_info_request(&self, destination: Name) {
        let source = self.our_name();
        self.send_rpc(Rpc::ConnectionInfoRequest {
            source,
            destination,
            connection_info: source.0,
        });
    }

    pub fn send_candidate_info(&self, destination: Name) {
        let candidate = Candidate(self.0.borrow().our_attributes);
        self.send_rpc(Rpc::CandidateInfo(CandidateInfo {
            old_public_id: candidate,
            new_public_id: candidate,
            destination,
            valid: true,
        }));
    }

    pub fn send_resource_proof_response(&self, destination: Name, proof: Proof) {
        let candidate = Candidate(self.0.borrow().our_attributes);
        self.send_rpc(Rpc::ResourceProofResponse {
            candidate,
            destination,
            proof,
        });
    }

    pub fn increment_nodes_work_units(&self) {
        self.action_triggered(ActionTriggered::WorkUnitIncremented);
    }

    pub fn store_merge_infos(&self, merge_info: MergeInfo) {
        self.0.borrow_mut().store_merge_infos(merge_info);
    }

    pub fn has_merge_infos(&self) -> bool {
        self.0.borrow().merge_infos.is_some()
    }

    pub fn merge_needed(&self) -> bool {
        self.0.borrow().merge_needed
    }
}

impl Default for Action {
    fn default() -> Action {
        Action::new(InnerAction::new_with_our_attributes(Attributes::default()))
    }
}

impl Debug for Action {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        self.0.borrow().fmt(formatter)
    }
}

impl PartialEq for Action {
    fn eq(&self, other: &Self) -> bool {
        self.0.borrow().eq(&*other.0.borrow())
    }
}
