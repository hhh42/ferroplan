//! Oracle probe: replay a classic-format plan file against the ORIGINAL
//! domain/problem and print the independent verifier's verdicts.
//! Usage: verify_plan <domain> <problem> <plan-text-file>
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dom = std::fs::read_to_string(&a[1]).unwrap();
    let prob = std::fs::read_to_string(&a[2]).unwrap();
    let plan_text = std::fs::read_to_string(&a[3]).unwrap();
    let steps = ferroplan::plan::parse_classical(&plan_text);
    let v = ferroplan::verify::verify(&dom, &prob, &steps).expect("verify");
    println!(
        "steps {}; hard_goal_met {}; constraints_met {}; metric {}; sat {}, vio {}",
        steps.len(), v.hard_goal_met, v.constraints_met, v.metric, v.satisfied, v.violated
    );
}
