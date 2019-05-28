// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use std::fmt::{self, Debug, Formatter};

#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Eq, Ord)]
pub struct Name(pub i32);

impl Distribution<Name> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Name {
        Name(rng.gen_range(-9999, 10000))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Eq, Ord)]
pub struct Age(pub i32);

impl Distribution<Age> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Age {
        Age(rng.gen_range(5, 101))
    }
}

impl Age {
    pub fn increment_by_one(self) -> Age {
        Age(self.0 + 1)
    }
}

#[derive(Clone, Copy, Default, PartialEq, PartialOrd, Eq, Ord)]
pub struct Attributes {
    pub age: Age,
    pub name: Name,
}

impl Debug for Attributes {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "{:?}, {:?}", self.age, self.name)
    }
}

impl Distribution<Attributes> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Attributes {
        Attributes {
            age: rng.gen(),
            name: rng.gen(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct Candidate(pub Attributes);

impl Candidate {
    pub fn name(self) -> Name {
        self.0.name
    }
}

impl Distribution<Candidate> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Candidate {
        Candidate(rng.gen())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Eq, Ord)]
pub struct Node(pub Attributes);

impl Node {
    pub fn name(self) -> Name {
        self.0.name
    }
}

impl Distribution<Node> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Node {
        Node(rng.gen())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeChange {
    AddWithState(Node, State),
    ReplaceWith(Name, Node, State),
    State(Node, State),
    Remove(Name),
    Elder(Node, bool),
}

impl NodeChange {
    pub fn to_event(self) -> Event {
        Event::NodeChange(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct RelocatedInfo {
    pub candidate: Candidate,
    pub expected_age: Age,
    pub target_interval_centre: Name,
    pub section_info: SectionInfo,
}

impl RelocatedInfo {
    #[allow(dead_code)]
    pub fn old_public_id(&self) -> Candidate {
        self.candidate
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub enum State {
    // Online ordered first Online node are chosen for elder
    Online,
    // Relocating an adult that has reached its work unit count
    RelocatingAgeIncrease,
    // Relocating to a new hop with a shorter section prefix
    RelocatingHop,
    // Relocating back online node
    RelocatingBackOnline,
    // Complete relocation, only waiting for info to be processed
    Relocated(RelocatedInfo),
    // Not a full adult / Not known public id: still wait candidate info / connection
    WaitingCandidateInfo(RelocatedInfo),
    // Not a full adult: still wait proofing
    WaitingProofing,
    // When a node that was previous online lost connection
    Offline,
}

impl State {
    pub fn is_relocating(self) -> bool {
        self == State::RelocatingAgeIncrease
            || self == State::RelocatingHop
            || self == State::RelocatingBackOnline
    }

    pub fn waiting_candidate_info(self) -> Option<RelocatedInfo> {
        match self {
            State::WaitingCandidateInfo(info) => Some(info),
            _ => None,
        }
    }

    pub fn is_not_yet_full_node(self) -> bool {
        match self {
            State::WaitingCandidateInfo(_) | State::WaitingProofing | State::RelocatingHop => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeState {
    pub node: Node,
    pub work_units_done: i32,
    pub is_elder: bool,
    pub state: State,
}

impl NodeState {
    pub fn default_elder() -> NodeState {
        NodeState {
            is_elder: true,
            ..NodeState::default()
        }
    }
}

impl Default for NodeState {
    fn default() -> NodeState {
        NodeState {
            node: Default::default(),
            work_units_done: Default::default(),
            is_elder: Default::default(),
            state: State::Online,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Ord, Eq)]
pub struct Section(pub i32);

#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Ord, Eq)]
pub struct SectionInfo(pub Section, pub i32 /*contain full membership */);

impl Distribution<SectionInfo> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> SectionInfo {
        // Avoid randomly generating default `Section(0)`.
        SectionInfo(Section(rng.gen_range(1, i32::max_value())), rng.gen())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Ord, Eq)]
pub struct GenesisPfxInfo(pub SectionInfo);

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
pub enum ChurnNeeded {
    Split,
    Merge,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChangeElder {
    pub changes: Vec<(Node, bool)>,
    pub new_section: SectionInfo,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProofRequest {
    pub value: i32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Proof {
    ValidPart,
    ValidEnd,
    Invalid,
}

impl Proof {
    pub fn is_valid(self) -> bool {
        match self {
            Proof::ValidPart | Proof::ValidEnd => true,
            Proof::Invalid => false,
        }
    }
}

#[derive(Debug, PartialEq, Default, Copy, Clone)]
pub struct ProofSource(pub i32);

impl ProofSource {
    pub fn next_part(&mut self) -> Option<Proof> {
        if self.0 > -1 {
            self.0 -= 1;
        }

        self.resend()
    }

    pub fn resend(self) -> Option<Proof> {
        if self.0 > 0 {
            Some(Proof::ValidPart)
        } else if self.0 == 0 {
            Some(Proof::ValidEnd)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandidateInfo {
    pub old_public_id: Candidate,
    pub new_public_id: Candidate,
    pub destination: Name,
    pub valid: bool,
}

// Event passed to get out of "Wait for" state in flow diagram:
// Pass to try_next to the implementations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaitedEvent {
    Rpc(Rpc),
    ParsecConsensus(ParsecVote),
    LocalEvent(LocalEvent),
}

// Event allowing to drive the tests and collect output, a superset of WaitedEvent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    Rpc(Rpc),
    ParsecConsensus(ParsecVote),
    LocalEvent(LocalEvent),
    TestEvent(TestEvent),

    NodeChange(NodeChange),
    ActionTriggered(ActionTriggered),
}

impl Event {
    pub fn to_waited_event(&self) -> Option<WaitedEvent> {
        match *self {
            Event::Rpc(rpc) => Some(WaitedEvent::Rpc(rpc)),
            Event::ParsecConsensus(parsec_vote) => Some(WaitedEvent::ParsecConsensus(parsec_vote)),
            Event::LocalEvent(local_event) => Some(WaitedEvent::LocalEvent(local_event)),
            Event::TestEvent(_) | Event::NodeChange(_) | Event::ActionTriggered(_) => None,
        }
    }

    pub fn to_test_event(&self) -> Option<TestEvent> {
        match *self {
            Event::TestEvent(test_event) => Some(test_event),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Rpc {
    RefuseCandidate(Candidate),
    RelocateResponse(RelocatedInfo),
    RelocatedInfo(RelocatedInfo),

    ExpectCandidate(Candidate),

    NodeConnected(Candidate, GenesisPfxInfo),

    ResourceProof {
        candidate: Candidate,
        source: Name,
        proof: ProofRequest,
    },
    ResourceProofReceipt {
        candidate: Candidate,
        source: Name,
    },
    NodeApproval(Candidate, GenesisPfxInfo),

    ResourceProofResponse {
        candidate: Candidate,
        destination: Name,
        proof: Proof,
    },
    CandidateInfo(CandidateInfo),

    ConnectionInfoRequest {
        source: Name,
        destination: Name,
        connection_info: i32,
    },
    ConnectionInfoResponse {
        source: Name,
        destination: Name,
        connection_info: i32,
    },

    Merge(SectionInfo),
}

impl Rpc {
    pub fn to_event(&self) -> Event {
        Event::Rpc(*self)
    }

    pub fn destination(&self) -> Option<Name> {
        match self {
            Rpc::RefuseCandidate(_)
            | Rpc::RelocateResponse(_)
            | Rpc::RelocatedInfo(_)
            | Rpc::ExpectCandidate(_)
            | Rpc::Merge(_) => None,

            Rpc::NodeApproval(candidate, _)
            | Rpc::NodeConnected(candidate, _)
            | Rpc::ResourceProof { candidate, .. }
            | Rpc::ResourceProofReceipt { candidate, .. } => Some(candidate.0.name),

            Rpc::ResourceProofResponse { destination, .. }
            | Rpc::CandidateInfo(CandidateInfo { destination, .. })
            | Rpc::ConnectionInfoRequest { destination, .. }
            | Rpc::ConnectionInfoResponse { destination, .. } => Some(*destination),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ParsecVote {
    ExpectCandidate(Candidate),

    Online(Candidate, Candidate),
    PurgeCandidate(Candidate),
    CheckResourceProof,

    AddElderNode(Node),
    RemoveElderNode(Node),
    NewSectionInfo(SectionInfo),

    WorkUnitIncrement,
    CheckRelocate,
    RefuseCandidate(Candidate),
    RelocateResponse(RelocatedInfo),
    RelocatedInfo(RelocatedInfo),

    CheckElder,

    Offline(Node),
    BackOnline(Node),

    NeighbourMerge(SectionInfo),
}

impl ParsecVote {
    pub fn to_event(&self) -> Event {
        Event::ParsecConsensus(*self)
    }

    pub fn candidate(&self) -> Option<Candidate> {
        match self {
            ParsecVote::ExpectCandidate(candidate)
            | ParsecVote::Online(candidate, _)
            | ParsecVote::PurgeCandidate(candidate)
            | ParsecVote::RefuseCandidate(candidate)
            | ParsecVote::RelocateResponse(RelocatedInfo { candidate, .. }) => Some(*candidate),

            ParsecVote::CheckResourceProof
            | ParsecVote::AddElderNode(_)
            | ParsecVote::RemoveElderNode(_)
            | ParsecVote::NewSectionInfo(_)
            | ParsecVote::WorkUnitIncrement
            | ParsecVote::CheckRelocate
            | ParsecVote::RelocatedInfo(_)
            | ParsecVote::CheckElder
            | ParsecVote::Offline(_)
            | ParsecVote::BackOnline(_)
            | ParsecVote::NeighbourMerge(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LocalEvent {
    TimeoutAccept,
    CheckResourceProofTimeout,

    TimeoutWorkUnit,
    TimeoutCheckRelocate,

    TimeoutCheckElder,
    JoiningTimeoutResendInfo,
    JoiningTimeoutConnectRefused,
    JoiningTimeoutProofRefused,
    ResourceProofForElderReady(Name),
    NodeDetectedOffline(Node),
    NodeDetectedBackOnline(Node),
}

impl LocalEvent {
    pub fn to_event(&self) -> Event {
        Event::LocalEvent(*self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TestEvent {
    SetChurnNeeded(ChurnNeeded),
    SetShortestPrefix(Option<Section>),
    SetWorkUnitEnoughToRelocate(Node),
    SetResourceProof(Name, ProofSource),
}

impl TestEvent {
    pub fn to_event(self) -> Event {
        Event::TestEvent(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActionTriggered {
    WorkUnitIncremented,
    MergeInfoStored(SectionInfo),
    OurSectionChanged(SectionInfo),

    CompleteMerge,
    CompleteSplit,

    Scheduled(LocalEvent),
    Killed(LocalEvent),

    ComputeResourceProofForElder(Name),

    // WaitedEvent that should be handled by a flow but are not.
    NotYetImplementedErrorTriggered,
    // Unexpected event ignored.
    UnexpectedEventErrorTriggered,
}

impl ActionTriggered {
    pub fn to_event(self) -> Event {
        Event::ActionTriggered(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TryResult {
    Handled,
    Unhandled,
}
