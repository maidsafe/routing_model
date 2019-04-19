// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use crate::{
    actions::*,
    state::*,
    utilities::{
        Age, Attributes, Candidate, CandidateInfo, Event, GenesisPfxInfo, LocalEvent, MergeInfo,
        Name, Node, NodeChange, NodeState, ParsecVote, Proof, ProofRequest, ProofSource,
        RelocatedInfo, Rpc, Section, SectionInfo, State, TestEvent,
    },
};
use lazy_static::lazy_static;
use pretty_assertions::assert_eq;

macro_rules! to_collect {
    ($($item:expr),*) => {{
        let mut val = Vec::new();
        $(
            let _ = val.push($item.clone());
        )*
        val.into_iter().collect()
    }}
}

const ATTRIBUTES_1_OLD: Attributes = Attributes { name: 1001, age: 9 };
const ATTRIBUTES_1: Attributes = Attributes { name: 1, age: 10 };

const ATTRIBUTES_2_OLD: Attributes = Attributes { name: 1002, age: 9 };
const ATTRIBUTES_2: Attributes = Attributes { name: 2, age: 10 };

const CANDIDATE_1_OLD: Candidate = Candidate(ATTRIBUTES_1_OLD);
const CANDIDATE_1: Candidate = Candidate(ATTRIBUTES_1);
const CANDIDATE_2_OLD: Candidate = Candidate(ATTRIBUTES_2_OLD);
const CANDIDATE_2: Candidate = Candidate(ATTRIBUTES_2);
const CANDIDATE_130: Candidate = Candidate(Attributes { name: 130, age: 30 });
const CANDIDATE_205: Candidate = Candidate(Attributes { name: 205, age: 5 });
const OTHER_SECTION_1: Section = Section(1);
const DST_SECTION_200: Section = Section(200);

const NODE_1_OLD: Node = Node(ATTRIBUTES_1_OLD);
const NODE_1: Node = Node(ATTRIBUTES_1);
const NODE_2_OLD: Node = Node(ATTRIBUTES_2_OLD);
const NODE_2: Node = Node(ATTRIBUTES_2);
const SET_ONLINE_NODE_1: NodeChange = NodeChange::State(Node(ATTRIBUTES_1), State::Online);
const REMOVE_NODE_1: NodeChange = NodeChange::Remove(Name(ATTRIBUTES_1.name));

const NODE_ELDER_109: Node = Node(Attributes { name: 109, age: 9 });
const NODE_ELDER_110: Node = Node(Attributes { name: 110, age: 10 });
const NODE_ELDER_111: Node = Node(Attributes { name: 111, age: 11 });
const NODE_ELDER_130: Node = Node(Attributes { name: 130, age: 30 });
const NODE_ELDER_131: Node = Node(Attributes { name: 131, age: 31 });
const NODE_ELDER_132: Node = Node(Attributes { name: 132, age: 32 });

const NAME_109: Name = Name(NODE_ELDER_109.0.name);
const NAME_110: Name = Name(NODE_ELDER_110.0.name);
const NAME_111: Name = Name(NODE_ELDER_111.0.name);

const YOUNG_ADULT_205: Node = Node(Attributes { name: 205, age: 5 });
const SECTION_INFO_1: SectionInfo = SectionInfo(OUR_SECTION, 1);
const SECTION_INFO_2: SectionInfo = SectionInfo(OUR_SECTION, 2);
const DST_SECTION_INFO_200: SectionInfo = SectionInfo(DST_SECTION_200, 0);

const CANDIDATE_INFO_VALID_1: CandidateInfo = CandidateInfo {
    old_public_id: CANDIDATE_1_OLD,
    new_public_id: CANDIDATE_1,
    destination: TARGET_INTERVAL_1,
    valid: true,
};

const CANDIDATE_RELOCATED_INFO_1: RelocatedInfo = RelocatedInfo {
    candidate: CANDIDATE_1_OLD,
    expected_age: Age(CANDIDATE_1_OLD.0.age + 1),
    target_interval_centre: TARGET_INTERVAL_1,
    section_info: OUR_INITIAL_SECTION_INFO,
};

const CANDIDATE_INFO_VALID_RPC_1: Rpc = Rpc::CandidateInfo(CANDIDATE_INFO_VALID_1);
const CANDIDATE_INFO_VALID_PARSEC_VOTE_1: ParsecVote =
    ParsecVote::CandidateConnected(CANDIDATE_INFO_VALID_1);
const TARGET_INTERVAL_1: Name = Name(1234);
const TARGET_INTERVAL_2: Name = Name(1235);

const OUR_SECTION: Section = Section(0);
const OUR_NODE: Node = NODE_ELDER_132;
const OUR_NAME: Name = Name(OUR_NODE.0.name);
const OUR_NODE_CANDIDATE: Candidate = Candidate(NODE_ELDER_132.0);
const OUR_PROOF_REQUEST: ProofRequest = ProofRequest { value: OUR_NAME.0 };
const OUR_INITIAL_SECTION_INFO: SectionInfo = SectionInfo(OUR_SECTION, 0);
const OUR_NEXT_SECTION_INFO: SectionInfo = SectionInfo(OUR_SECTION, 1);
const OUR_GENESIS_INFO: GenesisPfxInfo = GenesisPfxInfo(OUR_INITIAL_SECTION_INFO);

lazy_static! {
    static ref INNER_ACTION_132: InnerAction = InnerAction::new_with_our_attributes(OUR_NODE.0)
        .with_next_target_interval(TARGET_INTERVAL_1);
    static ref INNER_ACTION_YOUNG_ELDERS: InnerAction = INNER_ACTION_132
        .clone()
        .extend_current_nodes_with(
            &NodeState {
                is_elder: true,
                ..NodeState::default()
            },
            &[NODE_ELDER_109, NODE_ELDER_110, NODE_ELDER_132]
        )
        .extend_current_nodes_with(&NodeState::default(), &[YOUNG_ADULT_205]);
    static ref INNER_ACTION_OLD_ELDERS: InnerAction = INNER_ACTION_132
        .clone()
        .extend_current_nodes_with(
            &NodeState {
                is_elder: true,
                ..NodeState::default()
            },
            &[NODE_ELDER_130, NODE_ELDER_131, NODE_ELDER_132]
        )
        .extend_current_nodes_with(&NodeState::default(), &[YOUNG_ADULT_205]);
    static ref INNER_ACTION_YOUNG_ELDERS_WITH_WAITING_ELDER: InnerAction = INNER_ACTION_132
        .clone()
        .extend_current_nodes_with(
            &NodeState {
                is_elder: true,
                ..NodeState::default()
            },
            &[NODE_ELDER_109, NODE_ELDER_110, NODE_ELDER_111]
        )
        .extend_current_nodes_with(&NodeState::default(), &[NODE_ELDER_130]);
    static ref INNER_ACTION_WITH_DST_SECTION_200: InnerAction =
        INNER_ACTION_132.clone().with_section_members(
            DST_SECTION_INFO_200,
            &[NODE_ELDER_109, NODE_ELDER_110, NODE_ELDER_111]
        );
}

