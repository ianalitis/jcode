use super::PlanSpendReservations;

#[test]
fn plan_spend_cap_refuses_the_next_child_and_ambiguous_keeps_exposure() {
    let mut reservations = PlanSpendReservations::new(100);
    let first = reservations
        .reserve_child("coordinator", 60)
        .expect("first child fits");
    reservations.mark_ambiguous(&first);

    let error = reservations
        .reserve_child("coordinator", 60)
        .expect_err("ambiguous exposure must stop the next child at the cap");
    assert!(error.to_string().contains("cap 100"));
    assert_eq!(reservations.ledger.exposure_micro_usd(), 60);
}
