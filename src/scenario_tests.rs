// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use crate::{
    actions::{Action, InnerAction},
    state::{JoiningState, MemberState},
    utilities::{
        ActionTriggered, Age, Attributes, Candidate, CandidateInfo, ChurnNeeded, Event,
        GenesisPfxInfo, LocalEvent, Name, Node, NodeChange, NodeState, ParsecVote, Proof,
        ProofRequest, ProofSource, RelocatedInfo, Rpc, Section, SectionInfo, State, TestEvent,
        TryResult,
    },
};
use lazy_static::lazy_static;
use pretty_assertions::assert_eq;

const ATTRIBUTES_1_OLD: Attributes = Attributes {
    name: Name(1001),
    age: Age(9),
};
const ATTRIBUTES_1: Attributes = Attributes {
    name: Name(1),
    age: Age(10),
};

const ATTRIBUTES_2_OLD: Attributes = Attributes {
    name: Name(1002),
    age: Age(9),
};
const ATTRIBUTES_2: Attributes = Attributes {
    name: Name(2),
    age: Age(10),
};

const ATTRIBUTES_132_OLD: Attributes = Attributes {
    name: Name(132),
    age: Age(31),
};
const ATTRIBUTES_132: Attributes = Attributes {
    name: Name(132),
    age: Age(32),
};

const CANDIDATE_1_OLD: Candidate = Candidate(ATTRIBUTES_1_OLD);
const CANDIDATE_1: Candidate = Candidate(ATTRIBUTES_1);
const CANDIDATE_2_OLD: Candidate = Candidate(ATTRIBUTES_2_OLD);
const CANDIDATE_2: Candidate = Candidate(ATTRIBUTES_2);
const CANDIDATE_130: Candidate = Candidate(Attributes {
    name: Name(130),
    age: Age(30),
});
const CANDIDATE_205: Candidate = Candidate(Attributes {
    name: Name(205),
    age: Age(5),
});
const OTHER_SECTION_1: Section = Section(1);
const OTHER_SECTION_2: Section = Section(2);
const DST_SECTION_200: Section = Section(200);
const MERGED_SECTION_2: Section = Section(2);

const NODE_1_OLD: Node = Node(ATTRIBUTES_1_OLD);
const NODE_1: Node = Node(ATTRIBUTES_1);
const NODE_2_OLD: Node = Node(ATTRIBUTES_2_OLD);
const NODE_2: Node = Node(ATTRIBUTES_2);
const SET_ONLINE_NODE_1: NodeChange =
    NodeChange::ReplaceWith(TARGET_INTERVAL_1, NODE_1, State::Online);

const NODE_ELDER_109: Node = Node(Attributes {
    name: Name(109),
    age: Age(9),
});
const NODE_ELDER_110: Node = Node(Attributes {
    name: Name(110),
    age: Age(10),
});
const NODE_ELDER_111: Node = Node(Attributes {
    name: Name(111),
    age: Age(11),
});
const NODE_ELDER_130: Node = Node(Attributes {
    name: Name(130),
    age: Age(30),
});
const NODE_ELDER_131: Node = Node(Attributes {
    name: Name(131),
    age: Age(31),
});
const NODE_ELDER_132: Node = Node(ATTRIBUTES_132);

const NAME_110: Name = NODE_ELDER_110.0.name;
const NAME_111: Name = NODE_ELDER_111.0.name;

const YOUNG_ADULT_205: Node = Node(Attributes {
    name: Name(205),
    age: Age(5),
});
const SECTION_INFO_1: SectionInfo = SectionInfo(OUR_SECTION, 1);
const SECTION_INFO_2: SectionInfo = SectionInfo(OUR_SECTION, 2);
const DST_SECTION_INFO_200: SectionInfo = SectionInfo(DST_SECTION_200, 0);

const OTHER_SECTION_INFO: SectionInfo = SectionInfo(OTHER_SECTION_1, 0);
const REMOTE_OTHER_SECTION_INFO: SectionInfo = SectionInfo(OTHER_SECTION_2, 0);
const MERGED_SECTION_INFO: SectionInfo = SectionInfo(MERGED_SECTION_2, 0);

const SPLIT_SECTION_INFO_1: SectionInfo = SectionInfo(Section(1), 0);
const SPLIT_SECTION_INFO_2: SectionInfo = SectionInfo(Section(2), 0);

const CANDIDATE_INFO_VALID_1: CandidateInfo = CandidateInfo {
    old_public_id: CANDIDATE_1_OLD,
    new_public_id: CANDIDATE_1,
    destination: TARGET_INTERVAL_1,
    valid: true,
};
const REMOVE_CANDIDATE_1: NodeChange = NodeChange::Remove(TARGET_INTERVAL_1);

const CANDIDATE_RELOCATED_INFO_1: RelocatedInfo = RelocatedInfo {
    candidate: CANDIDATE_1_OLD,
    expected_age: Age(CANDIDATE_1_OLD.0.age.0 + 1),
    target_interval_centre: TARGET_INTERVAL_1,
    section_info: OUR_INITIAL_SECTION_INFO,
};

const CANDIDATE_RELOCATED_INFO_132: RelocatedInfo = RelocatedInfo {
    candidate: OUR_NODE_CANDIDATE_OLD,
    expected_age: OUR_NODE.0.age,
    target_interval_centre: TARGET_INTERVAL_1,
    section_info: DST_SECTION_INFO_200,
};

const CANDIDATE_INFO_VALID_RPC_1: Rpc = Rpc::CandidateInfo(CANDIDATE_INFO_VALID_1);
const TARGET_INTERVAL_1: Name = Name(1234);
const TARGET_INTERVAL_2: Name = Name(1235);

