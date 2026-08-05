use ferroplan::{
    solve_planning_type, Agent, PlanningMethod, PlanningProblem, PlanningTask, PlanningType,
    QueueState, RdfTriple, Tool, UniversalGoal, UniversalPlanningRequest, UniversalState,
    UniversalTransition, WorkflowEdge,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn set(values: &[&str]) -> BTreeSet<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

fn state(id: &str, facts: &[&str]) -> UniversalState {
    UniversalState {
        id: id.to_owned(),
        facts: set(facts),
        fluents: BTreeMap::new(),
    }
}

fn edge(action: &str, from: &str, to: &str) -> UniversalTransition {
    UniversalTransition {
        action: action.to_owned(),
        from: from.to_owned(),
        to: to.to_owned(),
        cost: 1,
        duration: 1,
        reward: 0,
        probability_ppm: 1_000_000,
        observation: None,
        requires: BTreeSet::new(),
    }
}

fn chain_problem() -> PlanningProblem {
    PlanningProblem {
        states: vec![state("s0", &[]), state("s1", &[]), state("g", &["done"])],
        initial_states: vec!["s0".to_owned()],
        goal: UniversalGoal {
            facts: set(&["done"]),
            ..UniversalGoal::default()
        },
        transitions: vec![edge("prepare", "s0", "s1"), edge("finish", "s1", "g")],
        ..PlanningProblem::default()
    }
}

fn probabilistic_problem() -> PlanningProblem {
    let mut problem = chain_problem();
    problem.transitions = vec![
        UniversalTransition {
            action: "try".to_owned(),
            from: "s0".to_owned(),
            to: "g".to_owned(),
            probability_ppm: 700_000,
            ..edge("try", "s0", "g")
        },
        UniversalTransition {
            action: "try".to_owned(),
            from: "s0".to_owned(),
            to: "s0".to_owned(),
            probability_ppm: 300_000,
            ..edge("try", "s0", "s0")
        },
    ];
    problem
}

fn fond_problem() -> PlanningProblem {
    PlanningProblem {
        states: vec![
            state("s0", &[]),
            state("g1", &["done"]),
            state("g2", &["done"]),
        ],
        initial_states: vec!["s0".to_owned()],
        goal: UniversalGoal {
            facts: set(&["done"]),
            ..UniversalGoal::default()
        },
        transitions: vec![
            UniversalTransition {
                action: "commit".to_owned(),
                from: "s0".to_owned(),
                to: "g1".to_owned(),
                probability_ppm: 500_000,
                ..edge("commit", "s0", "g1")
            },
            UniversalTransition {
                action: "commit".to_owned(),
                from: "s0".to_owned(),
                to: "g2".to_owned(),
                probability_ppm: 500_000,
                ..edge("commit", "s0", "g2")
            },
        ],
        ..PlanningProblem::default()
    }
}

fn belief_problem(contingent: bool) -> PlanningProblem {
    let mut transitions = vec![edge("resolve", "a", "ga"), edge("resolve", "b", "gb")];
    if contingent {
        transitions[0].observation = Some("saw-a".to_owned());
        transitions[1].observation = Some("saw-b".to_owned());
    }
    PlanningProblem {
        states: vec![
            state("a", &[]),
            state("b", &[]),
            state("ga", &["done"]),
            state("gb", &["done"]),
        ],
        initial_states: vec!["a".to_owned(), "b".to_owned()],
        goal: UniversalGoal {
            facts: set(&["done"]),
            ..UniversalGoal::default()
        },
        transitions,
        ..PlanningProblem::default()
    }
}

fn hierarchy_problem() -> PlanningProblem {
    PlanningProblem {
        tasks: vec![
            PlanningTask {
                id: "root".to_owned(),
                primitive_action: None,
                requires: BTreeSet::new(),
            },
            PlanningTask {
                id: "inspect".to_owned(),
                primitive_action: Some("inspect-repo".to_owned()),
                requires: set(&["read"]),
            },
            PlanningTask {
                id: "verify".to_owned(),
                primitive_action: Some("run-tests".to_owned()),
                requires: set(&["test"]),
            },
        ],
        root_tasks: vec!["root".to_owned()],
        methods: vec![PlanningMethod {
            id: "root-method".to_owned(),
            task: "root".to_owned(),
            subtasks: vec!["inspect".to_owned(), "verify".to_owned()],
        }],
        ..PlanningProblem::default()
    }
}

fn workflow_problem() -> PlanningProblem {
    let mut problem = hierarchy_problem();
    problem.root_tasks.clear();
    problem.methods.clear();
    problem.workflow_edges = vec![WorkflowEdge {
        before: "inspect".to_owned(),
        after: "verify".to_owned(),
    }];
    problem
}

fn multi_agent_problem() -> PlanningProblem {
    let mut problem = chain_problem();
    problem.transitions[0].requires = set(&["read"]);
    problem.transitions[1].requires = set(&["test"]);
    problem.agents = vec![
        Agent {
            id: "reader".to_owned(),
            capabilities: set(&["read"]),
            capacity: 1,
            current_wip: 0,
        },
        Agent {
            id: "tester".to_owned(),
            capabilities: set(&["test"]),
            capacity: 1,
            current_wip: 0,
        },
    ];
    problem
}

fn delegated_problem() -> PlanningProblem {
    let mut problem = hierarchy_problem();
    problem.agents = vec![
        Agent {
            id: "reader".to_owned(),
            capabilities: set(&["read"]),
            capacity: 1,
            current_wip: 0,
        },
        Agent {
            id: "tester".to_owned(),
            capabilities: set(&["test"]),
            capacity: 1,
            current_wip: 0,
        },
    ];
    problem
}

fn mcp_problem() -> PlanningProblem {
    let mut problem = hierarchy_problem();
    problem.tools = vec![
        Tool {
            id: "repo-read".to_owned(),
            capabilities: set(&["read"]),
            authority_bound: true,
            verifier_bound: true,
            receipt_bound: true,
        },
        Tool {
            id: "test-runner".to_owned(),
            capabilities: set(&["test"]),
            authority_bound: true,
            verifier_bound: true,
            receipt_bound: true,
        },
    ];
    problem
}

fn rdf_problem() -> PlanningProblem {
    PlanningProblem {
        rdf: vec![
            RdfTriple { subject: "s0".into(), predicate: "state".into(), object: "true".into() },
            RdfTriple { subject: "s0".into(), predicate: "initial".into(), object: "true".into() },
            RdfTriple { subject: "g".into(), predicate: "state".into(), object: "true".into() },
            RdfTriple { subject: "g".into(), predicate: "goal".into(), object: "true".into() },
            RdfTriple { subject: "move".into(), predicate: "from".into(), object: "s0".into() },
            RdfTriple { subject: "move".into(), predicate: "to".into(), object: "g".into() },
            RdfTriple { subject: "move".into(), predicate: "action".into(), object: "move".into() },
        ],
        ..PlanningProblem::default()
    }
}

fn fixtures() -> Vec<(PlanningType, PlanningProblem)> {
    let mut numeric = chain_problem();
    numeric.states[2].fluents.insert("quality".to_owned(), 10);
    numeric.goal.numeric_min.insert("quality".to_owned(), 10);

    let mut preferences = chain_problem();
    preferences.soft_goal_facts.insert("bonus".to_owned(), 3);

    let mut flow = chain_problem();
    flow.queues = vec![QueueState {
        id: "verify".to_owned(),
        current_wip: 0,
        max_wip: 1,
    }];

    vec![
        (PlanningType::Classical, chain_problem()),
        (PlanningType::CostOptimal, chain_problem()),
        (PlanningType::Numeric, numeric),
        (PlanningType::Temporal, chain_problem()),
        (PlanningType::Preferences, preferences),
        (PlanningType::Probabilistic, probabilistic_problem()),
        (PlanningType::Fond, fond_problem()),
        (PlanningType::Conformant, belief_problem(false)),
        (PlanningType::Contingent, belief_problem(true)),
        (PlanningType::Hierarchical, hierarchy_problem()),
        (PlanningType::PartialOrder, workflow_problem()),
        (PlanningType::Workflow, workflow_problem()),
        (PlanningType::FlowConstrained, flow),
        (PlanningType::ResolutionAdaptive, hierarchy_problem()),
        (PlanningType::MultiAgent, multi_agent_problem()),
        (PlanningType::RdfDerived, rdf_problem()),
        (PlanningType::A2aDelegated, delegated_problem()),
        (PlanningType::McpBound, mcp_problem()),
    ]
}

fn oracle_pythonpath() -> Option<String> {
    std::env::var("MFW_PLANNER_ORACLE_PYTHONPATH").ok()
}

#[test]
fn every_planner_is_admitted_by_the_mfw_python_oracle() {
    let pythonpath = match oracle_pythonpath() {
        Some(path) => path,
        None if std::env::var_os("FERROPLAN_REQUIRE_MFW_ORACLE").is_none() => return,
        None => panic!("MFW_PLANNER_ORACLE_PYTHONPATH is required"),
    };

    let root = std::env::temp_dir().join(format!(
        "ferroplan-mfw-oracle-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create oracle temp directory");

    for (index, (planning_type, problem)) in fixtures().into_iter().enumerate() {
        let request = UniversalPlanningRequest {
            planning_type,
            problem,
            limits: Default::default(),
        };
        let candidate = solve_planning_type(&request).expect("Ferroplan candidate");
        let request_path: PathBuf = root.join(format!("{index}-request.json"));
        let candidate_path: PathBuf = root.join(format!("{index}-candidate.json"));
        fs::write(&request_path, serde_json::to_vec(&request).unwrap()).unwrap();
        fs::write(&candidate_path, serde_json::to_vec(&candidate).unwrap()).unwrap();

        let output = Command::new("python3")
            .args([
                "-m",
                "mfw_planner_oracle",
                "compare",
                request_path.to_str().unwrap(),
                candidate_path.to_str().unwrap(),
            ])
            .env("PYTHONPATH", &pythonpath)
            .output()
            .expect("execute MFW Python oracle");

        let receipt: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "oracle emitted invalid JSON for {planning_type:?}: {error}; stderr={} stdout={}",
                String::from_utf8_lossy(&output.stderr),
                String::from_utf8_lossy(&output.stdout)
            )
        });
        assert!(
            output.status.success(),
            "oracle refused {planning_type:?}: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert_eq!(receipt["schema"], "urn:mfw:planner-oracle-receipt:v1");
        assert_eq!(receipt["oracle"], "mfw-python-v1");
        assert_eq!(receipt["agreement"], true, "{receipt}");
        assert_eq!(receipt["candidate_valid"], true, "{receipt}");
        assert!(
            receipt["receipt_digest"]
                .as_str()
                .is_some_and(|digest| digest.starts_with("sha256:")),
            "{receipt}"
        );
    }

    let _ = fs::remove_dir_all(root);
}
