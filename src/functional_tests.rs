// Copyright 2019 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under The General Public License (GPL), version 3.
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied. Please review the Licences for the specific language governing
// permissions and limitations relating to use of the SAFE Network Software.

use crate::{
    actions::{Action, InnerAction},
    state::MemberState,
    utilities::{
        Candidate, CandidateInfo, Event, Node, NodeState, ParsecVote, Proof, RelocatedInfo, Rpc,
        State, TestEvent, TryResult,
    },
};
use rand::{self, seq::SliceRandom, Rng, SeedableRng};
use rand_xorshift::XorShiftRng;
use std::{env, iter, thread};
use unwrap::unwrap;

fn get_rng() -> XorShiftRng {
    let env_var_name = "ROUTING_MODEL_SEED";
    let seed = env::var(env_var_name)
        .ok()
        .map(|value| {
            unwrap!(
                value.parse::<u64>(),
                "Env var 'ROUTING_MODEL_SEED={}' is not a valid u64.",
                value
            )
        })
        .unwrap_or_else(rand::random);
    println!(
        "To replay this '{}', set env var {}={}",
        unwrap!(thread::current().name()),
        env_var_name,
        seed
    );
    XorShiftRng::seed_from_u64(seed)
}

struct RandomEvents(Vec<Event>);

impl RandomEvents {
    /// With a 50% probability of skipping the event, try to handle each one in `self`.
    fn handle<T: Rng>(&self, member_state: &mut MemberState, rng: &mut T) {
        for optional_event in &self.0 {
            if rng.gen() {
                assert_eq!(TryResult::Handled, member_state.try_next(*optional_event));
            }
        }
    }
}

#[test]
fn relocate_adult_src() {
    let mut rng = get_rng();
    let nodes = iter::repeat_with(|| rng.gen())
        .take(6)
        .collect::<Vec<Node>>();

    let action = Action::new(
        InnerAction::new_with_our_attributes(rng.gen())
            .with_next_target_interval(rng.gen())
            .extend_current_nodes_with(&NodeState::default_elder(), &nodes),
    );

    // Sort into elders and adults.
    let to_become_adults = unwrap!(action.check_elder());
    let relocating_node = unwrap!(to_become_adults.changes.choose(&mut rng)).0;
    action.mark_elder_change(to_become_adults);

    let mut member_state = MemberState {
        action,
        ..Default::default()
    };

    assert!(member_state
        .action
        .inner()
        .our_current_nodes
        .contains_key(&relocating_node.0.name));

    let relocated_info = RelocatedInfo {
        candidate: Candidate(relocating_node.0),
        expected_age: relocating_node.0.age.increment_by_one(),
        target_interval_centre: rng.gen(),
        section_info: rng.gen(),
    };

    let required_events = [
        TestEvent::SetWorkUnitEnoughToRelocate(relocating_node).to_event(),
        ParsecVote::WorkUnitIncrement.to_event(),
        ParsecVote::CheckRelocate.to_event(),
        ParsecVote::RelocateResponse(relocated_info).to_event(),
    ];

    let optional_random_events = RandomEvents(vec![
        ParsecVote::WorkUnitIncrement.to_event(),
        ParsecVote::CheckRelocate.to_event(),
        Rpc::RelocateResponse(relocated_info).to_event(),
    ]);

    for required_event in &required_events {
        assert_eq!(TryResult::Handled, member_state.try_next(*required_event));
        optional_random_events.handle(&mut member_state, &mut rng);
    }

    assert_eq!(
        State::Relocated(relocated_info),
        unwrap!(member_state.action.node_state(relocating_node.0.name)).state
    );

    assert_eq!(
        TryResult::Handled,
        member_state.try_next(ParsecVote::RelocatedInfo(relocated_info).to_event())
    );
    assert!(member_state
        .action
        .node_state(relocating_node.0.name)
        .is_none());

    optional_random_events.handle(&mut member_state, &mut rng);
}

#[test]
fn relocate_adult_dst() {
    let mut rng = get_rng();

    let dst_nodes = iter::repeat_with(|| rng.gen())
        .take(6)
        .collect::<Vec<Node>>();

    let action = Action::new(
        InnerAction::new_with_our_attributes(rng.gen())
            .with_next_target_interval(rng.gen())
            .extend_current_nodes_with(&NodeState::default_elder(), &dst_nodes),
    );
    let dst_name = action.our_name();

    // Sort into elders and adults.
    let to_become_adults = unwrap!(action.check_elder());
    action.mark_elder_change(to_become_adults);

    let mut member_state = MemberState {
        action,
        ..Default::default()
    };

    let old_public_id = Candidate(rng.gen());
    let new_public_id = {
        let mut new_public_id = Candidate(rng.gen());
        new_public_id.0.age.0 = old_public_id.0.age.0 + 1;
        new_public_id
    };

    let candidate_info = CandidateInfo {
        old_public_id,
        new_public_id,
        destination: dst_name,
        waiting_candidate_name: member_state.action.inner().next_target_interval,
        valid: true,
    };

    let required_events = [
        ParsecVote::ExpectCandidate(old_public_id).to_event(),
        ParsecVote::CheckResourceProof.to_event(),
        ParsecVote::Online(old_public_id, new_public_id).to_event(),
        ParsecVote::CheckElder.to_event(),
    ];

    let optional_any_time = RandomEvents(vec![
        ParsecVote::WorkUnitIncrement.to_event(),
        ParsecVote::CheckRelocate.to_event(),
        Rpc::ExpectCandidate(old_public_id).to_event(),
    ]);

    let optional_after_expect_candidate = RandomEvents(vec![
        Rpc::CandidateInfo(candidate_info).to_event(),
        Rpc::ConnectionInfoResponse {
            source: rng.gen(),
            destination: dst_name,
            connection_info: rng.gen(),
        }
        .to_event(),
    ]);

    let optional_after_check_resource_proof = RandomEvents(vec![Rpc::ResourceProofResponse {
        candidate: new_public_id,
        destination: dst_name,
        proof: Proof::ValidPart,
    }
    .to_event()]);

    for (i, required_event) in required_events.iter().enumerate() {
        assert_eq!(TryResult::Handled, member_state.try_next(*required_event));
        optional_any_time.handle(&mut member_state, &mut rng);
        if i > 0 {
            optional_after_expect_candidate.handle(&mut member_state, &mut rng);
        }
        if i > 2 {
            optional_after_check_resource_proof.handle(&mut member_state, &mut rng);
        }
    }

    assert!(member_state
        .action
        .inner()
        .our_current_nodes
        .contains_key(&new_public_id.name()));

    assert_eq!(
        State::Online,
        unwrap!(member_state.action.node_state(new_public_id.name())).state
    );

    optional_any_time.handle(&mut member_state, &mut rng);
    optional_after_expect_candidate.handle(&mut member_state, &mut rng);
    optional_after_check_resource_proof.handle(&mut member_state, &mut rng);
}
