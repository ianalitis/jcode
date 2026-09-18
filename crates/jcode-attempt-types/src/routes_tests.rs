use super::*;
use chrono::TimeZone;

fn promoted_at() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 18, 20, 0, 0).unwrap()
}

fn entry() -> RouteEntry {
    RouteEntry {
        provider: "openrouter".into(),
        model_exact: "deepseek/deepseek-v4-flash-0731".into(),
        endpoint: "https://openrouter.ai/api/v1".into(),
        route_class: RouteClass::MeteredRemote,
        effort: Effort::Medium,
        admitted_data_class: DataClass::Public,
        promoted_at: promoted_at(),
        evidence_path: "/scratch/ci-scout/RECEIPT.md".into(),
        policy_version: "2026-09-18".into(),
    }
}

fn table() -> RouteTable {
    let mut t = RouteTable::default();
    t.promote("ci_scout", entry());
    t
}

const VALID_TOML: &str = r#"
version = 1

[routes.ci_scout]
provider = "openrouter"
model_exact = "deepseek/deepseek-v4-flash-0731"
endpoint = "https://openrouter.ai/api/v1"
route_class = "metered_remote"
effort = "medium"
admitted_data_class = "public"
promoted_at = "2026-09-18T20:00:00Z"
evidence_path = "/scratch/ci-scout/RECEIPT.md"
policy_version = "2026-09-18"
"#;

#[test]
fn parses_and_resolves_a_valid_entry() {
    let t = RouteTable::from_toml_str(VALID_TOML).unwrap();
    assert_eq!(t, table());
    let resolved = t.resolve("ci_scout", DataClass::Public).unwrap();
    assert_eq!(resolved.model_exact, "deepseek/deepseek-v4-flash-0731");
    assert_eq!(resolved.route_class, RouteClass::MeteredRemote);
}

#[test]
fn unknown_task_class_refuses_rather_than_defaulting() {
    let t = table();
    assert_eq!(
        t.resolve("not_promoted", DataClass::Public),
        Err(RouteTableError::UnknownTaskClass("not_promoted".into()))
    );
}

#[test]
fn private_request_outside_a_public_promotion_refuses() {
    let mut t = RouteTable::default();
    let mut e = entry();
    e.route_class = RouteClass::IncludedSubscription;
    e.admitted_data_class = DataClass::Public;
    t.promote("brief", e);
    assert_eq!(
        t.resolve("brief", DataClass::Private),
        Err(RouteTableError::OutsidePromotionScope {
            task_class: "brief".into(),
            requested: DataClass::Private,
            admitted: DataClass::Public,
        })
    );
    // The vetted class itself still resolves.
    assert!(t.resolve("brief", DataClass::Public).is_ok());
}

#[test]
fn secret_never_resolves_even_on_an_included_route() {
    let mut t = RouteTable::default();
    let mut e = entry();
    e.route_class = RouteClass::IncludedSubscription;
    e.admitted_data_class = DataClass::Private;
    t.promote("brief", e);
    assert!(matches!(
        t.resolve("brief", DataClass::Secret),
        Err(RouteTableError::DataClassNotEligible { .. })
    ));
}

#[test]
fn validate_rejects_an_entry_that_could_not_freeze() {
    // Private admitted on a metered route is a contradiction.
    let mut t = RouteTable::default();
    let mut e = entry();
    e.admitted_data_class = DataClass::Private;
    t.promote("bad", e);
    assert!(matches!(
        t.validate(),
        Err(RouteTableError::DataClassNotEligible { .. })
    ));

    // Banned router family.
    let mut t = RouteTable::default();
    let mut e = entry();
    e.model_exact = "openai/gpt-5.6".into();
    t.promote("bad", e);
    assert!(matches!(
        t.validate(),
        Err(RouteTableError::BannedRouterFamily { .. })
    ));

    // Floating alias.
    let mut t = RouteTable::default();
    let mut e = entry();
    e.model_exact = "deepseek-v4:latest".into();
    t.promote("bad", e);
    assert!(matches!(
        t.validate(),
        Err(RouteTableError::FloatingAlias { .. })
    ));

    // Empty evidence path.
    let mut t = RouteTable::default();
    let mut e = entry();
    e.evidence_path = "  ".into();
    t.promote("bad", e);
    assert!(matches!(
        t.validate(),
        Err(RouteTableError::EmptyField {
            field: "evidence_path",
            ..
        })
    ));
}

#[test]
fn empty_table_is_rejected() {
    let src = "version = 1\n";
    assert_eq!(
        RouteTable::from_toml_str(src),
        Err(RouteTableError::EmptyTable)
    );
}

#[test]
fn promote_and_demote_are_reversible() {
    let mut t = RouteTable::default();
    t.promote("brief", entry());
    assert!(t.resolve("brief", DataClass::Public).is_ok());
    let dropped = t.demote("brief").unwrap();
    assert_eq!(dropped.model_exact, "deepseek/deepseek-v4-flash-0731");
    assert!(matches!(
        t.resolve("brief", DataClass::Public),
        Err(RouteTableError::UnknownTaskClass(_))
    ));
    assert!(t.demote("brief").is_none());
}

#[test]
fn toml_round_trips() {
    let t = table();
    let src = t.to_toml_string().unwrap();
    let back = RouteTable::from_toml_str(&src).unwrap();
    assert_eq!(back, t);
}

#[test]
fn load_and_save_path_round_trip() {
    let path = std::env::temp_dir().join(format!(
        "jcode-routes-test-{}-{}.toml",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let t = table();
    t.save_to_path(&path).unwrap();
    let back = RouteTable::load_from_path(&path).unwrap();
    assert_eq!(back, t);
    let _ = std::fs::remove_file(&path);
}