const OUR_SECTION: Section = Section(0);
const OUR_NODE_OLD: Node = Node(ATTRIBUTES_132_OLD);
const OUR_NODE: Node = Node(ATTRIBUTES_132);
const OUR_NAME: Name = OUR_NODE.0.name;
const OUR_NODE_CANDIDATE: Candidate = Candidate(OUR_NODE.0);
const OUR_NODE_CANDIDATE_OLD: Candidate = Candidate(OUR_NODE_OLD.0);
const OUR_PROOF_REQUEST: ProofRequest = ProofRequest { value: OUR_NAME.0 };
const OUR_INITIAL_SECTION_INFO: SectionInfo = SectionInfo(OUR_SECTION, 0);
const OUR_GENESIS_INFO: GenesisPfxInfo = GenesisPfxInfo(OUR_INITIAL_SECTION_INFO);

lazy_static! {
    static ref INNER_ACTION_132: InnerAction = InnerAction::new_with_our_attributes(OUR_NODE.0)
        .with_next_target_interval(TARGET_INTERVAL_1);
    static ref INNER_ACTION_YOUNG_ELDERS: InnerAction = INNER_ACTION_132
        .clone()
        .extend_current_nodes_with(
            &NodeState::default_elder(),
            &[NODE_ELDER_109, NODE_ELDER_110, NODE_ELDER_132]
        )
        .extend_current_nodes_with(&NodeState::default(), &[YOUNG_ADULT_205]);
    static ref INNER_ACTION_OLD_ELDERS: InnerAction = INNER_ACTION_132
        .clone()
        .extend_current_nodes_with(
            &NodeState::default_elder(),
            &[NODE_ELDER_130, NODE_ELDER_131, NODE_ELDER_132]
        )
        .extend_current_nodes_with(&NodeState::default(), &[YOUNG_ADULT_205]);
    static ref INNER_ACTION_YOUNG_ELDERS_WITH_WAITING_ELDER: InnerAction = INNER_ACTION_132
        .clone()
        .extend_current_nodes_with(
            &NodeState::default_elder(),
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
}

fn process_events(mut state: MemberState, events: &[Event]) -> MemberState {
    for event in events.iter().cloned() {
        if TryResult::Unhandled == state.try_next(event) {
            state.failure_event(event);
        }

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
        expected_age: candidate.0.age.increment_by_one(),
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
            "",
            &initial_state_old_elders(),
            &[Rpc::ExpectCandidate(CANDIDATE_1_OLD).to_event()],
            &AssertState {
                action_our_events: vec![ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event()],
            },
        );
    }

    #[test]
    fn parsec_expect_candidate() {
        run_test(
            "",
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    NodeChange::AddWithState(
                        Node(Attributes {
                            name: TARGET_INTERVAL_1,
                            age: CANDIDATE_1.0.age,
                        }),
                        State::WaitingCandidateInfo(CANDIDATE_RELOCATED_INFO_1),
                    )
                    .to_event(),
                    Rpc::RelocateResponse(CANDIDATE_RELOCATED_INFO_1).to_event(),
                    ActionTriggered::Scheduled(LocalEvent::TimeoutAccept).to_event(),
                ],
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
            "Get ExpectCandidate again for same candidate reply with same Rpc::RelocateResponse",
            &initial_state,
            &[ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event()],
            &AssertState {
                action_our_events: vec![
                    Rpc::RelocateResponse(CANDIDATE_RELOCATED_INFO_1).to_event()
                ],
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_candidate_info11() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
        );

        run_test(
            "Start resource proofing candidate: Send RPC.",
            &initial_state,
            &[CANDIDATE_INFO_VALID_RPC_1.to_event()],
            &AssertState {
                action_our_events: vec![Rpc::ResourceProof {
                    candidate: CANDIDATE_1,
                    source: OUR_NAME,
                    proof: OUR_PROOF_REQUEST,
                }
                .to_event()],
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_candidate_info_twice() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
                CANDIDATE_INFO_VALID_RPC_1.to_event(),
            ],
        );

        run_test(
            "Start resource proofing candidate: Send same RPC again.",
            &initial_state,
            &[CANDIDATE_INFO_VALID_RPC_1.to_event()],
            &AssertState {
                action_our_events: vec![Rpc::ResourceProof {
                    candidate: CANDIDATE_1,
                    source: OUR_NAME,
                    proof: OUR_PROOF_REQUEST,
                }
                .to_event()],
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_candidate_info_then_expect_candidate_again() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
                CANDIDATE_INFO_VALID_RPC_1.to_event(),
            ],
        );

        run_test(
            "Continue accepting old ExpectCandidate until candidate resource proof complete",
            &initial_state,
            &[ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event()],
            &AssertState {
                action_our_events: vec![
                    Rpc::RelocateResponse(CANDIDATE_RELOCATED_INFO_1).to_event()
                ],
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_with_shorter_section_exists() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[TestEvent::SetShortestPrefix(Some(OTHER_SECTION_1)).to_event()],
        );

        let description =
            "Relocate candidate immediately when a section with shorter prefix exists.";
        run_test(
            description,
            &initial_state,
            &[ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event()],
            &AssertState {
                action_our_events: vec![Rpc::ExpectCandidate(CANDIDATE_1_OLD).to_event()],
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_invalid_candidate_info() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
        );

        run_test(
            "Discard invalid CandidateInfo",
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
                ParsecVote::CheckResourceProof.to_event(),
                CANDIDATE_INFO_VALID_RPC_1.to_event(),
            ],
        );

        run_test(
            "Timeout for resource proof: Vote to fail the candidate's resource proof",
            &initial_state,
            &[LocalEvent::TimeoutAccept.to_event()],
            &AssertState {
                action_our_events: vec![ParsecVote::PurgeCandidate(CANDIDATE_1_OLD).to_event()],
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
            "Discard CandidateInfo from candidate we are not or no longer expecting",
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
    fn parsec_expect_candidate_then_candidate_info_then_part_proof() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
                CANDIDATE_INFO_VALID_RPC_1.to_event(),
            ],
        );

        run_test(
            "Respond to proof from current candidate with receipt.",
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
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_candidate_info_then_end_proof() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
                CANDIDATE_INFO_VALID_RPC_1.to_event(),
            ],
        );

        run_test(
            "Vote candidate online when receiving the end of the proof and respond with receipt.",
            &initial_state,
            &[Rpc::ResourceProofResponse {
                candidate: CANDIDATE_1,
                destination: OUR_NAME,
                proof: Proof::ValidEnd,
            }
            .to_event()],
            &AssertState {
                action_our_events: vec![
                    ParsecVote::Online(CANDIDATE_1_OLD, CANDIDATE_1).to_event(),
                    Rpc::ResourceProofReceipt {
                        candidate: CANDIDATE_1,
                        source: OUR_NAME,
                    }
                    .to_event(),
                ],
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_candidate_info_then_end_proof_twice() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
                CANDIDATE_INFO_VALID_RPC_1.to_event(),
                Rpc::ResourceProofResponse {
                    candidate: CANDIDATE_1,
                    destination: OUR_NAME,
                    proof: Proof::ValidEnd,
                }
                .to_event(),
            ],
        );

        run_test(
            "Discard further ResourceProofResponse once voted online.",
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
                ParsecVote::CheckResourceProof.to_event(),
                CANDIDATE_INFO_VALID_RPC_1.to_event(),
            ],
        );

        run_test(
            "Discard invalid proofs.",
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
                ParsecVote::CheckResourceProof.to_event(),
                CANDIDATE_INFO_VALID_RPC_1.to_event(),
            ],
        );

        run_test(
            "Discard final proof from a candidate that is not the current one.",
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
                ParsecVote::CheckResourceProof.to_event(),
                CANDIDATE_INFO_VALID_RPC_1.to_event(),
            ],
        );

        let description =
            "Ignore parsec votes not for the current candidate. \
             The previous running resource proof for CANDIDATE_2 may have been cancelled, \
             or we would only see either of the votes later on.";
        run_test(
            description,
            &initial_state,
            &[
                ParsecVote::Online(CANDIDATE_2_OLD, CANDIDATE_2).to_event(),
                ParsecVote::PurgeCandidate(CANDIDATE_2_OLD).to_event(),
            ],
            &AssertState::default(),
        );
    }

    #[test]
    fn rpc_merge() {
        run_test(
            "",
            &initial_state_old_elders(),
            &[Rpc::Merge(OTHER_SECTION_INFO).to_event()],
            &AssertState {
                action_our_events: vec![ParsecVote::NeighbourMerge(OTHER_SECTION_INFO).to_event()],
            },
        );
    }

    #[test]
    fn parsec_neighbour_merge() {
        run_test(
            "When a neighbour Merge RPC is consensused, store its info to decide merging",
            &initial_state_old_elders(),
            &[ParsecVote::NeighbourMerge(OTHER_SECTION_INFO).to_event()],
            &AssertState {
                action_our_events: vec![
                    ActionTriggered::MergeInfoStored(OTHER_SECTION_INFO).to_event()
                ],
            },
        );
    }

    #[test]
    fn parsec_neighbour_merge_then_check_elder() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[ParsecVote::NeighbourMerge(OTHER_SECTION_INFO).to_event()],
        );

        run_test(
            "When we have neighbour info, we are ready to merge on next CheckElder",
            &initial_state,
            &[ParsecVote::CheckElder.to_event()],
            &AssertState {
                action_our_events: vec![
                    Rpc::Merge(SectionInfo(OUR_SECTION, 0)).to_event(),
                    ParsecVote::NewSectionInfo(MERGED_SECTION_INFO).to_event(),
                ],
            },
        );
    }

    #[test]
    fn parsec_merge_needed() {
        let initial_state = initial_state_old_elders();

        run_test(
            "Merge if we detect our section needs merging on CheckElder",
            &initial_state,
            &[
                TestEvent::SetChurnNeeded(ChurnNeeded::Merge).to_event(),
                ParsecVote::CheckElder.to_event(),
            ],
            &AssertState {
                action_our_events: vec![Rpc::Merge(SectionInfo(OUR_SECTION, 0)).to_event()],
            },
        );
    }

    #[test]
    fn parsec_merge_sibling() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                TestEvent::SetChurnNeeded(ChurnNeeded::Merge).to_event(),
                ParsecVote::CheckElder.to_event(),
            ],
        );
        run_test(
            "Decide to merge, and then later store merge infos in ProcessMerge",
            &initial_state,
            &[ParsecVote::NeighbourMerge(OTHER_SECTION_INFO).to_event()],
            &AssertState {
                action_our_events: vec![
                    ActionTriggered::MergeInfoStored(OTHER_SECTION_INFO).to_event(),
                    ParsecVote::NewSectionInfo(MERGED_SECTION_INFO).to_event(),
                ],
            },
        );
    }

    #[test]
    fn parsec_merge_non_sibling() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                TestEvent::SetChurnNeeded(ChurnNeeded::Merge).to_event(),
                ParsecVote::CheckElder.to_event(),
            ],
        );
        run_test(
            "Get consensus on merging with a non-sibling neighbour",
            &initial_state,
            &[ParsecVote::NeighbourMerge(REMOTE_OTHER_SECTION_INFO).to_event()],
            &AssertState {
                action_our_events: vec![ActionTriggered::MergeInfoStored(
                    REMOTE_OTHER_SECTION_INFO,
                )
                .to_event()],
            },
        );
    }

    #[test]
    fn parsec_new_section() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::NeighbourMerge(OTHER_SECTION_INFO).to_event(),
                ParsecVote::CheckElder.to_event(),
            ],
        );
        run_test(
            "Get consensus on new section after merge and finalise",
            &initial_state,
            &[ParsecVote::NewSectionInfo(MERGED_SECTION_INFO).to_event()],
            &AssertState {
                action_our_events: vec![
                    ActionTriggered::CompleteMerge.to_event(),
                    ActionTriggered::Scheduled(LocalEvent::TimeoutCheckElder).to_event(),
                ],
            },
        );
    }

    #[test]
    fn parsec_split_needed() {
        run_test(
            "Split if we detect our section needs splitting on CheckElder",
            &initial_state_old_elders(),
            &[
                TestEvent::SetChurnNeeded(ChurnNeeded::Split).to_event(),
                ParsecVote::CheckElder.to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    ParsecVote::NewSectionInfo(SPLIT_SECTION_INFO_1).to_event(),
                    ParsecVote::NewSectionInfo(SPLIT_SECTION_INFO_2).to_event(),
                ],
            },
        );
    }

    #[test]
    fn parsec_split_complete() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                TestEvent::SetChurnNeeded(ChurnNeeded::Split).to_event(),
                ParsecVote::CheckElder.to_event(),
            ],
        );
        run_test(
            "Get consensus on new sections after split and finalise",
            &initial_state,
            &[
                ParsecVote::NewSectionInfo(SPLIT_SECTION_INFO_1).to_event(),
                ParsecVote::NewSectionInfo(SPLIT_SECTION_INFO_2).to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    ActionTriggered::CompleteSplit.to_event(),
                    ActionTriggered::Scheduled(LocalEvent::TimeoutCheckElder).to_event(),
                ],
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_online_no_elder_change() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
        );

        let description =
            "Accept a new node (No Elder Change): send RPC and schedule next ResourceProof. \
             CheckElder has no work.";
        run_test(
            description,
            &initial_state,
            &[
                ParsecVote::Online(CANDIDATE_1_OLD, CANDIDATE_1).to_event(),
                ParsecVote::CheckElder.to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    SET_ONLINE_NODE_1.to_event(),
                    Rpc::NodeApproval(CANDIDATE_1, OUR_GENESIS_INFO).to_event(),
                    ActionTriggered::Scheduled(LocalEvent::CheckResourceProofTimeout).to_event(),
                    ActionTriggered::Scheduled(LocalEvent::TimeoutCheckElder).to_event(),
                ],
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_online_elder_change() {
        let initial_state = arrange_initial_state(
            &initial_state_young_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
        );

        let description =
            "Accept a new node (Elder Change): send RPC and schedule next ResourceProof. \
             CheckElder start updating the section when triggered.";
        run_test(
            description,
            &initial_state,
            &[
                ParsecVote::Online(CANDIDATE_1_OLD, CANDIDATE_1).to_event(),
                ParsecVote::CheckElder.to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    SET_ONLINE_NODE_1.to_event(),
                    Rpc::NodeApproval(CANDIDATE_1, OUR_GENESIS_INFO).to_event(),
                    ActionTriggered::Scheduled(LocalEvent::CheckResourceProofTimeout).to_event(),
                    ParsecVote::AddElderNode(NODE_1).to_event(),
                    ParsecVote::RemoveElderNode(NODE_ELDER_109).to_event(),
                    ParsecVote::NewSectionInfo(SECTION_INFO_1).to_event(),
                ],
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_online_elder_change_get_wrong_votes() {
        let initial_state = arrange_initial_state(
            &initial_state_young_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
                ParsecVote::Online(CANDIDATE_1_OLD, CANDIDATE_1).to_event(),
                ParsecVote::CheckElder.to_event(),
            ],
        );

        let description = "Accept a new node (Elder Change) - Check Elder Triggered. \
                           Error if consensus unexpected votes.";
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
                    ActionTriggered::UnexpectedEventErrorTriggered.to_event(),
                    ActionTriggered::UnexpectedEventErrorTriggered.to_event(),
                    ActionTriggered::UnexpectedEventErrorTriggered.to_event(),
                ],
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_online_elder_change_remove_elder() {
        let initial_state = arrange_initial_state(
            &initial_state_young_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
                ParsecVote::Online(CANDIDATE_1_OLD, CANDIDATE_1).to_event(),
                ParsecVote::CheckElder.to_event(),
            ],
        );

        let description = "Accept a new node (Elder Change) - Check Elder Triggered. \
                           Nothing change until all expected votes have happened.";
        run_test(
            description,
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
                ParsecVote::CheckResourceProof.to_event(),
                ParsecVote::Online(CANDIDATE_1_OLD, CANDIDATE_1).to_event(),
                ParsecVote::CheckElder.to_event(),
                ParsecVote::RemoveElderNode(NODE_ELDER_109).to_event(),
            ],
        );

        let description =
            "Accept a new node (Elder Change) - Check Elder Triggered and votes completed. \
             Once completed, update our section and elders";
        run_test(
            description,
            &initial_state,
            &[
                ParsecVote::AddElderNode(NODE_1).to_event(),
                ParsecVote::NewSectionInfo(SECTION_INFO_1).to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    NodeChange::Elder(NODE_1, true).to_event(),
                    NodeChange::Elder(NODE_ELDER_109, false).to_event(),
                    ActionTriggered::OurSectionChanged(SECTION_INFO_1).to_event(),
                    ActionTriggered::Scheduled(LocalEvent::TimeoutCheckElder).to_event(),
                ],
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_when_candidate_completed_with_elder_change_in_progress() {
        let initial_state = arrange_initial_state(
            &initial_state_young_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
                ParsecVote::Online(CANDIDATE_1_OLD, CANDIDATE_1).to_event(),
                ParsecVote::CheckElder.to_event(),
            ],
        );

        let description =
            "Accept new candidate even if elder where changed by first candidate joining. \
             Use old section as new one not yet consensused.";
        run_test(
            description,
            &initial_state,
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_2_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
            &&AssertState {
                action_our_events: vec![
                    NodeChange::AddWithState(
                        Node(Attributes {
                            name: TARGET_INTERVAL_2,
                            age: CANDIDATE_2.0.age,
                        }),
                        State::WaitingCandidateInfo(RelocatedInfo {
                            candidate: CANDIDATE_2_OLD,
                            expected_age: CANDIDATE_2.0.age,
                            target_interval_centre: TARGET_INTERVAL_2,
                            section_info: OUR_INITIAL_SECTION_INFO,
                        }),
                    )
                    .to_event(),
                    Rpc::RelocateResponse(RelocatedInfo {
                        candidate: CANDIDATE_2_OLD,
                        expected_age: CANDIDATE_2.0.age,
                        target_interval_centre: TARGET_INTERVAL_2,
                        section_info: OUR_INITIAL_SECTION_INFO,
                    })
                    .to_event(),
                    ActionTriggered::Scheduled(LocalEvent::TimeoutAccept).to_event(),
                ],
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_purge() {
        let initial_state = arrange_initial_state(
            &initial_state_young_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
            ],
        );

        run_test(
            "Complete proof failing the candidate: Remove and schedule next candidate.",
            &initial_state,
            &[ParsecVote::PurgeCandidate(CANDIDATE_1_OLD).to_event()],
            &AssertState {
                action_our_events: vec![
                    REMOVE_CANDIDATE_1.to_event(),
                    ActionTriggered::Scheduled(LocalEvent::CheckResourceProofTimeout).to_event(),
                ],
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_then_candidate_info_then_purge() {
        let initial_state = arrange_initial_state(
            &initial_state_young_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
                CANDIDATE_INFO_VALID_RPC_1.to_event(),
            ],
        );

        run_test(
            "Complete proof failing the candidate: Remove and schedule next candidate.",
            &initial_state,
            &[ParsecVote::PurgeCandidate(CANDIDATE_1_OLD).to_event()],
            &AssertState {
                action_our_events: vec![
                    REMOVE_CANDIDATE_1.to_event(),
                    ActionTriggered::Scheduled(LocalEvent::CheckResourceProofTimeout).to_event(),
                ],
            },
        );
    }

    #[test]
    fn parsec_expect_candidate_twice() {
        let initial_state = arrange_initial_state(
            &initial_state_young_elders(),
            &[
                ParsecVote::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                ParsecVote::CheckResourceProof.to_event(),
                CANDIDATE_INFO_VALID_RPC_1.to_event(),
            ],
        );

        run_test(
            "Refuse new candidate if first not completed",
            &initial_state,
            &[ParsecVote::ExpectCandidate(CANDIDATE_2_OLD).to_event()],
            &AssertState {
                action_our_events: vec![Rpc::RefuseCandidate(CANDIDATE_2_OLD).to_event()],
            },
        );
    }

    #[test]
    fn parsec_unexpected_purge_online() {
        let description = "Get unexpected Parsec consensus Online and PurgeCandidate. \
                           Candidate may have triggered both votes: only consider the first";
        run_test(
            description,
            &initial_state_old_elders(),
            &[
                ParsecVote::Online(CANDIDATE_1_OLD, CANDIDATE_1).to_event(),
                ParsecVote::PurgeCandidate(CANDIDATE_1_OLD).to_event(),
            ],
            &AssertState::default(),
        );
    }

    #[test]
    fn rpc_unexpected_candidate_info_resource_proof_response() {
        let description = "Get unexpected RPC CandidateInfo and ResourceProofResponse. \
                           Candidate RPC may arrive after candidate was purged or accepted";
        run_test(
            description,
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
            },
        );
    }

    #[test]
    fn parsec_offline() {
        run_test(
            "Change node state when consensus offline.",
            &initial_state_old_elders(),
            &[ParsecVote::Offline(NODE_ELDER_130).to_event()],
            &AssertState {
                action_our_events: vec![
                    NodeChange::State(NODE_ELDER_130, State::Offline).to_event()
                ],
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
            "On CheckElder, initiate removing offline elder from the SectionInfo.",
            &initial_state,
            &[ParsecVote::CheckElder.to_event()],
            &AssertState {
                action_our_events: vec![
                    ParsecVote::AddElderNode(YOUNG_ADULT_205).to_event(),
                    ParsecVote::RemoveElderNode(NODE_ELDER_130).to_event(),
                    ParsecVote::NewSectionInfo(SECTION_INFO_1).to_event(),
                ],
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
            "Relocate nodes coming back online.",
            &initial_state,
            &[ParsecVote::BackOnline(NODE_ELDER_130).to_event()],
            &AssertState {
                action_our_events: vec![NodeChange::State(
                    NODE_ELDER_130,
                    State::RelocatingBackOnline,
                )
                .to_event()],
            },
        );
    }
}

//////////////////
/// Src
//////////////////

mod src_tests {
    use super::*;
    use itertools::Itertools;

    #[test]
    fn local_event_time_out_work_unit() {
        run_test(
            "Work unit timer and vote together to keep span consistent.",
            &initial_state_old_elders(),
            &[LocalEvent::TimeoutWorkUnit.to_event()],
            &AssertState {
                action_our_events: vec![
                    ParsecVote::WorkUnitIncrement.to_event(),
                    ActionTriggered::Scheduled(LocalEvent::TimeoutWorkUnit).to_event(),
                ],
            },
        );
    }

    #[test]
    fn start_relocation() {
        run_test(
            "When work unit is sufficient, CheckRelocate initiate relocating non elder node.",
            &initial_state_old_elders(),
            &[
                TestEvent::SetWorkUnitEnoughToRelocate(YOUNG_ADULT_205).to_event(),
                ParsecVote::WorkUnitIncrement.to_event(),
                ParsecVote::CheckRelocate.to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    ActionTriggered::WorkUnitIncremented.to_event(),
                    NodeChange::State(YOUNG_ADULT_205, State::RelocatingAgeIncrease).to_event(),
                    Rpc::ExpectCandidate(CANDIDATE_205).to_event(),
                ],
            },
        );
    }

    #[test]
    fn parsec_check_work_unit_increment_has_no_effect_if_relocating_node() {
        let initial_state = arrange_initial_state(
            &initial_state_old_elders(),
            &[
                TestEvent::SetWorkUnitEnoughToRelocate(YOUNG_ADULT_205).to_event(),
                ParsecVote::WorkUnitIncrement.to_event(),
                TestEvent::SetWorkUnitEnoughToRelocate(NODE_ELDER_130).to_event(),
            ],
        );

        run_test(
            "Additional WorkUnitIncrement does not trigger a new relocate if one started",
            &initial_state,
            &[ParsecVote::WorkUnitIncrement.to_event()],
            &AssertState {
                action_our_events: vec![ActionTriggered::WorkUnitIncremented.to_event()],
            },
        );
    }

    #[test]
    fn parsec_check_get_node_to_relocate_uses_online_nodes_only() {
        let mut all_non_online_states = vec![
            State::RelocatingAgeIncrease,
            State::RelocatingHop,
            State::RelocatingBackOnline,
            State::Relocated(get_relocated_info(CANDIDATE_1_OLD, SECTION_INFO_1)),
            State::WaitingCandidateInfo(get_relocated_info(CANDIDATE_2_OLD, SECTION_INFO_2)),
            State::WaitingProofing,
            State::Offline,
        ];

        let age = CANDIDATE_1_OLD.0.age;
        let states_with_enough_work_units = all_non_online_states
            .drain(..)
            .enumerate()
            .map(|(name_increment, state)| {
                let node = Node(Attributes {
                    name: Name(1000 + name_increment as i32),
                    age,
                });
                NodeState {
                    node,
                    is_elder: false,
                    work_units_done: age.0,
                    state,
                }
            })
            .collect_vec();
        let inner_action = INNER_ACTION_YOUNG_ELDERS
            .clone()
            .extend_current_nodes(&states_with_enough_work_units);

        let initial_state = MemberState {
            action: Action::new(inner_action),
            ..Default::default()
        };

        run_test(
            "Check get_node_to_relocate doesn't choose anything but 'Online' nodes",
            &initial_state,
            &[ParsecVote::WorkUnitIncrement.to_event()],
            &AssertState {
                action_our_events: vec![ActionTriggered::WorkUnitIncremented.to_event()],
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
                    ActionTriggered::WorkUnitIncremented.to_event(),
                    NodeChange::State(YOUNG_ADULT_205, State::RelocatingAgeIncrease).to_event(),
                    Rpc::ExpectCandidate(CANDIDATE_205).to_event(),
                    Rpc::ExpectCandidate(CANDIDATE_1_OLD).to_event(),
                    Rpc::ExpectCandidate(CANDIDATE_2).to_event(),
                    Rpc::ExpectCandidate(CANDIDATE_205).to_event(),
                ],
            },
        );
    }

    #[test]
    fn parsec_relocate_trigger_elder_change() {
        run_test(
            "Work unit trigger relocation (Elder Change): Update Elders and relocate only after.",
            &initial_state_old_elders(),
            &[
                TestEvent::SetWorkUnitEnoughToRelocate(NODE_ELDER_130).to_event(),
                ParsecVote::WorkUnitIncrement.to_event(),
                ParsecVote::CheckRelocate.to_event(),
                ParsecVote::CheckElder.to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    ActionTriggered::WorkUnitIncremented.to_event(),
                    NodeChange::State(NODE_ELDER_130, State::RelocatingAgeIncrease).to_event(),
                    ParsecVote::AddElderNode(YOUNG_ADULT_205).to_event(),
                    ParsecVote::RemoveElderNode(NODE_ELDER_130).to_event(),
                    ParsecVote::NewSectionInfo(SECTION_INFO_1).to_event(),
                ],
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
            "Work unit trigger relocation (Elder Change): Update Elders and relocate only after.",
            &initial_state,
            &[
                ParsecVote::RemoveElderNode(NODE_ELDER_130).to_event(),
                ParsecVote::AddElderNode(YOUNG_ADULT_205).to_event(),
                ParsecVote::NewSectionInfo(SECTION_INFO_1).to_event(),
                ParsecVote::CheckRelocate.to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    NodeChange::Elder(YOUNG_ADULT_205, true).to_event(),
                    NodeChange::Elder(NODE_ELDER_130, false).to_event(),
                    ActionTriggered::OurSectionChanged(SECTION_INFO_1).to_event(),
                    ActionTriggered::Scheduled(LocalEvent::TimeoutCheckElder).to_event(),
                    Rpc::ExpectCandidate(CANDIDATE_130).to_event(),
                ],
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
            "Vote for RPC to be processed",
            &initial_state,
            &[Rpc::RefuseCandidate(CANDIDATE_205).to_event()],
            &AssertState {
                action_our_events: vec![ParsecVote::RefuseCandidate(CANDIDATE_205).to_event()],
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
            "Vote for RPC to be processed",
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

        let description = "When RelocateResponse, update node state and vote for RelocatedInfo. \
                           When RelocatedInfo consensused send RPC and remove node,";
        run_test(
            description,
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
            "On refuse, only get ready to resend ExpectCandidate on next check.",
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
            "On next check after refuse, re-send ExpectCandidate.",
            &initial_state,
            &[ParsecVote::CheckRelocate.to_event()],
            &AssertState {
                action_our_events: vec![Rpc::ExpectCandidate(CANDIDATE_205).to_event()],
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
            "On next check after refuse, re-send ExpectCandidate for a node that was elder.",
            &initial_state,
            &[ParsecVote::CheckRelocate.to_event()],
            &AssertState {
                action_our_events: vec![Rpc::ExpectCandidate(CANDIDATE_130).to_event()],
            },
        );
    }

    #[test]
    fn unexpected_refuse_or_accept_candidate() {
        run_test(
            "Vote for unexpected responses to ExpectCandidate as we may be lagging.",
            &initial_state_old_elders(),
            &[
                Rpc::RefuseCandidate(CANDIDATE_205).to_event(),
                Rpc::RelocateResponse(get_relocated_info(CANDIDATE_205, DST_SECTION_INFO_200))
                    .to_event(),
            ],
            &AssertState {
                action_our_events: vec![
                    ParsecVote::RefuseCandidate(CANDIDATE_205).to_event(),
                    ParsecVote::RelocateResponse(get_relocated_info(
                        CANDIDATE_205,
                        DST_SECTION_INFO_200,
                    ))
                    .to_event(),
                ],
            },
        );
    }
}

mod node_tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[derive(Debug, PartialEq, Default, Clone)]
    struct AssertJoiningState {
        action_our_events: Vec<Event>,
        routine_complete_output: Option<GenesisPfxInfo>,
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
                routine_complete_output: final_state.join_routine.routine_complete_output,
            },
            final_state.failure,
        );
        let expected_state = (expected_state.clone(), None);

        assert_eq!(expected_state, final_state, "{}", test_name);
    }

    fn process_joining_events(mut state: JoiningState, events: &[Event]) -> JoiningState {
        for event in events.iter().cloned() {
            if TryResult::Unhandled == state.try_next(event) {
                state.failure_event(event);
            }

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
        let mut initial_state = initial_joining_state_with_dst_200();
        initial_state.start(CANDIDATE_RELOCATED_INFO_132);

        run_joining_test(
            "",
            &initial_state,
            &[],
            &AssertJoiningState {
                action_our_events: vec![
                    Rpc::CandidateInfo(CandidateInfo {
                        old_public_id: OUR_NODE_CANDIDATE_OLD,
                        new_public_id: OUR_NODE_CANDIDATE,
                        destination: TARGET_INTERVAL_1,
                        valid: true,
                    })
                    .to_event(),
                    ActionTriggered::Scheduled(LocalEvent::JoiningTimeoutResendInfo).to_event(),
                    ActionTriggered::Scheduled(LocalEvent::JoiningTimeoutConnectRefused).to_event(),
                ],
                routine_complete_output: None,
            },
        );
    }

    #[test]
    fn joining_resend_timeout() {
        let mut initial_state = initial_joining_state_with_dst_200();
        initial_state.start(CANDIDATE_RELOCATED_INFO_132);

        let initial_state = arrange_initial_joining_state(&initial_state, &[]);

        run_joining_test(
            "When not yet connected, resend CandidateInfo.",
            &initial_state,
            &[LocalEvent::JoiningTimeoutResendInfo.to_event()],
            &AssertJoiningState {
                action_our_events: vec![
                    Rpc::CandidateInfo(CandidateInfo {
                        old_public_id: OUR_NODE_CANDIDATE_OLD,
                        new_public_id: OUR_NODE_CANDIDATE,
                        destination: TARGET_INTERVAL_1,
                        valid: true,
                    })
                    .to_event(),
                    ActionTriggered::Scheduled(LocalEvent::JoiningTimeoutResendInfo).to_event(),
                ],
                routine_complete_output: None,
            },
        );
    }

    #[test]
    fn joining_receive_two_connection_info() {
        let mut initial_state = initial_joining_state_with_dst_200();
        initial_state.start(CANDIDATE_RELOCATED_INFO_132);

        let initial_state = arrange_initial_joining_state(&initial_state, &[]);

        run_joining_test(
            "",
            &initial_state,
            &[
                Rpc::ConnectionInfoRequest {
                    source: NAME_110,
                    destination: OUR_NAME,
                    connection_info: NAME_110.0,
                }
                .to_event(),
                Rpc::ConnectionInfoRequest {
                    source: NAME_111,
                    destination: OUR_NAME,
                    connection_info: NAME_111.0,
                }
                .to_event(),
            ],
            &AssertJoiningState {
                action_our_events: vec![
                    Rpc::ConnectionInfoResponse {
                        source: OUR_NAME,
                        destination: NAME_110,
                        connection_info: OUR_NAME.0,
                    }
                    .to_event(),
                    Rpc::ConnectionInfoResponse {
                        source: OUR_NAME,
                        destination: NAME_111,
                        connection_info: OUR_NAME.0,
                    }
                    .to_event(),
                ],
                routine_complete_output: None,
            },
        );
    }

    #[test]
    fn joining_receive_node_connected() {
        let mut initial_state = initial_joining_state_with_dst_200();
        initial_state.start(CANDIDATE_RELOCATED_INFO_132);

        let initial_state = arrange_initial_joining_state(&initial_state, &[]);

        run_joining_test(
            "",
            &initial_state,
            &[
                Rpc::NodeConnected(OUR_NODE_CANDIDATE, GenesisPfxInfo(DST_SECTION_INFO_200))
                    .to_event(),
            ],
            &AssertJoiningState {
                action_our_events: vec![ActionTriggered::Killed(
                    LocalEvent::JoiningTimeoutConnectRefused,
                )
                .to_event()],
                routine_complete_output: None,
            },
        );
    }

    #[test]
    fn joining_receive_two_resource_proof() {
        let mut initial_state = initial_joining_state_with_dst_200();
        initial_state.start(CANDIDATE_RELOCATED_INFO_132);

        let initial_state = arrange_initial_joining_state(
            &initial_state,
            &[
                Rpc::NodeConnected(OUR_NODE_CANDIDATE, GenesisPfxInfo(DST_SECTION_INFO_200))
                    .to_event(),
            ],
        );

        run_joining_test(
            "Start computing resource proof when receiving ResourceProof RPC and setup timers.",
            &initial_state,
            &[
                Rpc::ResourceProof {
                    candidate: OUR_NODE_CANDIDATE,
                    source: NAME_111,
                    proof: ProofRequest { value: NAME_111.0 },
                }
                .to_event(),
                Rpc::ResourceProof {
                    candidate: OUR_NODE_CANDIDATE,
                    source: NAME_110,
                    proof: ProofRequest { value: NAME_111.0 },
                }
                .to_event(),
            ],
            &AssertJoiningState {
                action_our_events: vec![
                    ActionTriggered::Scheduled(LocalEvent::JoiningTimeoutProofRefused).to_event(),
                    ActionTriggered::ComputeResourceProofForElder(NAME_111).to_event(),
                    ActionTriggered::Scheduled(LocalEvent::JoiningTimeoutProofRefused).to_event(),
                    ActionTriggered::ComputeResourceProofForElder(NAME_110).to_event(),
                ],
                routine_complete_output: None,
            },
        );
    }

    #[test]
    fn joining_computed_two_proofs() {
        let mut initial_state = initial_joining_state_with_dst_200();
        initial_state.start(CANDIDATE_RELOCATED_INFO_132);

        let initial_state = arrange_initial_joining_state(
            &initial_state,
            &[
                Rpc::NodeConnected(OUR_NODE_CANDIDATE, GenesisPfxInfo(DST_SECTION_INFO_200))
                    .to_event(),
            ],
        );

        run_joining_test(
            "When proof computed, start sending response to correct Elder.",
            &initial_state,
            &[
                TestEvent::SetResourceProof(NAME_111, ProofSource(2)).to_event(),
                LocalEvent::ResourceProofForElderReady(NAME_111).to_event(),
                TestEvent::SetResourceProof(NAME_110, ProofSource(2)).to_event(),
                LocalEvent::ResourceProofForElderReady(NAME_110).to_event(),
            ],
            &AssertJoiningState {
                action_our_events: vec![
                    Rpc::ResourceProofResponse {
                        candidate: OUR_NODE_CANDIDATE,
                        destination: NAME_111,
                        proof: Proof::ValidPart,
                    }
                    .to_event(),
                    Rpc::ResourceProofResponse {
                        candidate: OUR_NODE_CANDIDATE,
                        destination: NAME_110,
                        proof: Proof::ValidPart,
                    }
                    .to_event(),
                ],
                routine_complete_output: None,
            },
        );
    }

    #[test]
    fn joining_got_part_proof_receipt() {
        let mut initial_state = initial_joining_state_with_dst_200();
        initial_state.start(CANDIDATE_RELOCATED_INFO_132);

        let initial_state = arrange_initial_joining_state(
            &initial_state,
            &[
                Rpc::NodeConnected(OUR_NODE_CANDIDATE, GenesisPfxInfo(DST_SECTION_INFO_200))
                    .to_event(),
                Rpc::ResourceProof {
                    candidate: OUR_NODE_CANDIDATE,
                    source: NAME_111,
                    proof: ProofRequest { value: NAME_111.0 },
                }
                .to_event(),
                TestEvent::SetResourceProof(NAME_111, ProofSource(2)).to_event(),
                LocalEvent::ResourceProofForElderReady(NAME_111).to_event(),
            ],
        );

        run_joining_test(
            "On receiving receipt, send the next part (end) of the proof to that Elder.",
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
                routine_complete_output: None,
            },
        );
    }

    #[test]
    fn joining_got_end_proof_receipt() {
        let mut initial_state = initial_joining_state_with_dst_200();
        initial_state.start(CANDIDATE_RELOCATED_INFO_132);

        let initial_state = arrange_initial_joining_state(
            &initial_state,
            &[
                Rpc::NodeConnected(OUR_NODE_CANDIDATE, GenesisPfxInfo(DST_SECTION_INFO_200))
                    .to_event(),
                Rpc::ResourceProof {
                    candidate: OUR_NODE_CANDIDATE,
                    source: NAME_111,
                    proof: ProofRequest { value: NAME_111.0 },
                }
                .to_event(),
                TestEvent::SetResourceProof(NAME_111, ProofSource(2)).to_event(),
                LocalEvent::ResourceProofForElderReady(NAME_111).to_event(),
                Rpc::ResourceProofReceipt {
                    candidate: OUR_NODE_CANDIDATE,
                    source: NAME_111,
                }
                .to_event(),
            ],
        );

        run_joining_test(
            "On receiving receipt for end, do not send anymore.",
            &initial_state,
            &[Rpc::ResourceProofReceipt {
                candidate: OUR_NODE_CANDIDATE,
                source: NAME_111,
            }
            .to_event()],
            &AssertJoiningState {
                action_our_events: vec![],
                routine_complete_output: None,
            },
        );
    }

    #[test]
    fn joining_resend_timeout_one_proof_completed_one_in_progress() {
        let mut initial_state = initial_joining_state_with_dst_200();
        initial_state.start(CANDIDATE_RELOCATED_INFO_132);

        let initial_state = arrange_initial_joining_state(
            &initial_state,
            &[
                Rpc::NodeConnected(OUR_NODE_CANDIDATE, GenesisPfxInfo(DST_SECTION_INFO_200))
                    .to_event(),
                TestEvent::SetResourceProof(NAME_111, ProofSource(2)).to_event(),
                LocalEvent::ResourceProofForElderReady(NAME_111).to_event(),
                TestEvent::SetResourceProof(NAME_110, ProofSource(2)).to_event(),
                LocalEvent::ResourceProofForElderReady(NAME_110).to_event(),
                Rpc::ResourceProofReceipt {
                    candidate: OUR_NODE_CANDIDATE,
                    source: NAME_111,
                }
                .to_event(),
                Rpc::ResourceProofReceipt {
                    candidate: OUR_NODE_CANDIDATE,
                    source: NAME_111,
                }
                .to_event(),
            ],
        );

        run_joining_test(
            "When connected, resend the incomplete proofs not sent within timeout.",
            &initial_state,
            &[
                LocalEvent::JoiningTimeoutResendInfo.to_event(),
                LocalEvent::JoiningTimeoutResendInfo.to_event(),
            ],
            &AssertJoiningState {
                action_our_events: vec![
                    ActionTriggered::Scheduled(LocalEvent::JoiningTimeoutResendInfo).to_event(),
                    Rpc::ResourceProofResponse {
                        candidate: OUR_NODE_CANDIDATE,
                        destination: NAME_110,
                        proof: Proof::ValidPart,
                    }
                    .to_event(),
                    ActionTriggered::Scheduled(LocalEvent::JoiningTimeoutResendInfo).to_event(),
                ],
                routine_complete_output: None,
            },
        );
    }

    #[test]
    fn joining_approved() {
        let mut initial_state = initial_joining_state_with_dst_200();
        initial_state.start(CANDIDATE_RELOCATED_INFO_132);

        let initial_state = arrange_initial_joining_state(&initial_state, &[]);

        run_joining_test(
            "On NodeApproval: complete the routine work.",
            &initial_state,
            &[
                Rpc::NodeApproval(OUR_NODE_CANDIDATE, GenesisPfxInfo(DST_SECTION_INFO_200))
                    .to_event(),
            ],
            &AssertJoiningState {
                routine_complete_output: Some(GenesisPfxInfo(DST_SECTION_INFO_200)),
                ..AssertJoiningState::default()
            },
        );
    }
}