#[derive(Debug, PartialEq, Default, Clone)]
struct AssertState {
    action_our_events: Vec<Event>,
    action_our_section: SectionInfo,
    action_merge_infos: Option<MergeInfo>,
}

fn process_events(mut state: MemberState, events: &[Event]) -> MemberState {
    for event in events.iter().cloned() {
        state = match state.try_next(event) {
            Some(next_state) => next_state,
            None => state.failure_event(event),
        };

        if state.failure.is_some() {
            break;
        }
    }

    state
}

fn run_test(
    test_name: &str,
    start_state: &MemberState,
    events: &[Event],
    expected_state: &AssertState,
) {
    let final_state = process_events(start_state.clone(), &events);
    let action = final_state.action.inner();

    let final_state = (
        AssertState {
            action_our_events: action.our_events,
            action_our_section: action.our_section,
            action_merge_infos: action.merge_infos,
        },
        final_state.failure,
    );
    let expected_state = (expected_state.clone(), None);

    assert_eq!(expected_state, final_state, "{}", test_name);
}

fn arrange_initial_state(state: &MemberState, events: &[Event]) -> MemberState {
    let state = process_events(state.clone(), events);
    state.action.remove_processed_state();
    state
}

fn initial_state_young_elders() -> MemberState {
    MemberState {
        action: Action::new(INNER_ACTION_YOUNG_ELDERS.clone()),
        ..Default::default()
    }
}

fn initial_state_old_elders() -> MemberState {
    MemberState {
        action: Action::new(INNER_ACTION_OLD_ELDERS.clone()),
        ..Default::default()
    }
}

fn get_relocated_info(candidate: Candidate, section_info: SectionInfo) -> RelocatedInfo {
    RelocatedInfo {
        candidate,
        expected_age: Age(candidate.0.age + 1),
        target_interval_centre: TARGET_INTERVAL_1,
        section_info,
    }
}

//////////////////
/// Dst
//////////////////

mod dst_tests {
    use super::*;

