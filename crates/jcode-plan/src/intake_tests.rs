//! Fixture-scored agreement test for the intake classifier.

use crate::intake::classify;
use crate::intake::extract_features;
use jcode_attempt_types::DataClass;
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    #[allow(dead_code)]
    id: String,
    text: String,
    expected: Expected,
}

#[derive(Deserialize)]
struct Expected {
    #[allow(dead_code)]
    task_class: String,
    judgment: String,
    containment: String,
}

#[derive(Deserialize)]
struct FixtureFile {
    packets: Vec<Fixture>,
}

const FIXTURE: &str = include_str!("../fixtures/intake_packets.json");

#[test]
fn intake_fixture_agreement() {
    let file: FixtureFile = serde_json::from_str(FIXTURE).expect("fixture json parses");
    assert_eq!(file.packets.len(), 25, "fixture must contain 25 packets");

    let mut task_class_hits = 0;
    let mut judgment_hits = 0;
    let mut containment_hits = 0;
    let mut disagreements: Vec<String> = Vec::new();

    for packet in &file.packets {
        let features = extract_features(&packet.text);
        let decision = classify(&features, DataClass::Synthetic);
        if decision.task_class.as_str() == packet.expected.task_class {
            task_class_hits += 1;
        } else {
            disagreements.push(format!(
                "{}: task_class got {} want {}",
                packet.id,
                decision.task_class.as_str(),
                packet.expected.task_class
            ));
        }
        if decision.judgment.as_str() == packet.expected.judgment {
            judgment_hits += 1;
        } else {
            disagreements.push(format!(
                "{}: judgment got {} want {}",
                packet.id,
                decision.judgment.as_str(),
                packet.expected.judgment
            ));
        }
        if decision.containment.as_str() == packet.expected.containment {
            containment_hits += 1;
        } else {
            disagreements.push(format!(
                "{}: containment got {} want {}",
                packet.id,
                decision.containment.as_str(),
                packet.expected.containment
            ));
        }
    }

    println!("task_class agreement: {task_class_hits}/25");
    println!("judgment agreement: {judgment_hits}/25");
    println!("containment agreement: {containment_hits}/25");
    if !disagreements.is_empty() {
        println!("disagreements:");
        for d in &disagreements {
            println!("  {d}");
        }
    }

    assert!(task_class_hits >= 22, "task_class {task_class_hits}/25");
    assert!(judgment_hits >= 20, "judgment {judgment_hits}/25");
    assert!(containment_hits >= 20, "containment {containment_hits}/25");
}
