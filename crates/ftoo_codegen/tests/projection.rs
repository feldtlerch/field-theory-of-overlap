use ftoo_manifest::{Evidence, load_projection};
use std::path::Path;

#[test]
fn committed_artifact_is_checked_by_the_ordinary_workspace_gate() {
    use ftoo_manifest::{HistoryChange as H, NodeId as N, TagId as T};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    ftoo_codegen::check_projection(&root).unwrap();
    let bytes = std::fs::read(root.join("generated/manifest.json")).unwrap();
    let loaded = load_projection(&bytes).unwrap();
    let book = loaded.addressing().unwrap();
    assert!(std::ptr::eq(book.graph(), loaded.manifest()));
    let tags = [
        (T::LangevinCoefficientOne, "c:1", N::LangevinCoefficientOne),
        (
            T::LangevinCoefficientThree,
            "c:3",
            N::LangevinCoefficientThree,
        ),
        (
            T::LangevinCoefficientOneValue,
            "v:1",
            N::LangevinCoefficientOneValue,
        ),
        (
            T::LangevinCoefficientThreeValue,
            "v:3",
            N::LangevinCoefficientThreeValue,
        ),
    ];
    assert_eq!(
        loaded
            .tags()
            .iter()
            .map(|d| (d.tag(), d.spelling(), d.target()))
            .collect::<Vec<_>>(),
        tags
    );
    assert_eq!(loaded.live(), tags.map(|(t, _, _)| t));
    assert_eq!(loaded.retired().len(), 0);
    assert_eq!(loaded.out_of_slice().len(), 0);
    assert_eq!(book.retired().count(), 0);
    assert_eq!(book.out_of_slice().count(), 0);
    let mut events = Vec::new();
    for (ordinal, identity, formulation) in [
        (
            1,
            N::LangevinCoefficientOne,
            "The coefficient of k in the three-dimensional Langevin expansion.",
        ),
        (
            2,
            N::LangevinCoefficientThree,
            "The coefficient of k cubed in the same expansion.",
        ),
        (
            3,
            N::LangevinCoefficientOneValue,
            "The first coefficient is exactly 1/3.",
        ),
        (
            4,
            N::LangevinCoefficientThreeValue,
            "The third coefficient is exactly -1/45.",
        ),
    ] {
        events.push((
            ordinal,
            "proposal:manifest:kernel-bite",
            "Retrospective registration of the M1 seed identity.".into(),
            H::AdmitIdentity {
                identity,
                original_formulation: formulation.into(),
            },
        ));
    }
    for (ordinal, (tag, handle, target)) in (5..=8).zip(tags) {
        assert_eq!(book.resolve(handle), Ok(target));
        events.push((
            ordinal,
            "proposal:manifest:addressing-lifecycle-bite",
            format!(
                "The handle addresses the existing seed referent {}.",
                target.semantic_name()
            ),
            H::AdmitTag { tag },
        ));
    }
    assert_eq!(
        loaded
            .history()
            .iter()
            .map(|e| (
                e.ordinal(),
                e.authority_record(),
                e.reason().to_owned(),
                e.change().clone()
            ))
            .collect::<Vec<_>>(),
        events
    );
    assert_eq!(
        book.history().cloned().collect::<Vec<_>>(),
        loaded.history()
    );
}

fn lossless<T: TryInto<u64>>(value: T) -> u64
where
    T::Error: std::fmt::Debug,
{
    value.try_into().expect("engine limit must fit u64")
}

#[test]
fn loaded_actual_artifact_audits_both_receipts_and_all_71_limits() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bytes = std::fs::read(root.join("generated/manifest.json")).unwrap();
    let loaded = load_projection(&bytes).unwrap();
    let report = ftoo_audit::audit(loaded.manifest()).unwrap();
    assert_eq!(report.outcomes().len(), 2);
    for row in report.outcomes() {
        let ftoo_audit::Verdict::Certified(receipt) = row.verdict() else {
            panic!("expected certified receipt");
        };
        let exported = loaded
            .budgets()
            .iter()
            .find(|b| b.spec().id == receipt.budget())
            .unwrap()
            .limits();
        let limits = receipt.limits();
        let mut count = 0;
        macro_rules! fields {
            ($($field:ident = $expected:literal),* $(,)?) => { $(
                assert_eq!(exported.$field(),$expected,stringify!($field));
                assert_eq!(lossless(limits.$field),exported.$field(),stringify!($field));
                count+=1;
            )* };
        }
        fields! {
            source_bytes=262_144,name_bytes=256,original_occurrences=10_000,depth=128,
            vertices=50_000,edges=200_000,staged_vertices=50_000,staged_edges=200_000,
            payload_bytes=16_777_216,staged_payload_bytes=16_777_216,work=3_000_000,
            arithmetic_steps=500_000,component_bits=4_096,intermediate_bits=8_192,
            literal_bytes=256,decimal_exponent=128,integer_exponent=256,polynomial_degree=64,
            polynomial_variables=16,polynomial_terms=4_096,matrix_entries=4_096,
            condition_atoms=256,condition_steps=100_000,match_attempts=50_000,match_batch=32,
            frontier_labels=8_192,queue_entries=16_384,transitions=20_000,expansion_credits=20_000,
            peak_expression_size=50_000,proof_nodes=20_000,witness_bytes=1_048_576,
            output_bytes=1_048_576,diagnostic_records=128,replay_work=1_500_000,
            cost_quota=4,progress_quota=1,case_arms=256,region_nodes=2_048,region_cells=256,
            refinement_pairs=4_096,local_contexts=128,hypothesis_depth=32,boundary_nodes=128,
            jet_order=8,jet_entries=4_096,delta_order=8,test_nodes=256,pairing_steps=4_096,
            pullback_zeros=64,coefficient_ops=2_000_000,denominator_factors=256,coefficient_maps=256,
            tower_levels=8,tower_degree=256,tower_reductions=100_000,gcd_steps=100_000,
            component_splits=128,orbit_permutations=40_320,orbit_members=256,orbit_terms=262_144,
            symmetric_steps=4_096,resolvent_depth=8,resolvent_degree=32,selection_slots=64,
            solution_branches=256,family_parameters=64,solve_identities=8_192,linear_pivots=256,
            isolation_steps=100_000,local_branches=64,
        }
        assert_eq!(count, 71);
        assert_eq!(
            report.status(row.claim()).unwrap().evidence(),
            ftoo_audit::AuditedEvidence::Certified
        );
        assert_eq!(
            loaded.manifest().status(row.claim()).unwrap().evidence(),
            Evidence::Deferred
        );
    }
}