    #[test]
    fn rpc_expect_candidate() {
        run_test(
            "Get RPC ExpectCandidate",
            &initial_state_old_elders(),
            &[Rpc::ExpectCandidate(CANDIDATE_1_OLD).to_event()],
            &AssertState {
                action_our_events: vec![ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate() {
        run_test(
            "Get Parsec ExpectCandidate",
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    NodeChange::AddWithState(
                        Node(Attributes {
                            name: TARGET_INTERVAL_1.0,
                            age: CANDIDATE_1.0.age,
                        }),
                        State::WaitingCandidateInfo(CANDIDATE_RELOCATED_INFO_1),
                    )
                    .to_event(),
                    Rpc::RelocateResponse(CANDIDATE_RELOCATED_INFO_1).to_event(),
                    LocalEvent::CheckResourceProofTimeout.to_event(),
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_candidate_twice() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event()],
        );

        run_test(
            "Get Parsec ExpectCandidate then Purge",
            &initial_state,
            &[ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event()],
            &AssertState {
                action_our_events: vec![
                    Rpc::RelocateResponse(CANDIDATE_RELOCATED_INFO_1).to_event()
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_candidate_info() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Purge",
            &initial_state,
            &[CANDIDATE_INFO_VALID_RPC_1.to_event()],
            &AssertState {
                action_our_events: vec![Rpc::ConnectionInfoRequest {
                    source: OUR_NAME,
                    destination: CANDIDATE_1.name(),
                    connection_info: OUR_NAME.0,
                }
                .to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_candidate_info_twice() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                CANDIDATE_INFO_VALID_RPC_1.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Purge",
            &initial_state,
            &[CANDIDATE_INFO_VALID_RPC_1.to_event()],
            &AssertState {
                action_our_events: vec![Rpc::ConnectionInfoRequest {
                    source: OUR_NAME,
                    destination: CANDIDATE_1.name(),
                    connection_info: OUR_NAME.0,
                }
                .to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_candidate_info_and_connect_response() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
                CANDIDATE_INFO_VALID_RPC_1.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Purge",
            &initial_state,
            &[Rpc::ConnectionInfoResponse {
                source: CANDIDATE_1.name(),
                destination: TARGET_INTERVAL_1,
                connection_info: 0,
            }
            .to_event()],
            &AssertState {
                action_our_events: vec![CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_candidate_info_twice_and_connect_response_twice() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
                CANDIDATE_INFO_VALID_RPC_1.to_event(),
                CANDIDATE_INFO_VALID_RPC_1.to_event(),
                Rpc::ConnectionInfoResponse {
                    source: CANDIDATE_1.name(),
                    destination: TARGET_INTERVAL_1,
                    connection_info: 0,
                }
                .to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Purge",
            &initial_state,
            &[Rpc::ConnectionInfoResponse {
                source: CANDIDATE_1.name(),
                destination: TARGET_INTERVAL_1,
                connection_info: 0,
            }
            .to_event()],
            &AssertState {
                action_our_events: vec![LocalEvent::NotYetImplementedEvent.to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_parsec_candidate_info() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event()],
        );

        run_test(
            "Get Parsec ExpectCandidate then Purge",
            &initial_state,
            &[
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    NodeChange::ReplaceWith(TARGET_INTERVAL_1, NODE_1, State::WaitingProofing)
                        .to_event(),
                    Rpc::NodeConnected(CANDIDATE_1, OUR_GENESIS_INFO).to_event(),
                    Rpc::RefuseCandidate(CANDIDATE_1_OLD).to_event(),
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_parsec_candidate_info_with_shorter_section_exists() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                TestEvent::SetShortestPrefix(Some(OTHER_SECTION_1)).to_event(),
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Purge",
            &initial_state,
            &[
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckRelocate.to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    NodeChange::ReplaceWith(TARGET_INTERVAL_1, NODE_1, State::RelocatingHop)
                        .to_event(),
                    Rpc::NodeConnected(CANDIDATE_1, OUR_GENESIS_INFO).to_event(),
                    Rpc::RefuseCandidate(CANDIDATE_1_OLD).to_event(),
                    Rpc::ExpectCandidate(CANDIDATE_1).to_event(),
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_candidate_info_after_parsec_candidate_info() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Purge",
            &initial_state,
            &[CANDIDATE_INFO_VALID_RPC_1.to_event()],
            &AssertState::default(),
        );
    }

    #[test]
    fn parsec_expect_candidate_then_check_too_long_timeout() {
        run_test(
            "Timeout trigger a vote",
            &initial_state_old_elders(),
            &[LocalEvent::CheckRelocatedNodeConnectionTimeout.to_event()],
            &AssertState {
                action_our_events: vec![ParsecVote::CheckRelocatedNodeConnection.to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_check_too_long_twice() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckRelocatedNodeConnection.to_event(),
            ],
        );
        run_test(
            "Drop connecting node still connecting after two CheckRelocatedNodeConnection",
            &initial_state,
            &[
                ParsecVote::CheckRelocatedNodeConnection.to_event(),
                CANDIDATE_INFO_VALID_RPC_1.to_event(),
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    NodeChange::Remove(TARGET_INTERVAL_1).to_event(),
                    LocalEvent::CheckRelocatedNodeConnectionTimeout.to_event(),
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_check_too_long_twice_after_valid_info_rpc() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                CANDIDATE_INFO_VALID_RPC_1.to_event(),
                ParsecVote::CheckRelocatedNodeConnection.to_event(),
            ],
        );
        run_test(
            "Drop connecting node still connecting after two CheckRelocatedNodeConnection",
            &initial_state,
            &[
                ParsecVote::CheckRelocatedNodeConnection.to_event(),
                Rpc::ConnectionInfoResponse {
                    source: CANDIDATE_1.name(),
                    destination: TARGET_INTERVAL_1,
                    connection_info: 0,
                }
                .to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    NodeChange::Remove(TARGET_INTERVAL_1).to_event(),
                    LocalEvent::CheckRelocatedNodeConnectionTimeout.to_event(),
                    LocalEvent::NotYetImplementedEvent.to_event(),
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_invalid_candidate_info() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                //ParsecVote::CheckResourceProof.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Purge",
            &initial_state,
            &[Rpc::CandidateInfo(CandidateInfo {
                old_public_id: CANDIDATE_1_OLD,
                new_public_id: CANDIDATE_1,
                destination: OUR_NAME,
                valid: false,
            })
            .to_event()],
            &AssertState::default(),
        );
    }

    #[test]
    fn parsec_expect_candidate_then_time_out() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Purge",
            &initial_state,
            &[LocalEvent::TimeoutAccept.to_event()],
            &AssertState {
                action_our_events: vec![ParsecVote::PurgeCandidate(CANDIDATE_1).to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_wrong_candidate_info() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Purge",
            &initial_state,
            &[Rpc::CandidateInfo(CandidateInfo {
                old_public_id: CANDIDATE_2,
                new_public_id: CANDIDATE_2,
                destination: OUR_NAME,
                valid: true,
            })
            .to_event()],
            &AssertState::default(),
        );
    }

    #[test]
    fn parsec_expect_candidate_then_candidate_info_then_check_resource_proof() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Purge",
            &initial_state,
            &[ParsecVote::CheckResourceProof.to_event()],
            &AssertState {
                action_our_events: vec![Rpc::ResourceProof {
                    candidate: CANDIDATE_1,
                    source: OUR_NAME,
                    proof: OUR_PROOF_REQUEST,
                }
                .to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_candidate_info_then_part_proof() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Purge",
            &initial_state,
            &[Rpc::ResourceProofResponse {
                candidate: CANDIDATE_1,
                destination: OUR_NAME,
                proof: Proof::ValidPart,
            }
            .to_event()],
            &AssertState {
                action_our_events: vec![Rpc::ResourceProofReceipt {
                    candidate: CANDIDATE_1,
                    source: OUR_NAME,
                }
                .to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_candidate_info_then_end_proof() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Purge",
            &initial_state,
            &[Rpc::ResourceProofResponse {
                candidate: CANDIDATE_1,
                destination: OUR_NAME,
                proof: Proof::ValidEnd,
            }
            .to_event()],
            &AssertState {
                action_our_events: vec![ParsecVote::Online(CANDIDATE_1).to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_candidate_info_then_end_proof_twice() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
                ParsecVote::CheckResourceProof.to_event(),
                Rpc::ResourceProofResponse {
                    candidate: CANDIDATE_1,
                    destination: OUR_NAME,
                    proof: Proof::ValidEnd,
                }
                .to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Purge",
            &initial_state,
            &[Rpc::ResourceProofResponse {
                candidate: CANDIDATE_1,
                destination: OUR_NAME,
                proof: Proof::ValidEnd,
            }
            .to_event()],
            &AssertState::default(),
        );
    }

    #[test]
    fn parsec_expect_candidate_then_candidate_info_then_invalid_proof() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Purge",
            &initial_state,
            &[Rpc::ResourceProofResponse {
                candidate: CANDIDATE_1,
                destination: OUR_NAME,
                proof: Proof::Invalid,
            }
            .to_event()],
            &AssertState::default(),
        );
    }

    #[test]
    fn parsec_expect_candidate_then_candidate_info_then_end_proof_wrong_candidate() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Purge",
            &initial_state,
            &[Rpc::ResourceProofResponse {
                candidate: CANDIDATE_2,
                destination: OUR_NAME,
                proof: Proof::ValidEnd,
            }
            .to_event()],
            &AssertState::default(),
        );
    }

    #[test]
    fn parsec_expect_candidate_then_purge_and_online_for_wrong_candidate() {
        let initial_state = arrange_initial_state(
            &initial_state_young_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Purge",
            &initial_state,
            &[
                ParsecVote::Online(CANDIDATE_2).to_event(),
                ParsecVote::PurgeCandidate(CANDIDATE_2).to_event(),
            ],
            &AssertState::default(),
        );
    }

    #[test]
    fn rpc_merge() {
        run_test(
            "Get RPC Merge",
            &initial_state_old_elders(),
            &[Rpc::Merge.to_event()],
            &AssertState {
                action_our_events: vec![ParsecVote::NeighbourMerge(MergeInfo).to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_neighbour_merge() {
        run_test(
            "Get Parsec NeighbourMergeInfo",
            &initial_state_old_elders(),
            &[ParsecVote::NeighbourMerge(MergeInfo).to_event()],
            &AssertState {
                action_merge_infos: Some(MergeInfo),
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_neighbour_merge_then_check_elder() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[ParsecVote::NeighbourMerge(MergeInfo).to_event()],
        );

        run_test(
            "Get Parsec NeighbourMergeInfo then CheckElder",
            &initial_state,
            &[ParsecVote::CheckElder.to_event()],
            &AssertState {
                action_merge_infos: Some(MergeInfo),
                action_our_events: vec![Rpc::Merge.to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_merge_needed() {
        let initial_state = initial_state_old_elders();

        run_test(
            "Merge needed",
            &initial_state,
            &[
                TestEvent::SetMergeNeeded(true).to_event(),
                ParsecVote::CheckElder.to_event(),
            ],
            &AssertState {
                action_our_events: vec![Rpc::Merge.to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_online_no_elder_change() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Online (No Elder Change)",
            &initial_state,
            &[
                ParsecVote::Online(CANDIDATE_1).to_event(),
                ParsecVote::CheckElder.to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    SET_ONLINE_NODE_1.to_event(),
                    Rpc::NodeApproval(CANDIDATE_1, OUR_GENESIS_INFO).to_event(),
                    LocalEvent::CheckResourceProofTimeout.to_event(),
                    LocalEvent::TimeoutCheckElder.to_event(),
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_online_elder_change() {
        let initial_state = arrange_initial_state(
            &initial_state_young_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Online (Elder Change)",
            &initial_state,
            &[
                ParsecVote::Online(CANDIDATE_1).to_event(),
                ParsecVote::CheckElder.to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    SET_ONLINE_NODE_1.to_event(),
                    Rpc::NodeApproval(CANDIDATE_1, OUR_GENESIS_INFO).to_event(),
                    LocalEvent::CheckResourceProofTimeout.to_event(),
                    ParsecVote::AddElderNode(NODE_1).to_event(),
                    ParsecVote::RemoveElderNode(NODE_ELDER_109).to_event(),
                    ParsecVote::NewSectionInfo(SECTION_INFO_1).to_event(),
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_online_elder_change_get_wrong_votes() {
        let initial_state = arrange_initial_state(
            &initial_state_young_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
                ParsecVote::CheckResourceProof.to_event(),
                ParsecVote::Online(CANDIDATE_1).to_event(),
                ParsecVote::CheckElder.to_event(),
            ],
        );

        let description =
            "Get Parsec ExpectCandidate then Online (Elder Change) RemoveElderNode \
             for wrong elder, AddElderNode for wrong node, NewSectionInfo for wrong section";
        run_test(
            description,
            &initial_state,
            &[
                ParsecVote::RemoveElderNode(NODE_1).to_event(),
                ParsecVote::AddElderNode(NODE_ELDER_109).to_event(),
                ParsecVote::NewSectionInfo(SECTION_INFO_2).to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    LocalEvent::UnexpectedEventIgnored.to_event(),
                    LocalEvent::UnexpectedEventIgnored.to_event(),
                    LocalEvent::UnexpectedEventIgnored.to_event(),
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_online_elder_change_remove_elder() {
        let initial_state = arrange_initial_state(
            &initial_state_young_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
                ParsecVote::CheckResourceProof.to_event(),
                ParsecVote::Online(CANDIDATE_1).to_event(),
                ParsecVote::CheckElder.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Online (Elder Change) then RemoveElderNode",
            &initial_state,
            &[ParsecVote::RemoveElderNode(NODE_ELDER_109).to_event()],
            &AssertState::default(),
        );
    }

    #[test]
    fn parsec_expect_candidate_then_online_elder_change_complete_elder() {
        let initial_state = arrange_initial_state(
            &initial_state_young_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
                ParsecVote::CheckResourceProof.to_event(),
                ParsecVote::Online(CANDIDATE_1).to_event(),
                ParsecVote::CheckElder.to_event(),
                ParsecVote::RemoveElderNode(NODE_ELDER_109).to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Online (Elder Change) then \
             RemoveElderNode, AddElderNode, NewSectionInfo",
            &initial_state,
            &[
                ParsecVote::AddElderNode(NODE_1).to_event(),
                ParsecVote::NewSectionInfo(SECTION_INFO_1).to_event(),
            ],
            &AssertState {
                action_our_section: SECTION_INFO_1,
                action_our_events: vec![
                    NodeChange::Elder(NODE_1, true).to_event(),
                    NodeChange::Elder(NODE_ELDER_109, false).to_event(),
                    LocalEvent::TimeoutCheckElder.to_event(),
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_when_candidate_completed_with_elder_change() {
        let initial_state = arrange_initial_state(
            &initial_state_young_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
                ParsecVote::CheckResourceProof.to_event(),
                ParsecVote::Online(CANDIDATE_1).to_event(),
                ParsecVote::CheckElder.to_event(),
                ParsecVote::RemoveElderNode(NODE_ELDER_109).to_event(),
                ParsecVote::AddElderNode(NODE_1).to_event(),
                ParsecVote::NewSectionInfo(SECTION_INFO_1).to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate after first candidate completed \
             with elder change",
            &initial_state,
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_2_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
            &&AssertState {
                action_our_section: SECTION_INFO_1,
                action_our_events: vec![
                    NodeChange::AddWithState(
                        Node(Attributes {
                            name: TARGET_INTERVAL_2.0,
                            age: CANDIDATE_2.0.age,
                        }),
                        State::WaitingCandidateInfo(RelocatedInfo {
                            candidate: CANDIDATE_2_OLD,
                            expected_age: CANDIDATE_2.0.age(),
                            target_interval_centre: TARGET_INTERVAL_2,
                            section_info: OUR_NEXT_SECTION_INFO,
                        }),
                    )
                    .to_event(),
                    Rpc::RelocateResponse(RelocatedInfo {
                        candidate: CANDIDATE_2_OLD,
                        expected_age: CANDIDATE_2.0.age(),
                        target_interval_centre: TARGET_INTERVAL_2,
                        section_info: OUR_NEXT_SECTION_INFO,
                    })
                    .to_event(),
                    LocalEvent::CheckResourceProofTimeout.to_event(),
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_purge() {
        let initial_state = arrange_initial_state(
            &initial_state_young_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Purge",
            &initial_state,
            &[ParsecVote::PurgeCandidate(CANDIDATE_1).to_event()],
            &AssertState {
                action_our_events: vec![
                    REMOVE_NODE_1.to_event(),
                    LocalEvent::CheckResourceProofTimeout.to_event(),
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_twice() {
        let initial_state = arrange_initial_state(
            &initial_state_young_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                CANDIDATE_INFO_VALID_PARSEC_VOTE_1.to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
        );

        run_test(
            &"Get Parsec 2 ExpectCandidate",
            &initial_state,
            &[ParsecVote::ExpectCandidate(CANDIDATE_2_OLD).to_event()],
            &AssertState {
                action_our_events: vec![Rpc::RefuseCandidate(CANDIDATE_2_OLD).to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_unexpected_purge_online() {
        run_test(
            "Get unexpected Parsec consensus Online and PurgeCandidate. \
             Candidate may have trigger both vote: only consider the first",
            &initial_state_old_elders(),
            &[
                ParsecVote::Online(CANDIDATE_1).to_event(),
                ParsecVote::PurgeCandidate(CANDIDATE_1).to_event(),
            ],
            &AssertState::default(),
        );
    }

    #[test]
    fn rpc_unexpected_candidate_info_resource_proof_response() {
        run_test(
            "Get unexpected RPC CandidateInfo and ResourceProofResponse. \
             Candidate RPC may arrive after candidate was purged or accepted",
            &initial_state_old_elders(),
            &[
                CANDIDATE_INFO_VALID_RPC_1.to_event(),
                Rpc::ResourceProofResponse {
                    candidate: CANDIDATE_1,
                    destination: OUR_NAME,
                    proof: Proof::ValidEnd,
                }
                .to_event(),
            ],
            &AssertState::default(),
        );
    }

    #[test]
    fn local_events_offline_online_again_for_different_nodes() {
        run_test(
            "Get local event node detected offline online again different nodes",
            &initial_state_old_elders(),
            &[
                LocalEvent::NodeDetectedOffline(NODE_ELDER_130).to_event(),
                LocalEvent::NodeDetectedBackOnline(NODE_ELDER_131).to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    ParsecVote::Offline(NODE_ELDER_130).to_event(),
                    ParsecVote::BackOnline(NODE_ELDER_131).to_event(),
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_offline() {
        run_test(
            "Get parsec consensus offline",
            &initial_state_old_elders(),
            &[ParsecVote::Offline(NODE_ELDER_130).to_event()],
            &AssertState {
                action_our_events: vec![
                    NodeChange::State(NODE_ELDER_130, State::Offline).to_event()
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_offline_then_check_elder() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[ParsecVote::Offline(NODE_ELDER_130).to_event()],
        );
        run_test(
            "Get parsec consensus offline then check elder",
            &initial_state,
            &[ParsecVote::CheckElder.to_event()],
            &AssertState {
                action_our_events: vec![
                    ParsecVote::AddElderNode(YOUNG_ADULT_205).to_event(),
                    ParsecVote::RemoveElderNode(NODE_ELDER_130).to_event(),
                    ParsecVote::NewSectionInfo(SECTION_INFO_1).to_event(),
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_offline_then_parsec_online() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[ParsecVote::Offline(NODE_ELDER_130).to_event()],
        );
        run_test(
            "Get parsec consensus offline then parsec online",
            &initial_state,
            &[ParsecVote::BackOnline(NODE_ELDER_130).to_event()],
            &AssertState {
                action_our_events: vec![NodeChange::State(
                    NODE_ELDER_130,
                    State::RelocatingBackOnline,
                )
                .to_event()],
                ..AssertState::default()
            },
        );
    }
}

//////////////////
/// Src
//////////////////

mod src_tests {
    use super::*;

    #[test]
    fn local_event_time_out_work_unit() {
        run_test(
            "",
            &initial_state_old_elders(),
            &[LocalEvent::TimeoutWorkUnit.to_event()],
            &AssertState {
                action_our_events: vec![
                    ParsecVote::WorkUnitIncrement.to_event(),
                    LocalEvent::TimeoutWorkUnit.to_event(),
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn start_relocation() {
        run_test(
            "Trigger events to relocate node",
            &initial_state_old_elders(),
            &[
                TestEvent::SetWorkUnitEnoughToRelocate(YOUNG_ADULT_205).to_event(),
                ParsecVote::WorkUnitIncrement.to_event(),
                ParsecVote::CheckRelocate.to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    NodeChange::State(YOUNG_ADULT_205, State::RelocatingAgeIncrease).to_event(),
                    Rpc::ExpectCandidate(CANDIDATE_205).to_event(),
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_check_relocate_trigger_again_no_retry() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                TestEvent::SetWorkUnitEnoughToRelocate(YOUNG_ADULT_205).to_event(),
                ParsecVote::WorkUnitIncrement.to_event(),
                ParsecVote::CheckRelocate.to_event(),
            ],
        );

        run_test(
            "Additional CheckRelocate do not trigger a resend",
            &initial_state,
            &[
                ParsecVote::CheckRelocate.to_event(),
                ParsecVote::CheckRelocate.to_event(),
            ],
            &AssertState::default(),
        );
    }

    #[test]
    fn parsec_relocation_trigger_again_until_retry() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                TestEvent::SetWorkUnitEnoughToRelocate(YOUNG_ADULT_205).to_event(),
                ParsecVote::WorkUnitIncrement.to_event(),
                ParsecVote::CheckRelocate.to_event(),
                ParsecVote::CheckRelocate.to_event(),
                ParsecVote::CheckRelocate.to_event(),
            ],
        );

        run_test(
            "Enough additional CheckRelocate trigger a resend",
            &initial_state,
            &[ParsecVote::CheckRelocate.to_event()],
            &AssertState {
                action_our_events: vec![Rpc::ExpectCandidate(CANDIDATE_205).to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_check_relocate_trigger_again_with_relocating_hop_and_back_online() {
        let initial_state = MemberState {
            action: Action::new(
                INNER_ACTION_OLD_ELDERS
                    .clone()
                    .extend_current_nodes_with(
                        &NodeState {
                            state: State::RelocatingHop,
                            ..NodeState::default()
                        },
                        &[NODE_1_OLD],
                    )
                    .extend_current_nodes_with(
                        &NodeState {
                            state: State::RelocatingBackOnline,
                            ..NodeState::default()
                        },
                        &[NODE_2, NODE_2_OLD, NODE_1],
                    ),
            ),
            ..MemberState::default()
        };

        let description = "RelocatingHop or RelocatingBackOnline does not stop relocating our \
        adults. Also relocated nodes are relocated AgeIncrease, then Hop, then BackOnline, break \
        tie by age then name";
        run_test(
            description,
            &initial_state,
            &[
                TestEvent::SetWorkUnitEnoughToRelocate(YOUNG_ADULT_205).to_event(),
                ParsecVote::WorkUnitIncrement.to_event(),
                ParsecVote::CheckRelocate.to_event(),
                ParsecVote::CheckRelocate.to_event(),
                ParsecVote::CheckRelocate.to_event(),
                ParsecVote::CheckRelocate.to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    NodeChange::State(YOUNG_ADULT_205, State::RelocatingAgeIncrease).to_event(),
                    Rpc::ExpectCandidate(CANDIDATE_205).to_event(),
                    Rpc::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                    Rpc::ExpectCandidate(CANDIDATE_2).to_event(),
                    Rpc::ExpectCandidate(CANDIDATE_205).to_event(),
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_relocate_trigger_elder_change() {
        run_test(
            "Get Parsec ExpectCandidate then Online (Elder Change)",
            &initial_state_old_elders(),
            &[
                TestEvent::SetWorkUnitEnoughToRelocate(NODE_ELDER_130).to_event(),
                ParsecVote::WorkUnitIncrement.to_event(),
                ParsecVote::CheckRelocate.to_event(),
                ParsecVote::CheckElder.to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    NodeChange::State(NODE_ELDER_130, State::RelocatingAgeIncrease).to_event(),
                    ParsecVote::AddElderNode(YOUNG_ADULT_205).to_event(),
                    ParsecVote::RemoveElderNode(NODE_ELDER_130).to_event(),
                    ParsecVote::NewSectionInfo(SECTION_INFO_1).to_event(),
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_relocate_trigger_elder_change_complete() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                TestEvent::SetWorkUnitEnoughToRelocate(NODE_ELDER_130).to_event(),
                ParsecVote::WorkUnitIncrement.to_event(),
                ParsecVote::CheckElder.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate then Online (Elder Change)",
            &initial_state,
            &[
                ParsecVote::RemoveElderNode(NODE_ELDER_130).to_event(),
                ParsecVote::AddElderNode(YOUNG_ADULT_205).to_event(),
                ParsecVote::NewSectionInfo(SECTION_INFO_1).to_event(),
                ParsecVote::CheckRelocate.to_event(),
            ],
            &AssertState {
                action_our_section: SECTION_INFO_1,
                action_our_events: vec![
                    NodeChange::Elder(YOUNG_ADULT_205, true).to_event(),
                    NodeChange::Elder(NODE_ELDER_130, false).to_event(),
                    LocalEvent::TimeoutCheckElder.to_event(),
                    Rpc::ExpectCandidate(CANDIDATE_130).to_event(),
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_relocation_trigger_refuse_candidate_rpc() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                TestEvent::SetWorkUnitEnoughToRelocate(YOUNG_ADULT_205).to_event(),
                ParsecVote::WorkUnitIncrement.to_event(),
                ParsecVote::CheckRelocate.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate",
            &initial_state,
            &[Rpc::RefuseCandidate(CANDIDATE_205).to_event()],
            &AssertState {
                action_our_events: vec![ParsecVote::RefuseCandidate(CANDIDATE_205).to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_relocation_trigger_relocate_response_rpc() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                TestEvent::SetWorkUnitEnoughToRelocate(YOUNG_ADULT_205).to_event(),
                ParsecVote::WorkUnitIncrement.to_event(),
                ParsecVote::CheckRelocate.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate",
            &initial_state,
            &[
                Rpc::RelocateResponse(get_relocated_info(CANDIDATE_205, DST_SECTION_INFO_200))
                    .to_event(),
            ],
            &AssertState {
                action_our_events: vec![ParsecVote::RelocateResponse(get_relocated_info(
                    CANDIDATE_205,
                    DST_SECTION_INFO_200,
                ))
                .to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_relocation_trigger_accept() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                TestEvent::SetWorkUnitEnoughToRelocate(YOUNG_ADULT_205).to_event(),
                ParsecVote::WorkUnitIncrement.to_event(),
                ParsecVote::CheckRelocate.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate",
            &initial_state,
            &[
                ParsecVote::RelocateResponse(get_relocated_info(
                    CANDIDATE_205,
                    DST_SECTION_INFO_200,
                ))
                .to_event(),
                ParsecVote::RelocatedInfo(get_relocated_info(CANDIDATE_205, DST_SECTION_INFO_200))
                    .to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    NodeChange::State(
                        YOUNG_ADULT_205,
                        State::Relocated(get_relocated_info(CANDIDATE_205, DST_SECTION_INFO_200)),
                    )
                    .to_event(),
                    ParsecVote::RelocatedInfo(get_relocated_info(
                        CANDIDATE_205,
                        DST_SECTION_INFO_200,
                    ))
                    .to_event(),
                    Rpc::RelocatedInfo(get_relocated_info(CANDIDATE_205, DST_SECTION_INFO_200))
                        .to_event(),
                    NodeChange::Remove(YOUNG_ADULT_205.name()).to_event(),
                ],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_relocation_trigger_refuse() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                TestEvent::SetWorkUnitEnoughToRelocate(YOUNG_ADULT_205).to_event(),
                ParsecVote::WorkUnitIncrement.to_event(),
                ParsecVote::CheckRelocate.to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate",
            &initial_state,
            &[ParsecVote::RefuseCandidate(CANDIDATE_205).to_event()],
            &AssertState::default(),
        );
    }

    #[test]
    fn parsec_relocation_trigger_refuse_trigger_again() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                TestEvent::SetWorkUnitEnoughToRelocate(YOUNG_ADULT_205).to_event(),
                ParsecVote::WorkUnitIncrement.to_event(),
                ParsecVote::CheckRelocate.to_event(),
                ParsecVote::RefuseCandidate(CANDIDATE_205).to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate",
            &initial_state,
            &[ParsecVote::CheckRelocate.to_event()],
            &AssertState {
                action_our_events: vec![Rpc::ExpectCandidate(CANDIDATE_205).to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn parsec_relocation_trigger_elder_change_refuse_trigger_again() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                TestEvent::SetWorkUnitEnoughToRelocate(NODE_ELDER_130).to_event(),
                ParsecVote::WorkUnitIncrement.to_event(),
                ParsecVote::CheckElder.to_event(),
                ParsecVote::RemoveElderNode(NODE_ELDER_130).to_event(),
                ParsecVote::AddElderNode(YOUNG_ADULT_205).to_event(),
                ParsecVote::NewSectionInfo(SECTION_INFO_1).to_event(),
                ParsecVote::CheckRelocate.to_event(),
                ParsecVote::RefuseCandidate(CANDIDATE_130).to_event(),
            ],
        );

        run_test(
            "Get Parsec ExpectCandidate",
            &initial_state,
            &[ParsecVote::CheckRelocate.to_event()],
            &AssertState {
                action_our_section: SECTION_INFO_1,
                action_our_events: vec![Rpc::ExpectCandidate(CANDIDATE_130).to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn unexpected_refuse_candidate() {
        run_test(
            "Get RPC ExpectCandidate",
            &initial_state_old_elders(),
            &[Rpc::RefuseCandidate(CANDIDATE_205).to_event()],
            &AssertState {
                action_our_events: vec![ParsecVote::RefuseCandidate(CANDIDATE_205).to_event()],
                ..AssertState::default()
            },
        );
    }

    #[test]
    fn unexpected_relocate_response() {
        run_test(
            "Get RPC ExpectCandidate",
            &initial_state_old_elders(),
            &[
                Rpc::RelocateResponse(get_relocated_info(CANDIDATE_205, DST_SECTION_INFO_200))
                    .to_event(),
            ],
            &AssertState {
                action_our_events: vec![ParsecVote::RelocateResponse(get_relocated_info(
                    CANDIDATE_205,
                    DST_SECTION_INFO_200,
                ))
                .to_event()],
                ..AssertState::default()
            },
        );
    }
}

mod node_tests {
    use super::*;
    use crate::state::JoiningRelocateCandidateState;
    use pretty_assertions::assert_eq;

    #[derive(Debug, PartialEq, Default, Clone)]
    struct AssertJoiningState {
        action_our_events: Vec<Event>,
        action_our_section: SectionInfo,
        join_routine: JoiningRelocateCandidateState,
    }

    fn run_joining_test(
        test_name: &str,
        start_state: &JoiningState,
        events: &[Event],
        expected_state: &AssertJoiningState,
    ) {
        let final_state = process_joining_events(start_state.clone(), &events);
        let action = final_state.action.inner();

        let final_state = (
            AssertJoiningState {
                action_our_events: action.our_events,
                action_our_section: action.our_section,
                join_routine: final_state.join_routine,
            },
            final_state.failure,
        );
        let expected_state = (expected_state.clone(), None);

        assert_eq!(expected_state, final_state, "{}", test_name);
    }

    fn process_joining_events(mut state: JoiningState, events: &[Event]) -> JoiningState {
        for event in events.iter().cloned() {
            state = match state.try_next(event) {
                Some(next_state) => next_state,
                None => state.failure_event(event),
            };

            if state.failure.is_some() {
                break;
            }
        }

        state
    }

    fn arrange_initial_joining_state(state: &JoiningState, events: &[Event]) -> JoiningState {
        let state = process_joining_events(state.clone(), events);
        state.action.remove_processed_state();
        state
    }

    fn initial_joining_state_with_dst_200() -> JoiningState {
        JoiningState {
            action: Action::new(INNER_ACTION_WITH_DST_SECTION_200.clone()),
            ..Default::default()
        }
    }

    //////////////////
    /// Joining Relocate Node
    //////////////////

    #[test]
    fn joining_start() {
        run_joining_test(
            "",
            &initial_joining_state_with_dst_200().start(DST_SECTION_INFO_200),
            &[],
            &AssertJoiningState {
                action_our_events: vec![
                    Rpc::ConnectionInfoRequest {
                        source: OUR_NAME,
                        destination: NAME_109,
                        connection_info: OUR_NAME.0,
                    }
                    .to_event(),
                    Rpc::ConnectionInfoRequest {
                        source: OUR_NAME,
                        destination: NAME_110,
                        connection_info: OUR_NAME.0,
                    }
                    .to_event(),
                    Rpc::ConnectionInfoRequest {
                        source: OUR_NAME,
                        destination: NAME_111,
                        connection_info: OUR_NAME.0,
                    }
                    .to_event(),
                    LocalEvent::JoiningTimeoutResendCandidateInfo.to_event(),
                    LocalEvent::JoiningTimeoutRefused.to_event(),
                ],
                join_routine: JoiningRelocateCandidateState {
                    has_resource_proofs: to_collect![
                        (NAME_109, (false, None)),
                        (NAME_110, (false, None)),
                        (NAME_111, (false, None))
                    ],
                    ..JoiningRelocateCandidateState::default()
                },
                ..AssertJoiningState::default()
            },
        );
    }

    #[test]
    fn joining_receive_two_connection_info() {
        let initial_state = arrange_initial_joining_state(
            &initial_joining_state_with_dst_200().start(DST_SECTION_INFO_200),
            &[],
        );

        run_joining_test(
            "",
            &initial_state,
            &[
                Rpc::ConnectionInfoResponse {
                    source: NAME_110,
                    destination: OUR_NAME,
                    connection_info: NAME_110.0,
                }
                .to_event(),
                Rpc::ConnectionInfoResponse {
                    source: NAME_111,
                    destination: OUR_NAME,
                    connection_info: NAME_111.0,
                }
                .to_event(),
            ],
            &AssertJoiningState {
                action_our_events: vec![
                    Rpc::CandidateInfo(CandidateInfo {
                        old_public_id: OUR_NODE_CANDIDATE,
                        new_public_id: OUR_NODE_CANDIDATE,
                        destination: NAME_110,
                        valid: true,
                    })
                    .to_event(),
                    Rpc::CandidateInfo(CandidateInfo {
                        old_public_id: OUR_NODE_CANDIDATE,
                        new_public_id: OUR_NODE_CANDIDATE,
                        destination: NAME_111,
                        valid: true,
                    })
                    .to_event(),
                ],
                join_routine: JoiningRelocateCandidateState {
                    has_resource_proofs: to_collect![
                        (NAME_109, (false, None)),
                        (NAME_110, (false, None)),
                        (NAME_111, (false, None))
                    ],
                    ..JoiningRelocateCandidateState::default()
                },
                ..AssertJoiningState::default()
            },
        );
    }

    #[test]
    fn joining_receive_one_resource_proof() {
        let initial_state = arrange_initial_joining_state(
            &initial_joining_state_with_dst_200().start(DST_SECTION_INFO_200),
            &[
                Rpc::ConnectionInfoResponse {
                    source: NAME_110,
                    destination: OUR_NAME,
                    connection_info: NAME_110.0,
                }
                .to_event(),
                Rpc::ConnectionInfoResponse {
                    source: NAME_111,
                    destination: OUR_NAME,
                    connection_info: NAME_111.0,
                }
                .to_event(),
            ],
        );

        run_joining_test(
            "",
            &initial_state,
            &[Rpc::ResourceProof {
                candidate: OUR_NODE_CANDIDATE,
                source: NAME_111,
                proof: ProofRequest { value: NAME_111.0 },
            }
            .to_event()],
            &AssertJoiningState {
                action_our_events: vec![LocalEvent::ComputeResourceProofForElder(
                    NAME_111,
                    ProofSource(2),
                )
                .to_event()],
                join_routine: JoiningRelocateCandidateState {
                    has_resource_proofs: to_collect![
                        (NAME_109, (false, None)),
                        (NAME_110, (false, None)),
                        (NAME_111, (true, None))
                    ],
                    ..JoiningRelocateCandidateState::default()
                },
                ..AssertJoiningState::default()
            },
        );
    }

    #[test]
    fn joining_computed_one_proof_one_proof() {
        let initial_state = arrange_initial_joining_state(
            &initial_joining_state_with_dst_200().start(DST_SECTION_INFO_200),
            &[
                Rpc::ConnectionInfoResponse {
                    source: NAME_111,
                    destination: OUR_NAME,
                    connection_info: NAME_111.0,
                }
                .to_event(),
                Rpc::ResourceProof {
                    candidate: OUR_NODE_CANDIDATE,
                    source: NAME_111,
                    proof: ProofRequest { value: NAME_111.0 },
                }
                .to_event(),
            ],
        );

        run_joining_test(
            "",
            &initial_state,
            &[LocalEvent::ComputeResourceProofForElder(NAME_111, ProofSource(2)).to_event()],
            &AssertJoiningState {
                action_our_events: vec![Rpc::ResourceProofResponse {
                    candidate: OUR_NODE_CANDIDATE,
                    destination: NAME_111,
                    proof: Proof::ValidPart,
                }
                .to_event()],
                join_routine: JoiningRelocateCandidateState {
                    has_resource_proofs: to_collect![
                        (NAME_109, (false, None)),
                        (NAME_110, (false, None)),
                        (NAME_111, (true, Some(ProofSource(1))))
                    ],
                    ..JoiningRelocateCandidateState::default()
                },
                ..AssertJoiningState::default()
            },
        );
    }

    #[test]
    fn joining_got_one_proof_receipt() {
        let initial_state = arrange_initial_joining_state(
            &initial_joining_state_with_dst_200().start(DST_SECTION_INFO_200),
            &[
                Rpc::ConnectionInfoResponse {
                    source: NAME_111,
                    destination: OUR_NAME,
                    connection_info: NAME_111.0,
                }
                .to_event(),
                Rpc::ResourceProof {
                    candidate: OUR_NODE_CANDIDATE,
                    source: NAME_111,
                    proof: ProofRequest { value: NAME_111.0 },
                }
                .to_event(),
                LocalEvent::ComputeResourceProofForElder(NAME_111, ProofSource(2)).to_event(),
            ],
        );

        run_joining_test(
            "",
            &initial_state,
            &[Rpc::ResourceProofReceipt {
                candidate: OUR_NODE_CANDIDATE,
                source: NAME_111,
            }
            .to_event()],
            &AssertJoiningState {
                action_our_events: vec![Rpc::ResourceProofResponse {
                    candidate: OUR_NODE_CANDIDATE,
                    destination: NAME_111,
                    proof: Proof::ValidEnd,
                }
                .to_event()],
                join_routine: JoiningRelocateCandidateState {
                    has_resource_proofs: to_collect![
                        (NAME_109, (false, None)),
                        (NAME_110, (false, None)),
                        (NAME_111, (true, Some(ProofSource(0))))
                    ],
                    ..JoiningRelocateCandidateState::default()
                },
                ..AssertJoiningState::default()
            },
        );
    }

    #[test]
    fn joining_resend_timeout_after_one_proof() {
        let initial_state = arrange_initial_joining_state(
            &initial_joining_state_with_dst_200().start(DST_SECTION_INFO_200),
            &[
                Rpc::ConnectionInfoResponse {
                    source: NAME_110,
                    destination: OUR_NAME,
                    connection_info: NAME_110.0,
                }
                .to_event(),
                Rpc::ConnectionInfoResponse {
                    source: NAME_111,
                    destination: OUR_NAME,
                    connection_info: NAME_111.0,
                }
                .to_event(),
                Rpc::ResourceProof {
                    candidate: OUR_NODE_CANDIDATE,
                    source: NAME_111,
                    proof: ProofRequest { value: NAME_111.0 },
                }
                .to_event(),
            ],
        );

        run_joining_test(
            "",
            &initial_state,
            &[LocalEvent::JoiningTimeoutResendCandidateInfo.to_event()],
            &AssertJoiningState {
                action_our_events: vec![
                    Rpc::ConnectionInfoRequest {
                        source: OUR_NAME,
                        destination: NAME_109,
                        connection_info: OUR_NAME.0,
                    }
                    .to_event(),
                    Rpc::ConnectionInfoRequest {
                        source: OUR_NAME,
                        destination: NAME_110,
                        connection_info: OUR_NAME.0,
                    }
                    .to_event(),
                    LocalEvent::JoiningTimeoutResendCandidateInfo.to_event(),
                ],
                join_routine: JoiningRelocateCandidateState {
                    has_resource_proofs: to_collect![
                        (NAME_109, (false, None)),
                        (NAME_110, (false, None)),
                        (NAME_111, (true, None))
                    ],
                    ..JoiningRelocateCandidateState::default()
                },
                ..AssertJoiningState::default()
            },
        );
    }

    #[test]
    fn joining_approved() {
        let initial_state = arrange_initial_joining_state(
            &initial_joining_state_with_dst_200().start(DST_SECTION_INFO_200),
            &[],
        );

        run_joining_test(
            "",
            &initial_state,
            &[
                Rpc::NodeApproval(OUR_NODE_CANDIDATE, GenesisPfxInfo(DST_SECTION_INFO_200))
                    .to_event(),
            ],
            &AssertJoiningState {
                join_routine: JoiningRelocateCandidateState {
                    routine_complete: Some(GenesisPfxInfo(DST_SECTION_INFO_200)),
                    ..JoiningRelocateCandidateState::default()
                },
                ..AssertJoiningState::default()
            },
        );
    }
}
